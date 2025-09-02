use crate::error::{AppError, AppResult};
use crate::models::*;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use uuid::Uuid;
use tracing::{info, warn, debug, error};
use tokio::sync::Semaphore;

/// 会话状态
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Idle,        // 空闲
    Active,      // 活跃中（正在处理请求）
    Reserved,    // 已预留（准备处理请求）
    Expired,     // 已过期
}

/// DeepSeek会话信息
#[derive(Debug, Clone)]
pub struct DeepSeekSession {
    pub session_id: String,
    pub conversation_id: Option<String>,  // OpenAI兼容的conversation_id
    pub account_email: String,
    pub user_token: String,
    pub state: SessionState,
    pub last_used: u64,
    pub created_at: u64,
    pub messages_count: usize,
    pub api_key: String,  // 关联的API密钥
}

/// 账号会话池
#[derive(Debug)]
pub struct AccountSessionPool {
    pub account_email: String,
    pub user_token: String,
    pub sessions: HashMap<String, DeepSeekSession>,  // conversation_id -> session
    pub active_session: Option<String>,  // 当前活跃的会话ID
    pub last_activity: u64,
    pub semaphore: Arc<Semaphore>,  // 并发控制，每个账号同时只能有1个活跃会话
}

/// 会话池管理器
pub struct SessionPoolManager {
    /// 按API密钥分组的账号池: api_key -> [account_email -> SessionPool]
    pools: Arc<RwLock<HashMap<String, HashMap<String, AccountSessionPool>>>>,
    /// 会话映射: conversation_id -> (api_key, account_email)
    session_mapping: Arc<RwLock<HashMap<String, (String, String)>>>,
    /// 全局会话超时时间（秒）
    session_timeout: u64,
}

impl AccountSessionPool {
    pub fn new(account_email: String, user_token: String) -> Self {
        Self {
            account_email,
            user_token,
            sessions: HashMap::new(),
            active_session: None,
            last_activity: SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
            semaphore: Arc::new(Semaphore::new(1)), // 每个账号同时只能处理1个请求
        }
    }

    /// 创建新会话
    pub fn create_session(&mut self, conversation_id: Option<String>, api_key: String) -> String {
        let session_id = Uuid::new_v4().to_string();
        let conv_id = conversation_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        
        let session = DeepSeekSession {
            session_id: session_id.clone(),
            conversation_id: Some(conv_id.clone()),
            account_email: self.account_email.clone(),
            user_token: self.user_token.clone(),
            state: SessionState::Reserved,
            last_used: SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
            messages_count: 0,
            api_key,
        };

        self.sessions.insert(conv_id.clone(), session);
        self.last_activity = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_secs();
        
        conv_id
    }

    /// 获取或创建会话
    pub fn get_or_create_session(&mut self, conversation_id: Option<String>, api_key: String) -> AppResult<String> {
        match conversation_id {
            Some(conv_id) => {
                // 检查现有会话
                if let Some(session) = self.sessions.get_mut(&conv_id) {
                    if session.state != SessionState::Expired {
                        session.last_used = SystemTime::now().duration_since(UNIX_EPOCH)
                            .unwrap_or_default().as_secs();
                        return Ok(conv_id);
                    }
                }
                // 会话不存在或已过期，创建新的
                Ok(self.create_session(Some(conv_id), api_key))
            }
            None => {
                // 创建新会话
                Ok(self.create_session(None, api_key))
            }
        }
    }

    /// 设置会话为活跃状态
    pub fn activate_session(&mut self, conversation_id: &str) -> AppResult<()> {
        if let Some(session) = self.sessions.get_mut(conversation_id) {
            // 如果已有活跃会话且不是当前会话，需要等待
            if let Some(active_id) = &self.active_session {
                if active_id != conversation_id {
                    return Err(AppError::ServiceUnavailable(
                        "Account is busy with another session".to_string()
                    ));
                }
            }

            session.state = SessionState::Active;
            self.active_session = Some(conversation_id.to_string());
            self.last_activity = SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default().as_secs();
            
            debug!("Activated session {} for account {}", conversation_id, self.account_email);
            Ok(())
        } else {
            Err(AppError::NotFound("Session not found".to_string()))
        }
    }

    /// 释放会话
    pub fn release_session(&mut self, conversation_id: &str) {
        if let Some(session) = self.sessions.get_mut(conversation_id) {
            session.state = SessionState::Idle;
            session.messages_count += 1;
        }
        
        if self.active_session.as_ref() == Some(&conversation_id.to_string()) {
            self.active_session = None;
        }
        
        debug!("Released session {} for account {}", conversation_id, self.account_email);
    }

