use crate::error::{ApiError, ApiResult};
use crate::handlers::AppState;
use crate::models::ChatCompletionRequest;
use axum::{
    extract::State,
    http::HeaderMap,
    response::{sse::Event, Json, Sse, IntoResponse, Response},
};
use futures_util::{stream::StreamExt, Stream};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::pin::Pin;

/// 聊天补全处理器  
pub async fn completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response, ApiError> {
    // 验证请求
    if request.messages.is_empty() {
        return Err(ApiError::InvalidRequest("Messages cannot be empty".to_string()));
    }

    // 获取用户token和会话
    let (conversation_id, session) = if let Some(api_key) = get_api_key_from_header(&headers) {
        // 使用API密钥和会话池
        let (conv_id, session) = state.api_key_manager.acquire_session(&api_key, request.conversation_id.clone()).await
            .map_err(|e| ApiError::TokenError(format!("Failed to acquire session: {}", e)))?;
        (Some(conv_id), Some(session))
    } else {
        // 兼容模式：直接使用userToken
        let _user_token = get_authorization_and_token(&headers, &state)?;
        (request.conversation_id.clone(), None)
    };

    let user_token = session.as_ref()
        .map(|s| s.user_token.clone())
        .unwrap_or_else(|| get_authorization_and_token(&headers, &state).unwrap_or_default());

    let model = request.model.as_deref().unwrap_or("deepseek").to_lowercase();
    let stream = request.stream.unwrap_or(false);

    let result = if stream {
        // 流式响应
        let stream = state
            .client
            .create_completion_stream(&model, &request.messages, &user_token, conversation_id.as_deref())
            .await?;

        let sse_stream = create_sse_stream(stream);
        Ok(Sse::new(sse_stream).into_response())
    } else {
        // 非流式响应
        let response = state
            .client
            .create_completion(&model, &request.messages, &user_token, conversation_id.as_deref())
            .await?;

        Ok(Json(response).into_response())
    };

    // 释放会话
    if let Some(conv_id) = conversation_id {
        state.api_key_manager.release_session(&conv_id);
    }

    result
}

/// 获取模型列表
pub async fn models() -> Json<Value> {
    Json(json!({
        "object": "list",
        "data": [
            {
                "id": "deepseek",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek",
                "parent": null
            },
            {
                "id": "deepseek-search",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-search",
                "parent": null
            },
            {
                "id": "deepseek-think",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-think",
                "parent": null
            },
            {
                "id": "deepseek-r1",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-r1",
                "parent": null
            },
            {
                "id": "deepseek-r1-search",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-r1-search",
                "parent": null
            },
            {
                "id": "deepseek-think-search",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-think-search",
                "parent": null
            },
            {
                "id": "deepseek-think-silent",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-think-silent",
                "parent": null
            },
            {
                "id": "deepseek-r1-silent",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-r1-silent",
                "parent": null
            },
            {
                "id": "deepseek-search-silent",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-search-silent",
                "parent": null
            },
            {
                "id": "deepseek-think-fold",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-think-fold",
                "parent": null
            },
            {
                "id": "deepseek-r1-fold",
                "object": "model",
                "created": 1234567890,
                "owned_by": "deepseek",
                "permission": [],
                "root": "deepseek-r1-fold",
                "parent": null
            }
        ]
    }))
}

/// 从请求头获取API密钥
fn get_api_key_from_header(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    
    if let Some(api_key) = auth_str.strip_prefix("Bearer dsk-") {
        Some(format!("dsk-{}", api_key))
    } else {
        None
    }
}

/// 获取授权头和用户token
fn get_authorization_and_token(headers: &HeaderMap, state: &AppState) -> ApiResult<String> {
    // 从请求头获取Authorization
    let auth_header = headers
        .get("authorization")
        .ok_or_else(|| ApiError::TokenError("Authorization header missing".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| ApiError::TokenError("Invalid authorization header format".to_string()))?;

    // 检查是否是API密钥格式 (Bearer dsk-xxxx)
    if let Some(api_key) = auth_str.strip_prefix("Bearer dsk-") {
        let api_key = format!("dsk-{}", api_key);
        
        // 验证API密钥并获取userToken
        match state.api_key_manager.get_user_token(&api_key) {
            Ok(user_token) => Ok(user_token),
            Err(_) => Err(ApiError::TokenError("Invalid API key or no accounts associated".to_string())),
        }
    } else if let Some(token) = auth_str.strip_prefix("Bearer ") {
        // 直接使用用户提供的userToken
        Ok(token.to_string())
    } else {
        // 优先使用环境变量中的token（兼容模式）
        if let Some(auth) = &state.config.deepseek.authorization {
            Ok(auth.clone())
        } else {
            Err(ApiError::TokenError("Invalid authorization format".to_string()))
        }
    }
}

/// 创建SSE流
fn create_sse_stream(
    stream: Pin<Box<dyn Stream<Item = Result<String, ApiError>> + Send>>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    stream.map(|result| match result {
        Ok(data) => Ok(Event::default().data(data)),
        Err(e) => {
            tracing::error!("Stream error: {}", e);
            // 发送错误事件
            let error_data = json!({
                "error": {
                    "message": e.to_string(),
                    "type": "stream_error"
                }
            });
            Ok(Event::default().data(format!("data: {}\n\n", error_data)))
        }
    })
}
