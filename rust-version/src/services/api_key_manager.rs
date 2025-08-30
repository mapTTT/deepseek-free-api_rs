use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::services::login_service::LoginService;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use tracing::{info, warn, error, debug};
use serde_json;
use std::fs;
use std::path::Path;

pub struct ApiKeyManager {
    api_keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    user_tokens: Arc<RwLock<HashMap<String, Vec<String>>>>, // api_key -> user_tokens
    login_service: Arc<LoginService>,
    storage_path: String,
}

impl ApiKeyManager {
    pub fn new() -> Self {
        let login_service = Arc::new(LoginService::new());
        let storage_path = std::env::var("API_KEYS_STORAGE_PATH")
            .unwrap_or_else(|_| "./data/api_keys.json".to_string());

        let manager = Self {
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            user_tokens: Arc::new(RwLock::new(HashMap::new())),
            login_service,
            storage_path,
        };

        // 尝试加载已存在的API密钥
        if let Err(e) = manager.load_from_storage() {
            warn!("加载API密钥存储失败: {}", e);
        }

        manager
    }

    /// 创建新的API密钥
    pub fn create_api_key(&self, name: String, expires_days: Option<u32>) -> AppResult<CreateApiKeyResponse> {
        let api_key = format!("dsk-{}", Uuid::new_v4().simple().to_string());
        let created_at = SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::Internal(format!("获取时间戳失败: {}", e)))?
            .as_secs();

        let expires_at = expires_days.map(|days| {
            created_at + (days as u64 * 24 * 60 * 60)
        });

        let key_info = ApiKey {
            id: Uuid::new_v4().to_string(),
            key: api_key.clone(),
            name: name.clone(),
            user_tokens: Vec::new(),
            created_at,
            expires_at,
            usage_count: 0,
            is_active: true,
        };

        // 存储API密钥
        {
            let mut keys = self.api_keys.write();
            keys.insert(api_key.clone(), key_info);
        }

        {
            let mut tokens = self.user_tokens.write();
            tokens.insert(api_key.clone(), Vec::new());
        }

        // 保存到存储
        if let Err(e) = self.save_to_storage() {
            warn!("保存API密钥到存储失败: {}", e);
        }

        info!("创建了新的API密钥: {} ({})", name, api_key);

