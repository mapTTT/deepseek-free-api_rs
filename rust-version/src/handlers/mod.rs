pub mod chat;
pub mod health;
pub mod token;
pub mod api_keys;

use crate::config::Config;
use crate::error::ApiResult;
use crate::services::{DeepSeekClient, ApiKeyManager, LoginService};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<DeepSeekClient>,
    pub config: Config,
    pub api_key_manager: Arc<ApiKeyManager>,
    pub login_service: Arc<LoginService>,
}

pub async fn create_router(config: Config) -> ApiResult<Router> {
    let client = Arc::new(DeepSeekClient::new(config.clone()));
    let api_key_manager = Arc::new(ApiKeyManager::new());
    let login_service = Arc::new(LoginService::new());
    
    let state = AppState {
        client,
        config: config.clone(),
        api_key_manager,
        login_service,
    };

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        // 健康检查
        .route("/", get(health::root))
        .route("/ping", get(health::ping))
        
        // 聊天API - OpenAI兼容
        .route("/v1/chat/completions", post(chat::completions))
        
        // Token检查
        .route("/token/check", post(token::check))
        
        // 模型列表 - OpenAI兼容
        .route("/v1/models", get(chat::models))
        
        // API密钥管理
        .route("/api_keys/create", post(api_keys::create_api_key))
        .route("/api_keys/add_account", post(api_keys::add_account))
        .route("/api_keys/info", post(api_keys::get_api_key_info))
        .route("/api_keys/list", get(api_keys::list_api_keys))
        .route("/api_keys/deactivate", post(api_keys::deactivate_api_key))
        .route("/api_keys/cleanup", post(api_keys::cleanup_expired_keys))
        
        // 登录和Token验证（调试用）
        .route("/auth/login", post(api_keys::login_for_token))
        .route("/auth/verify", post(api_keys::verify_user_token))
        
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        )
        .with_state(state);

    Ok(app)
}