    /// 清理过期会话
    pub fn cleanup_expired_sessions(&mut self, timeout: u64) -> usize {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_secs();
        
        let initial_count = self.sessions.len();
        
        self.sessions.retain(|conv_id, session| {
            let is_expired = (now - session.last_used) > timeout;
            if is_expired && self.active_session.as_ref() == Some(conv_id) {
                self.active_session = None;
            }
            !is_expired
        });
        
        initial_count - self.sessions.len()
    }

    /// 检查账号是否可用
    pub fn is_available(&self) -> bool {
        self.active_session.is_none()
    }

    /// 获取负载分数（越低越好）
    pub fn get_load_score(&self) -> f64 {
        let base_score = if self.is_available() { 0.0 } else { 1000.0 };
        let session_count_penalty = self.sessions.len() as f64 * 0.1;
        let age_penalty = {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default().as_secs();
            (now - self.last_activity) as f64 * 0.01
        };
        
        base_score + session_count_penalty + age_penalty
    }
}

impl SessionPoolManager {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            session_mapping: Arc::new(RwLock::new(HashMap::new())),
            session_timeout: 3600, // 1小时超时
        }
    }

    /// 添加账号到指定API密钥
    pub fn add_account(&self, api_key: String, account_email: String, user_token: String) {
        let mut pools = self.pools.write();
        let api_pools = pools.entry(api_key).or_insert_with(HashMap::new);
        
        if !api_pools.contains_key(&account_email) {
            api_pools.insert(
                account_email.clone(),
                AccountSessionPool::new(account_email.clone(), user_token)
            );
            info!("Added account {} to API key pool", account_email);
        }
    }

    /// 获取最佳账号进行会话处理
    pub async fn acquire_session(
        &self,
        api_key: &str,
        conversation_id: Option<String>,
    ) -> AppResult<(String, DeepSeekSession)> {
        // 1. 如果有conversation_id，先尝试找到对应的会话
        if let Some(conv_id) = &conversation_id {
            let existing_mapping = {
                let mapping = self.session_mapping.read();
                mapping.get(conv_id).cloned()
            };
            
            if let Some((mapped_api_key, account_email)) = existing_mapping {
                if mapped_api_key == api_key {
                    return self.reuse_existing_session(api_key, &account_email, conv_id).await;
                }
            }
        }

        // 2. 寻找最佳可用账号
        let best_account = self.find_best_available_account(api_key)?;
        
        // 3. 获取账号的信号量
        let semaphore = {
            let pools = self.pools.read();
            pools.get(api_key)
                .and_then(|api_pools| api_pools.get(&best_account))
                .map(|pool| pool.semaphore.clone())
                .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?
        };

        // 4. 等待获取信号量（确保同时只有一个请求）
        let _permit = semaphore.acquire().await
            .map_err(|e| AppError::Internal(format!("Failed to acquire semaphore: {}", e)))?;

        // 5. 创建或获取会话
        let conv_id = {
            let mut pools = self.pools.write();
            let api_pools = pools.get_mut(api_key)
                .ok_or_else(|| AppError::NotFound("API key not found".to_string()))?;
            let account_pool = api_pools.get_mut(&best_account)
                .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;
            
            let conv_id = account_pool.get_or_create_session(conversation_id, api_key.to_string())?;
            account_pool.activate_session(&conv_id)?;
            conv_id
        };

        // 6. 更新会话映射
        {
            let mut mapping = self.session_mapping.write();
            mapping.insert(conv_id.clone(), (api_key.to_string(), best_account.clone()));
        }

        // 7. 返回会话信息
        let session = {
            let pools = self.pools.read();
            pools.get(api_key)
                .and_then(|api_pools| api_pools.get(&best_account))
                .and_then(|pool| pool.sessions.get(&conv_id))
                .cloned()
                .ok_or_else(|| AppError::Internal("Session disappeared".to_string()))?
        };

        info!("Acquired session {} for account {} (API: {})", conv_id, best_account, api_key);
        Ok((conv_id, session))
    }

    /// 复用现有会话
    async fn reuse_existing_session(
        &self,
        api_key: &str,
        account_email: &str,
        conversation_id: &str,
    ) -> AppResult<(String, DeepSeekSession)> {
        // 获取信号量
        let semaphore = {
            let pools = self.pools.read();
            pools.get(api_key)
                .and_then(|api_pools| api_pools.get(account_email))
                .map(|pool| pool.semaphore.clone())
                .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?
        };

        let _permit = semaphore.acquire().await
            .map_err(|e| AppError::Internal(format!("Failed to acquire semaphore: {}", e)))?;

        // 激活会话
        {
            let mut pools = self.pools.write();
            let api_pools = pools.get_mut(api_key)
                .ok_or_else(|| AppError::NotFound("API key not found".to_string()))?;
            let account_pool = api_pools.get_mut(account_email)
                .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;
            
            account_pool.activate_session(conversation_id)?;
        }

        let session = {
            let pools = self.pools.read();
            pools.get(api_key)
                .and_then(|api_pools| api_pools.get(account_email))
                .and_then(|pool| pool.sessions.get(conversation_id))
                .cloned()
                .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?
        };

        info!("Reusing session {} for account {} (API: {})", conversation_id, account_email, api_key);
        Ok((conversation_id.to_string(), session))
    }

    /// 释放会话
    pub fn release_session(&self, conversation_id: &str) {
        let mapping = self.session_mapping.read();
        if let Some((api_key, account_email)) = mapping.get(conversation_id) {
            let mut pools = self.pools.write();
            if let Some(api_pools) = pools.get_mut(api_key) {
                if let Some(account_pool) = api_pools.get_mut(account_email) {
                    account_pool.release_session(conversation_id);
                    info!("Released session {} for account {}", conversation_id, account_email);
                }
            }
        }
    }

    /// 找到最佳可用账号
    fn find_best_available_account(&self, api_key: &str) -> AppResult<String> {
        let pools = self.pools.read();
        let api_pools = pools.get(api_key)
            .ok_or_else(|| AppError::NotFound("API key not found".to_string()))?;

        if api_pools.is_empty() {
            return Err(AppError::NotFound("No accounts available for this API key".to_string()));
        }

        // 寻找负载最低的可用账号
        let best_account = api_pools.iter()
            .min_by(|(_, pool_a), (_, pool_b)| {
                pool_a.get_load_score()
                    .partial_cmp(&pool_b.get_load_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(email, _)| email.clone())
            .ok_or_else(|| AppError::ServiceUnavailable("No suitable account found".to_string()))?;

        debug!("Selected account {} for API key {}", best_account, api_key);
        Ok(best_account)
    }

    /// 定期清理过期会话
    pub async fn cleanup_expired_sessions(&self) -> AppResult<usize> {
        let mut total_cleaned = 0;
        let mut pools = self.pools.write();
        
        for (api_key, api_pools) in pools.iter_mut() {
            for (account_email, pool) in api_pools.iter_mut() {
                let cleaned = pool.cleanup_expired_sessions(self.session_timeout);
                if cleaned > 0 {
                    info!("Cleaned {} expired sessions for account {} (API: {})", 
                          cleaned, account_email, api_key);
                }
                total_cleaned += cleaned;
            }
        }

        // 清理会话映射
        let mut mapping = self.session_mapping.write();
        let initial_mapping_count = mapping.len();
        mapping.retain(|conv_id, (api_key, account_email)| {
            pools.get(api_key)
                .and_then(|api_pools| api_pools.get(account_email))
                .map(|pool| pool.sessions.contains_key(conv_id))
                .unwrap_or(false)
        });
        
        let mapping_cleaned = initial_mapping_count - mapping.len();
        if mapping_cleaned > 0 {
            info!("Cleaned {} orphaned session mappings", mapping_cleaned);
        }

        Ok(total_cleaned)
    }

    /// 获取API密钥的统计信息
    pub fn get_api_key_stats(&self, api_key: &str) -> Option<SessionPoolStats> {
        let pools = self.pools.read();
        let api_pools = pools.get(api_key)?;

        let mut stats = SessionPoolStats {
            api_key: api_key.to_string(),
            total_accounts: api_pools.len(),
            available_accounts: 0,
            active_sessions: 0,
            total_sessions: 0,
        };

        for (_, pool) in api_pools.iter() {
            if pool.is_available() {
                stats.available_accounts += 1;
            }
            if pool.active_session.is_some() {
                stats.active_sessions += 1;
            }
            stats.total_sessions += pool.sessions.len();
        }

        Some(stats)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionPoolStats {
    pub api_key: String,
    pub total_accounts: usize,
    pub available_accounts: usize,
    pub active_sessions: usize,
    pub total_sessions: usize,
}

impl Default for SessionPoolManager {
    fn default() -> Self {
        Self::new()
    }
}
