use axum::{
    extract::{State, Json},
    response::Json as JsonResponse,
};
use crate::{
    error::{ApiError, ApiResult},
    models::*,
    handlers::AppState,
};
use tracing::{info, warn};

/// 创建API密钥
pub async fn create_api_key(
    State(state): State<AppState>,
    Json(request): Json<CreateApiKeyRequest>,
) -> ApiResult<JsonResponse<CreateApiKeyResponse>> {
    info!("创建API密钥请求: {}", request.name);

    let response = state.api_key_manager.create_api_key(
        request.name,
        request.expires_days,
    ).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(JsonResponse(response))
}

/// 添加账户到API密钥
pub async fn add_account(
    State(state): State<AppState>,
    Json(request): Json<AddAccountRequest>,
) -> ApiResult<JsonResponse<AddAccountResponse>> {
    info!("为API密钥添加账户: {}", request.email);

    let response = state.api_key_manager.add_account(
        request.api_key,
        request.email,
        request.password,
    ).await.map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(JsonResponse(response))
}

/// 获取API密钥信息
pub async fn get_api_key_info(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<JsonResponse<ApiKeyInfo>> {
    let api_key = request.get("api_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("缺少api_key参数".to_string()))?;

    let info = state.api_key_manager.get_api_key_info(api_key)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    
    Ok(JsonResponse(info))
}

/// 列出所有API密钥
pub async fn list_api_keys(
    State(state): State<AppState>,
) -> ApiResult<JsonResponse<Vec<ApiKeyInfo>>> {
    let keys = state.api_key_manager.list_api_keys();
    
    Ok(JsonResponse(keys))
}

/// 停用API密钥
pub async fn deactivate_api_key(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<JsonResponse<serde_json::Value>> {
    let api_key = request.get("api_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("缺少api_key参数".to_string()))?;

    state.api_key_manager.deactivate_api_key(api_key)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    
    Ok(JsonResponse(serde_json::json!({
        "success": true,
        "message": "API密钥已停用"
    })))
}

/// 直接登录获取userToken（调试用）
pub async fn login_for_token(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<JsonResponse<LoginResponse>> {
    info!("登录请求: {}", request.email);

    match state.login_service.login(&request.email, &request.password).await {
        Ok(user_token) => {
            Ok(JsonResponse(LoginResponse {
                user_token,
                success: true,
                message: Some("登录成功".to_string()),
            }))
        }
        Err(e) => {
            warn!("登录失败: {}", e);
            Ok(JsonResponse(LoginResponse {
                user_token: "".to_string(),
                success: false,
                message: Some(e.to_string()),
            }))
        }
    }
}

/// 验证userToken是否有效
pub async fn verify_user_token(
    State(state): State<AppState>,
    Json(request): Json<TokenCheckRequest>,
) -> ApiResult<JsonResponse<TokenCheckResponse>> {
    let is_valid = state.login_service.verify_token(&request.token).await
        .unwrap_or(false);

    Ok(JsonResponse(TokenCheckResponse {
        live: is_valid,
    }))
}

/// 清理过期的API密钥
pub async fn cleanup_expired_keys(
    State(state): State<AppState>,
) -> ApiResult<JsonResponse<serde_json::Value>> {
    let cleaned_count = state.api_key_manager.cleanup_expired_keys().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    
    Ok(JsonResponse(serde_json::json!({
        "success": true,
        "message": format!("清理了 {} 个过期的API密钥", cleaned_count),
        "cleaned_count": cleaned_count
    })))
}

/// 获取会话池统计信息
pub async fn get_session_pool_stats(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<JsonResponse<serde_json::Value>> {
    let api_key = request.get("api_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("缺少api_key参数".to_string()))?;

    if let Some(stats) = state.api_key_manager.get_session_pool_stats(api_key) {
        Ok(JsonResponse(serde_json::json!(stats)))
    } else {
        Err(ApiError::NotFound("API密钥不存在或无统计信息".to_string()))
    }
}
