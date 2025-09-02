use crate::config::Config;
use crate::error::{ApiError, ApiResult};
use crate::models::*;
use crate::services::{ChallengeSolver, MessageProcessor, TokenManager};
use crate::utils::{
    generate_cookie, is_search_model, is_thinking_model,
    parse_conversation_id, unix_timestamp,
};
use futures_util::Stream;
use reqwest::Client;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// DeepSeek客户端
pub struct DeepSeekClient {
    client: Client,
    config: Config,
    token_manager: TokenManager,
    challenge_solver: ChallengeSolver,
    message_processor: MessageProcessor,
}

impl DeepSeekClient {
    pub fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();

        let token_manager = TokenManager::new(client.clone(), config.deepseek.access_token_expires);
        let challenge_solver = ChallengeSolver::new(config.deepseek.wasm_path.clone());
        let message_processor = MessageProcessor;

        Self {
            client,
            config,
            token_manager,
            challenge_solver,
            message_processor,
        }
    }

    /// 创建聊天完成
    pub async fn create_completion(
        &self,
        model: &str,
        messages: &[ChatMessage],
        token: &str,
        conversation_id: Option<&str>,
    ) -> ApiResult<ChatCompletionResponse> {
        let mut retry_count = 0;
        let max_retries = self.config.deepseek.max_retry_count;

        loop {
            match self
                .try_create_completion(model, messages, token, conversation_id)
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) if retry_count < max_retries => {
                    tracing::warn!("Completion failed, retrying: {}", e);
                    retry_count += 1;
                    tokio::time::sleep(Duration::from_millis(self.config.deepseek.retry_delay_ms))
                        .await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 尝试创建聊天完成
    async fn try_create_completion(
        &self,
        model: &str,
        messages: &[ChatMessage],
        token: &str,
        conversation_id: Option<&str>,
    ) -> ApiResult<ChatCompletionResponse> {
        tracing::info!("Creating completion for model: {}", model);

        // 解析对话ID
        let (ref_session_id, ref_parent_msg_id) = if let Some(conv_id) = conversation_id {
            parse_conversation_id(conv_id).unzip()
        } else {
            (None, None)
        };

        // 消息预处理
        let prompt = MessageProcessor::prepare_messages(messages);
        
        // 检查模型类型
        let is_search = is_search_model(model) || prompt.contains("联网搜索");
        let is_thinking = is_thinking_model(model) || prompt.contains("深度思考");

        // 检查深度思考配额
        if is_thinking {
            let quota = self.get_thinking_quota(token).await?;
            if quota <= 0 {
                return Err(ApiError::ServiceUnavailable("深度思考配额不足".to_string()));
            }
        }

        // 获取POW挑战并解决
        let challenge_response = self.get_challenge(token, "/api/v0/chat/completion").await?;
        let challenge_answer = self
            .challenge_solver
            .solve_challenge(&challenge_response.challenge, "/api/v0/chat/completion")
            .await?;

        // 创建会话
        let session_id = if let Some(id) = ref_session_id {
            id
        } else {
            self.create_session(token).await?
        };

        // 发送完成请求
        let access_token = self.token_manager.acquire_token(token).await?;
        let completion_request = CompletionRequest {
            chat_session_id: session_id.clone(),
            parent_message_id: ref_parent_msg_id,
            prompt,
            ref_file_ids: vec![],
            search_enabled: is_search,
            thinking_enabled: is_thinking,
        };

        let mut headers = self.create_headers(&access_token);
        headers.insert("X-Ds-Pow-Response", challenge_answer.parse().unwrap());

        let response = self
            .client
            .post(&format!("{}/api/v0/chat/completion", self.config.deepseek.base_url))
            .headers(headers)
            .json(&completion_request)
            .send()
            .await?;

        // 发送事件以降低封号风险
        let _ = self.send_events(&session_id, token).await;

        if response.headers().get("content-type")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.contains("text/event-stream"))
            .unwrap_or(false)
        {
            // 处理流式响应
            self.process_completion_stream(response, model, &session_id).await
        } else {
            Err(ApiError::ServiceUnavailable(
                "服务暂时不可用，第三方响应错误".to_string(),
            ))
        }
    }

    /// 创建流式聊天完成
    pub async fn create_completion_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        token: &str,
        conversation_id: Option<&str>,
    ) -> ApiResult<Pin<Box<dyn Stream<Item = Result<String, ApiError>> + Send>>> {
        let mut retry_count = 0;
        let max_retries = self.config.deepseek.max_retry_count;

        loop {
            match self
                .try_create_completion_stream(model, messages, token, conversation_id)
                .await
            {
                Ok(stream) => return Ok(stream),
                Err(e) if retry_count < max_retries => {
                    tracing::warn!("Stream creation failed, retrying: {}", e);
                    retry_count += 1;
                    tokio::time::sleep(Duration::from_millis(self.config.deepseek.retry_delay_ms))
                        .await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 尝试创建流式聊天完成
    async fn try_create_completion_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        token: &str,
        conversation_id: Option<&str>,
    ) -> ApiResult<Pin<Box<dyn Stream<Item = Result<String, ApiError>> + Send>>> {
        tracing::info!("Creating completion stream for model: {}", model);

        // 解析对话ID
        let (ref_session_id, ref_parent_msg_id) = if let Some(conv_id) = conversation_id {
            parse_conversation_id(conv_id).unzip()
        } else {
            (None, None)
        };

        // 消息预处理
        let prompt = MessageProcessor::prepare_messages(messages);
        
        // 检查模型类型
        let is_search = is_search_model(model) || prompt.contains("联网搜索");
        let is_thinking = is_thinking_model(model) || prompt.contains("深度思考");

        // 检查深度思考配额
        if is_thinking {
            let quota = self.get_thinking_quota(token).await?;
            if quota <= 0 {
                return Err(ApiError::ServiceUnavailable("深度思考配额不足".to_string()));
            }
        }

        // 获取POW挑战并解决
        let challenge_response = self.get_challenge(token, "/api/v0/chat/completion").await?;
        let challenge_answer = self
            .challenge_solver
            .solve_challenge(&challenge_response.challenge, "/api/v0/chat/completion")
            .await?;

        // 创建会话
        let session_id = if let Some(id) = ref_session_id {
            id
        } else {
            self.create_session(token).await?
        };

        // 发送完成请求
        let access_token = self.token_manager.acquire_token(token).await?;
        let completion_request = CompletionRequest {
            chat_session_id: session_id.clone(),
            parent_message_id: ref_parent_msg_id,
            prompt,
            ref_file_ids: vec![],
            search_enabled: is_search,
            thinking_enabled: is_thinking,
        };

        let mut headers = self.create_headers(&access_token);
        headers.insert("X-Ds-Pow-Response", challenge_answer.parse().unwrap());

        let response = self
            .client
            .post(&format!("{}/api/v0/chat/completion", self.config.deepseek.base_url))
            .headers(headers)
            .json(&completion_request)
            .send()
            .await?;

        // 发送事件以降低封号风险
        let session_id_clone = session_id.clone();
        let token_clone = token.to_string();
        let client_clone = self.clone();
        tokio::spawn(async move {
            let _ = client_clone.send_events(&session_id_clone, &token_clone).await;
        });

        if response.headers().get("content-type")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.contains("text/event-stream"))
            .unwrap_or(false)
        {
            // 创建转换流
            let stream = self.create_transform_stream(response, model, session_id).await?;
            Ok(stream)
        } else {
            Err(ApiError::ServiceUnavailable(
                "服务暂时不可用，第三方响应错误".to_string(),
            ))
        }
    }

    /// 处理完成流并返回完整响应
    async fn process_completion_stream(
        &self,
        response: reqwest::Response,
        model: &str,
        session_id: &str,
    ) -> ApiResult<ChatCompletionResponse> {
        let mut content = String::new();
        let message_id = "1".to_string(); // 简化处理

        // 简化流处理
        let bytes = response.bytes().await?;
        let text = String::from_utf8_lossy(&bytes);
        
        // 模拟处理SSE数据
        for line in text.lines() {
            if line.starts_with("data: ") && !line.contains("[DONE]") {
                let data_part = &line[6..]; // 移除 "data: " 前缀
                if let Ok(data) = serde_json::from_str::<DeepSeekStreamData>(data_part) {
                    if let Some(choices) = &data.choices {
                        for choice in choices {
                            if let Some(delta_content) = &choice.delta.content {
                                content.push_str(delta_content);
                            }
                        }
                    }
                }
            }
        }

        // 构造响应
        let final_content = MessageProcessor::add_search_references(&content, "");
        let conv_id = format!("{}@{}", session_id, message_id);

        Ok(ChatCompletionResponse {
            id: conv_id,
            object: "chat.completion".to_string(),
            created: unix_timestamp(),
            model: model.to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: Some(ChatMessage {
                    role: "assistant".to_string(),
                    content: ChatMessageContent::Text(final_content),
                }),
                delta: None,
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
            }),
        })
    }

    /// 创建转换流
    async fn create_transform_stream(
        &self,
        response: reqwest::Response,
        model: &str,
        session_id: String,
    ) -> ApiResult<Pin<Box<dyn Stream<Item = Result<String, ApiError>> + Send>>> {
        let (tx, rx) = mpsc::channel(100);
        let created = unix_timestamp();
        
        // 发送初始chunk
        let initial_chunk = StreamChunk {
            id: String::new(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: ChatMessageDelta {
                    role: Some("assistant".to_string()),
                    content: Some(String::new()),
                    reasoning_content: None,
                },
                finish_reason: None,
            }],
        };
        
        let initial_data = format!("data: {}\n\n", serde_json::to_string(&initial_chunk)?);
        if tx.send(Ok(initial_data)).await.is_err() {
            return Err(ApiError::InternalError("Failed to send initial chunk".to_string()));
        }

        // 启动后台任务处理流
        let model_clone = model.to_string();
        tokio::spawn(async move {
            // 简化流处理
            let bytes = match response.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    let _ = tx.send(Err(ApiError::HttpRequest(e))).await;
                    return;
                }
            };
            
            let text = String::from_utf8_lossy(&bytes);
            
            // 模拟处理SSE数据
            for line in text.lines() {
                if line.starts_with("data: ") && !line.contains("[DONE]") {
                    let data_part = &line[6..]; // 移除 "data: " 前缀
                    if let Ok(data) = serde_json::from_str::<DeepSeekStreamData>(data_part) {
                        if let Some(choices) = &data.choices {
                            for choice in choices {
                                if let Some(delta_content) = &choice.delta.content {
                                    let chunk = StreamChunk {
                                        id: format!("{}@1", session_id),
                                        object: "chat.completion.chunk".to_string(),
                                        created,
                                        model: model_clone.clone(),
                                        choices: vec![StreamChoice {
                                            index: 0,
                                            delta: ChatMessageDelta {
                                                role: Some("assistant".to_string()),
                                                content: Some(delta_content.clone()),
                                                reasoning_content: None,
                                            },
                                            finish_reason: None,
                                        }],
                                    };

                                    let chunk_data = format!(
                                        "data: {}\n\n",
                                        serde_json::to_string(&chunk).unwrap_or_default()
                                    );

                                    if tx.send(Ok(chunk_data)).await.is_err() {
                                        return;
                                    }
                                }

                                if choice.finish_reason.is_some() {
                                    // 发送结束chunk
                                    let final_chunk = StreamChunk {
                                        id: format!("{}@1", session_id),
                                        object: "chat.completion.chunk".to_string(),
                                        created,
                                        model: model_clone.clone(),
                                        choices: vec![StreamChoice {
                                            index: 0,
                                            delta: ChatMessageDelta {
                                                role: Some("assistant".to_string()),
                                                content: Some(String::new()),
                                                reasoning_content: None,
                                            },
                                            finish_reason: Some("stop".to_string()),
                                        }],
                                    };

                                    let final_data = format!(
                                        "data: {}\n\n",
                                        serde_json::to_string(&final_chunk).unwrap_or_default()
                                    );

                                    let _ = tx.send(Ok(final_data)).await;
                                    let _ = tx.send(Ok("data: [DONE]\n\n".to_string())).await;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            
            // 如果没有结束标记，手动发送结束
            let _ = tx.send(Ok("data: [DONE]\n\n".to_string())).await;
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    /// 创建会话
    async fn create_session(&self, token: &str) -> ApiResult<String> {
        let access_token = self.token_manager.acquire_token(token).await?;
        let headers = self.create_headers(&access_token);

        let session_request = serde_json::json!({
            "character_id": null
        });

        let response = self
            .client
            .post(&format!("{}/api/v0/chat_session/create", self.config.deepseek.base_url))
            .headers(headers)
            .json(&session_request)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;

        let result: DeepSeekResponse<ChatSession> = response.json().await?;
        
        match result.biz_data {
            Some(session) => Ok(session.id),
            None => Err(ApiError::ServiceUnavailable(
                "创建会话失败，可能是账号或IP地址被封禁".to_string(),
            )),
        }
    }

    /// 获取挑战
    async fn get_challenge(&self, token: &str, target_path: &str) -> ApiResult<ChallengeResponse> {
        let access_token = self.token_manager.acquire_token(token).await?;
        let headers = self.create_headers(&access_token);

        let challenge_request = ChallengeRequest {
            target_path: target_path.to_string(),
        };

        let response = self
            .client
            .post(&format!("{}/api/v0/chat/create_pow_challenge", self.config.deepseek.base_url))
            .headers(headers)
            .json(&challenge_request)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;

        let result: DeepSeekResponse<ChallengeResponse> = response.json().await?;
        
        match result.biz_data {
            Some(challenge_resp) => Ok(challenge_resp),
            None => Err(ApiError::ChallengeError("获取挑战失败".to_string())),
        }
    }

    /// 获取深度思考配额
    async fn get_thinking_quota(&self, token: &str) -> ApiResult<u32> {
        let access_token = self.token_manager.acquire_token(token).await?;
        let headers = self.create_headers(&access_token);

        let response = self
            .client
            .get(&format!("{}/api/v0/users/feature_quota", self.config.deepseek.base_url))
            .headers(headers)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;

        let result: DeepSeekResponse<FeatureQuota> = response.json().await?;
        
        match result.biz_data {
            Some(quota) => {
                let remaining = quota.thinking.quota.saturating_sub(quota.thinking.used);
                tracing::info!("Thinking quota: {}/{}", quota.thinking.used, quota.thinking.quota);
                Ok(remaining)
            }
            None => {
                tracing::warn!("Failed to get thinking quota");
                Ok(0)
            }
        }
    }

    /// 发送事件
    async fn send_events(&self, _session_id: &str, _token: &str) -> ApiResult<()> {
        // 实现事件发送逻辑，类似原TypeScript代码中的sendEvents函数
        // 这里简化实现
        tracing::debug!("Sending events for session: {}", _session_id);
        Ok(())
    }

    /// 检查token状态
    pub async fn check_token_status(&self, token: &str) -> ApiResult<bool> {
        self.token_manager.check_token_status(token).await
    }

    /// 创建请求头
    fn create_headers(&self, auth_token: &str) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        
        headers.insert("Accept", "*/*".parse().unwrap());
        headers.insert("Accept-Encoding", "gzip, deflate, br, zstd".parse().unwrap());
        headers.insert("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8".parse().unwrap());
        headers.insert("Origin", self.config.deepseek.base_url.parse().unwrap());
        headers.insert("Pragma", "no-cache".parse().unwrap());
        headers.insert("Priority", "u=1, i".parse().unwrap());
        headers.insert("Referer", format!("{}/", self.config.deepseek.base_url).parse().unwrap());
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
        headers.insert("Authorization", format!("Bearer {}", auth_token).parse().unwrap());

        headers
    }
}

impl Clone for DeepSeekClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            token_manager: TokenManager::new(self.client.clone(), self.config.deepseek.access_token_expires),
            challenge_solver: ChallengeSolver::new(self.config.deepseek.wasm_path.clone()),
            message_processor: MessageProcessor,
        }
    }
}
