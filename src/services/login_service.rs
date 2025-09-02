use crate::error::{AppError, AppResult};
use crate::models::*;
use reqwest::{Client, cookie::Jar};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH, SystemTime};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{info, warn, error, debug};

pub struct LoginService {
    client: Client,
    base_url: String,
}

impl LoginService {
    pub fn new() -> Self {
        // 创建一个支持cookie的HTTP客户端
        let jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_store(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: "https://chat.deepseek.com".to_string(),
        }
    }

    /// 登录DeepSeek并获取userToken
    pub async fn login(&self, email: &str, password: &str) -> AppResult<String> {
        info!("开始DeepSeek登录流程: {}", email);

        // 1. 首先访问登录页面获取必要的cookies和信息
        let login_page_url = format!("{}/sign_in", self.base_url);
        let response = self.client.get(&login_page_url).send().await
            .map_err(|e| AppError::ExternalApi(format!("访问登录页面失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::ExternalApi(format!("登录页面访问失败: {}", response.status())));
        }

        debug!("成功访问登录页面");

        // 2. 准备登录请求
        let login_url = format!("{}/api/v1/users/login", self.base_url);
        let login_payload = json!({
            "email": email,
            "password": password,
            "remember_me": true
        });

        // 3. 发送登录请求
        let login_response = self.client
            .post(&login_url)
            .header("Content-Type", "application/json")
            .header("Referer", &login_page_url)
            .header("X-Requested-With", "XMLHttpRequest")
            .json(&login_payload)
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("登录请求失败: {}", e)))?;

        let status = login_response.status();
        let response_text = login_response.text().await
            .map_err(|e| AppError::ExternalApi(format!("读取登录响应失败: {}", e)))?;

        debug!("登录响应状态: {}, 内容: {}", status, response_text);

        if !status.is_success() {
            // 尝试解析错误信息
            if let Ok(error_json) = serde_json::from_str::<Value>(&response_text) {
                let error_msg = error_json.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("登录失败");
                return Err(AppError::ExternalApi(format!("DeepSeek登录失败: {}", error_msg)));
            }
            return Err(AppError::ExternalApi(format!("登录失败，状态码: {}", status)));
        }

        // 4. 解析登录响应
        let login_result: Value = serde_json::from_str(&response_text)
            .map_err(|e| AppError::ExternalApi(format!("解析登录响应失败: {}", e)))?;

        // 检查登录是否成功
        if let Some(code) = login_result.get("code").and_then(|v| v.as_u64()) {
            if code != 0 {
                let error_msg = login_result.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("未知错误");
                return Err(AppError::ExternalApi(format!("DeepSeek登录失败: {}", error_msg)));
            }
        }

        // 5. 尝试通过不同方式获取token
        let user_token = self.extract_user_token(&login_result).await?;

        info!("DeepSeek登录成功，获取到userToken: {}...", 
              &user_token[..std::cmp::min(20, user_token.len())]);

        Ok(user_token)
    }

    /// 从登录响应或后续请求中提取userToken
    async fn extract_user_token(&self, login_response: &Value) -> AppResult<String> {
        // 方法1: 从登录响应中直接获取
        if let Some(token) = login_response.get("data")
            .and_then(|d| d.get("token"))
            .and_then(|t| t.as_str()) {
            return Ok(token.to_string());
        }

        // 方法2: 从响应的access_token字段获取
        if let Some(token) = login_response.get("access_token")
            .and_then(|t| t.as_str()) {
            return Ok(token.to_string());
        }

        // 方法3: 访问用户信息页面获取token
        debug!("尝试从用户信息接口获取token");
        let user_info_url = format!("{}/api/v1/users/current", self.base_url);
        let user_response = self.client.get(&user_info_url).send().await
            .map_err(|e| AppError::ExternalApi(format!("获取用户信息失败: {}", e)))?;

        if user_response.status().is_success() {
            let user_text = user_response.text().await
                .map_err(|e| AppError::ExternalApi(format!("读取用户信息失败: {}", e)))?;
            
            if let Ok(user_json) = serde_json::from_str::<Value>(&user_text) {
                if let Some(token) = user_json.get("data")
                    .and_then(|d| d.get("token"))
                    .and_then(|t| t.as_str()) {
                    return Ok(token.to_string());
                }
            }
        }

        // 方法4: 尝试访问聊天页面，从页面中提取token
        debug!("尝试从聊天页面获取token");
        let chat_url = format!("{}/", self.base_url);
        let chat_response = self.client.get(&chat_url).send().await
            .map_err(|e| AppError::ExternalApi(format!("访问聊天页面失败: {}", e)))?;

        if chat_response.status().is_success() {
            let html_content = chat_response.text().await
                .map_err(|e| AppError::ExternalApi(format!("读取聊天页面失败: {}", e)))?;
            
            // 尝试从HTML中提取token（通常在window.__INITIAL_STATE__或类似的变量中）
            if let Some(token) = self.extract_token_from_html(&html_content) {
                return Ok(token);
            }
        }

        // 方法5: 尝试从cookies中获取token
        debug!("尝试从cookies获取token");
        if let Some(token) = self.extract_token_from_cookies().await {
            return Ok(token);
        }

        Err(AppError::ExternalApi("无法获取userToken，登录可能失败".to_string()))
    }

    /// 从HTML内容中提取token
    fn extract_token_from_html(&self, html: &str) -> Option<String> {
        // 常见的token提取模式
        let patterns = [
            r#""token":"([^"]+)""#,
            r#"'token':'([^']+)'"#,
            r#"userToken["\s]*:["\s]*"([^"]+)""#,
            r#"access_token["\s]*:["\s]*"([^"]+)""#,
            r#"authToken["\s]*:["\s]*"([^"]+)""#,
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(html) {
                    if let Some(token) = captures.get(1) {
                        let token_str = token.as_str().trim();
                        if !token_str.is_empty() && token_str.len() > 10 {
                            debug!("从HTML中提取到token: {}...", &token_str[..std::cmp::min(20, token_str.len())]);
                            return Some(token_str.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// 从cookies中提取token
    async fn extract_token_from_cookies(&self) -> Option<String> {
        // 这里可以实现从cookies中提取token的逻辑
        // 由于reqwest的cookie jar API限制，这里先返回None
        // 在实际实现中，可能需要使用其他方法来访问cookies
        None
    }

    /// 验证token是否有效
    pub async fn verify_token(&self, token: &str) -> AppResult<bool> {
        let verify_url = format!("{}/api/v1/chat/sessions", self.base_url);
        
        let response = self.client
            .get(&verify_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("验证token失败: {}", e)))?;

        Ok(response.status().is_success())
    }

    /// 批量登录多个账户
    pub async fn batch_login(&self, accounts: Vec<(String, String)>) -> Vec<(String, Result<String, String>)> {
        let mut results = Vec::new();
        
        for (email, password) in accounts {
            let result = match self.login(&email, &password).await {
                Ok(token) => Ok(token),
                Err(e) => Err(e.to_string()),
            };
            results.push((email, result));
        }
        
        results
    }
}

impl Default for LoginService {
    fn default() -> Self {
        Self::new()
    }
}
