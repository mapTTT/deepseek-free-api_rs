use axum::{http::StatusCode, response::Json};
use serde_json::{json, Value};

/// 根路径处理器
pub async fn root() -> Json<Value> {
    Json(json!({
        "message": "DeepSeek Free API Server (Rust Version)",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "healthy"
    }))
}

/// 健康检查
pub async fn ping() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "message": "pong",
            "timestamp": chrono::Utc::now().timestamp(),
            "status": "healthy"
        }))
    )
}
