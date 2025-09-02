use crate::error::ApiError;
use crate::handlers::AppState;
use crate::models::{TokenCheckRequest, TokenCheckResponse};
use axum::{extract::State, response::Json};

/// 检查token状态
pub async fn check(
    State(state): State<AppState>,
    Json(request): Json<TokenCheckRequest>,
) -> Result<Json<TokenCheckResponse>, ApiError> {
    tracing::info!("Checking token status");

    let live = state.client.check_token_status(&request.token).await?;

    Ok(Json(TokenCheckResponse { live }))
}