        Ok(CreateApiKeyResponse {
            api_key,
            name,
            created_at,
            expires_at,
        })
    }

    /// 添加账户到API密钥
    pub async fn add_account(&self, api_key: String, email: String, password: String) -> AppResult<AddAccountResponse> {
        // 验证API密钥是否存在且有效
        if !self.is_api_key_valid(&api_key)? {
            return Err(AppError::Unauthorized("无效的API密钥".to_string()));
        }

        // 尝试登录获取userToken
        info!("为API密钥 {} 添加账户: {}", api_key, email);
        let user_token = self.login_service.login(&email, &password).await?;

        // 验证token是否有效
        if !self.login_service.verify_token(&user_token).await? {
            return Err(AppError::ExternalApi("获取的userToken无效".to_string()));
        }

        // 添加到token列表
        let accounts_count = {
            let mut tokens = self.user_tokens.write();
            let token_list = tokens.entry(api_key.clone()).or_insert_with(Vec::new);
            
            // 避免重复添加相同的token
            if !token_list.contains(&user_token) {
                token_list.push(user_token.clone());
            }
            
            token_list.len()
        };

        // 保存到存储
        if let Err(e) = self.save_to_storage() {
            warn!("保存账户信息失败: {}", e);
        }

        info!("成功为API密钥 {} 添加账户 {}，当前共有 {} 个账户", api_key, email, accounts_count);

        Ok(AddAccountResponse {
            success: true,
            message: format!("成功添加账户 {}", email),
            accounts_count,
        })
    }

    /// 获取API密钥的可用userToken
    pub fn get_user_token(&self, api_key: &str) -> AppResult<String> {
        if !self.is_api_key_valid(api_key)? {
            return Err(AppError::Unauthorized("无效的API密钥".to_string()));
        }

        let tokens = self.user_tokens.read();
        let token_list = tokens.get(api_key)
            .ok_or_else(|| AppError::NotFound("未找到关联的账户".to_string()))?;

        if token_list.is_empty() {
            return Err(AppError::NotFound("该API密钥下没有可用的账户".to_string()));
        }

        // 简单的轮询策略，可以后续扩展为更复杂的负载均衡
        let index = rand::random::<usize>() % token_list.len();
        let user_token = token_list[index].clone();

        // 记录使用次数
        self.increment_usage(api_key);

        Ok(user_token)
    }

    /// 检查API密钥是否有效
    pub fn is_api_key_valid(&self, api_key: &str) -> AppResult<bool> {
        let keys = self.api_keys.read();
        
        if let Some(key_info) = keys.get(api_key) {
            if !key_info.is_active {
                return Ok(false);
            }

            // 检查是否过期
            if let Some(expires_at) = key_info.expires_at {
                let now = SystemTime::now().duration_since(UNIX_EPOCH)
                    .map_err(|e| AppError::Internal(format!("获取时间戳失败: {}", e)))?
                    .as_secs();
                
                if now > expires_at {
                    return Ok(false);
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 获取API密钥信息
    pub fn get_api_key_info(&self, api_key: &str) -> AppResult<ApiKeyInfo> {
        let keys = self.api_keys.read();
        let key_info = keys.get(api_key)
            .ok_or_else(|| AppError::NotFound("API密钥不存在".to_string()))?;

        let tokens = self.user_tokens.read();
        let accounts_count = tokens.get(api_key)
            .map(|t| t.len())
            .unwrap_or(0);

        Ok(ApiKeyInfo {
            id: key_info.id.clone(),
            name: key_info.name.clone(),
            accounts_count,
            usage_count: key_info.usage_count,
            created_at: key_info.created_at,
            expires_at: key_info.expires_at,
            is_active: key_info.is_active,
        })
    }

    /// 列出所有API密钥
    pub fn list_api_keys(&self) -> Vec<ApiKeyInfo> {
        let keys = self.api_keys.read();
        let tokens = self.user_tokens.read();

        keys.iter().map(|(api_key, key_info)| {
            let accounts_count = tokens.get(api_key)
                .map(|t| t.len())
                .unwrap_or(0);

            ApiKeyInfo {
                id: key_info.id.clone(),
                name: key_info.name.clone(),
                accounts_count,
                usage_count: key_info.usage_count,
                created_at: key_info.created_at,
                expires_at: key_info.expires_at,
                is_active: key_info.is_active,
            }
        }).collect()
    }

    /// 停用API密钥
    pub fn deactivate_api_key(&self, api_key: &str) -> AppResult<()> {
        let mut keys = self.api_keys.write();
        if let Some(key_info) = keys.get_mut(api_key) {
            key_info.is_active = false;
            
            if let Err(e) = self.save_to_storage() {
                warn!("保存API密钥状态失败: {}", e);
            }
            
            info!("API密钥已停用: {}", api_key);
            Ok(())
        } else {
            Err(AppError::NotFound("API密钥不存在".to_string()))
        }
    }

    /// 增加使用次数
    fn increment_usage(&self, api_key: &str) {
        let mut keys = self.api_keys.write();
        if let Some(key_info) = keys.get_mut(api_key) {
            key_info.usage_count += 1;
        }
    }

    /// 保存到存储
    fn save_to_storage(&self) -> AppResult<()> {
        // 创建目录（如果不存在）
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::Internal(format!("创建存储目录失败: {}", e)))?;
        }

        let keys = self.api_keys.read();
        let tokens = self.user_tokens.read();

        let storage_data = serde_json::json!({
            "api_keys": *keys,
            "user_tokens": *tokens,
            "saved_at": SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });

        fs::write(&self.storage_path, serde_json::to_string_pretty(&storage_data)?)
            .map_err(|e| AppError::Internal(format!("写入存储文件失败: {}", e)))?;

        debug!("API密钥数据已保存到: {}", self.storage_path);
        Ok(())
    }

    /// 从存储加载
    fn load_from_storage(&self) -> AppResult<()> {
        if !Path::new(&self.storage_path).exists() {
            debug!("存储文件不存在，跳过加载: {}", self.storage_path);
            return Ok(());
        }

        let content = fs::read_to_string(&self.storage_path)
            .map_err(|e| AppError::Internal(format!("读取存储文件失败: {}", e)))?;

        let storage_data: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(api_keys_data) = storage_data.get("api_keys") {
            if let Ok(api_keys) = serde_json::from_value::<HashMap<String, ApiKey>>(api_keys_data.clone()) {
                *self.api_keys.write() = api_keys;
            }
        }

        if let Some(user_tokens_data) = storage_data.get("user_tokens") {
            if let Ok(user_tokens) = serde_json::from_value::<HashMap<String, Vec<String>>>(user_tokens_data.clone()) {
                *self.user_tokens.write() = user_tokens;
            }
        }

        info!("成功从存储加载API密钥数据: {}", self.storage_path);
        Ok(())
    }

    /// 清理过期的API密钥
    pub async fn cleanup_expired_keys(&self) -> AppResult<usize> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::Internal(format!("获取时间戳失败: {}", e)))?
            .as_secs();

        let mut cleaned_count = 0;
        
        {
            let mut keys = self.api_keys.write();
            let mut tokens = self.user_tokens.write();
            
            keys.retain(|api_key, key_info| {
                let should_keep = if let Some(expires_at) = key_info.expires_at {
                    now <= expires_at
                } else {
                    true // 没有过期时间，保留
                };
                
                if !should_keep {
                    tokens.remove(api_key);
                    cleaned_count += 1;
                    info!("清理过期API密钥: {}", api_key);
                }
                
                should_keep
            });
        }

        if cleaned_count > 0 {
            if let Err(e) = self.save_to_storage() {
                warn!("保存清理结果失败: {}", e);
            }
        }

        Ok(cleaned_count)
    }
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}
