use crate::error::{ApiError, ApiResult};
use crate::models::{DeepSeekResponse, UserInfo};
use crate::utils::{generate_cookie, unix_timestamp};
use parking_lot::RwLock;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Token信息
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub access_token: String,
    pub refresh_token: String,
    pub expire_time: u64,
}

/// Token管理器
pub struct TokenManager {
    client: Client,
    tokens: Arc<RwLock<HashMap<String, TokenInfo>>>,
    request_semaphores: Arc<RwLock<HashMap<String, Arc<Semaphore>>>>,
    access_token_expires: u64,
}

impl TokenManager {
    pub fn new(client: Client, access_token_expires: u64) -> Self {
        Self {
            client,
            tokens: Arc::new(RwLock::new(HashMap::new())),
            request_semaphores: Arc::new(RwLock::new(HashMap::new())),
            access_token_expires,
        }
    }

    /// 获取访问令牌
    pub async fn acquire_token(&self, refresh_token: &str) -> ApiResult<String> {
        // 检查是否需要刷新
        let current_time = unix_timestamp();
        
        {
            let tokens = self.tokens.read();
            if let Some(token_info) = tokens.get(refresh_token) {
                if current_time < token_info.expire_time {
                    return Ok(token_info.access_token.clone());
                }
            }
        }

        // 获取或创建信号量
        let semaphore = {
            let mut semaphores = self.request_semaphores.write();
            semaphores
                .entry(refresh_token.to_string())
                .or_insert_with(|| Arc::new(Semaphore::new(1)))
                .clone()
        };

        // 使用信号量确保只有一个请求在刷新token
        let _permit = semaphore.acquire().await.map_err(|e| {
            ApiError::InternalError(format!("Failed to acquire semaphore: {}", e))
        })?;

        // 双重检查锁定模式
        {
            let tokens = self.tokens.read();
            if let Some(token_info) = tokens.get(refresh_token) {
                if current_time < token_info.expire_time {
                    return Ok(token_info.access_token.clone());
                }
            }
        }

        // 刷新token
        let token_info = self.refresh_token(refresh_token).await?;
        
        // 更新缓存
        {
            let mut tokens = self.tokens.write();
            tokens.insert(refresh_token.to_string(), token_info.clone());
        }

        Ok(token_info.access_token)
    }

    /// 刷新token
    async fn refresh_token(&self, refresh_token: &str) -> ApiResult<TokenInfo> {
        tracing::info!("Refreshing token: {}", refresh_token);

        let headers = self.create_headers(Some(refresh_token));
        
        let response = self
            .client
            .get("https://chat.deepseek.com/api/v0/users/current")
            .headers(headers)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;

        let result: DeepSeekResponse<UserInfo> = response.json().await?;
        
        match result.biz_data {
            Some(user_info) => {
                tracing::info!("Token refresh successful");
                Ok(TokenInfo {
                    access_token: user_info.token.clone(),
                    refresh_token: user_info.token,
                    expire_time: unix_timestamp() + self.access_token_expires,
                })
            }
            None => {
                let error_msg = result.msg.unwrap_or_else(|| "Unknown error".to_string());
                if let Some(code) = result.code {
                    if code == 40003 {
                        // Token无效，从缓存中移除
                        self.remove_token(refresh_token);
                    }
                    Err(ApiError::DeepSeekApiError {
                        code,
                        message: error_msg,
                    })
                } else {
                    Err(ApiError::TokenError(error_msg))
                }
            }
        }
    }

    /// 检查token是否有效
    pub async fn check_token_status(&self, refresh_token: &str) -> ApiResult<bool> {
        match self.acquire_token(refresh_token).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 移除无效的token
    pub fn remove_token(&self, refresh_token: &str) {
        let mut tokens = self.tokens.write();
        tokens.remove(refresh_token);
    }

    /// 创建请求头
    fn create_headers(&self, auth_token: Option<&str>) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        
        headers.insert("Accept", "*/*".parse().unwrap());
        headers.insert("Accept-Encoding", "gzip, deflate, br, zstd".parse().unwrap());
        headers.insert("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8".parse().unwrap());
        headers.insert("Origin", "https://chat.deepseek.com".parse().unwrap());
        headers.insert("Pragma", "no-cache".parse().unwrap());
        headers.insert("Priority", "u=1, i".parse().unwrap());
        headers.insert("Referer", "https://chat.deepseek.com/".parse().unwrap());
        headers.insert(
            "Sec-Ch-Ua",
            r#""Chromium";v="134", "Not:A-Brand";v="24", "Google Chrome";v="134""#.parse().unwrap()
        );
        headers.insert("Sec-Ch-Ua-Mobile", "?0".parse().unwrap());
        headers.insert("Sec-Ch-Ua-Platform", r#""macOS""#.parse().unwrap());
        headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
        headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
        headers.insert("Sec-Fetch-Site", "same-origin".parse().unwrap());
        headers.insert(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36".parse().unwrap()
        );
        headers.insert("X-App-Version", "20241129.1".parse().unwrap());
        headers.insert("X-Client-Locale", "zh-CN".parse().unwrap());
        headers.insert("X-Client-Platform", "web".parse().unwrap());
        headers.insert("X-Client-Version", "1.0.0-always".parse().unwrap());
        headers.insert("Cookie", generate_cookie().parse().unwrap());

        if let Some(token) = auth_token {
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap()
            );
        }

        headers
    }

    /// 清理过期的semaphore
    pub async fn cleanup_semaphores(&self) {
        let mut semaphores = self.request_semaphores.write();
        semaphores.retain(|_, semaphore| semaphore.available_permits() > 0);
    }
}
