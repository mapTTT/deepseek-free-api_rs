use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;
pub type AppResult<T> = Result<T, AppError>; // 添加别名

// 添加AppError别名
pub use ApiError as AppError;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),
    
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Token error: {0}")]
    TokenError(String),
    
    #[error("Challenge calculation failed: {0}")]
    ChallengeError(String),
    
    #[error("DeepSeek API error: {code} - {message}")]
    DeepSeekApiError { code: u32, message: String },
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    // 新增错误类型
    #[error("External API error: {0}")]
    ExternalApi(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::HttpRequest(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ApiError::JsonError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::IoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::ConfigError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::TokenError(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::ChallengeError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::DeepSeekApiError { .. } => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::ServiceUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            ApiError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::Timeout(_) => (StatusCode::REQUEST_TIMEOUT, self.to_string()),
            ApiError::ExternalApi(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ApiError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "type": "api_error",
                "code": status.as_u16()
            }
        }));

        (status, body).into_response()
    }
}
