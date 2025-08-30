use anyhow::Result;
use colored::*;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod handlers;
mod models;
mod services;
mod utils;

use config::Config;
use handlers::create_router;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    init_logging()?;
    
    // 加载配置
    dotenv::dotenv().ok();
    let config = Config::load()?;
    
    println!("{}", "DeepSeek Free API Server (Rust Version)".bright_green().bold());
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Environment: {}", config.environment);
    println!("Server binding to: {}:{}", config.server.host, config.server.port);
    
    // 创建路由
    let app = create_router(config.clone()).await?;
    
    // 启动服务器
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    println!("{}", format!("Server started on http://{}", addr).bright_green().bold());
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

fn init_logging() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "deepseek_free_api=debug,tower_http=debug".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}
