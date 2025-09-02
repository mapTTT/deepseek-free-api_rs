use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub environment: String,
    pub server: ServerConfig,
    pub deepseek: DeepSeekConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekConfig {
    pub base_url: String,
    pub wasm_path: String,
    pub max_retry_count: u32,
    pub retry_delay_ms: u64,
    pub access_token_expires: u64,
    pub authorization: Option<String>, // 环境变量中的token
}

impl Default for Config {
    fn default() -> Self {
        Self {
            environment: "development".to_string(),
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8000,
                cors_origins: vec!["*".to_string()],
            },
            deepseek: DeepSeekConfig {
                base_url: "https://chat.deepseek.com".to_string(),
                wasm_path: "./sha3_wasm_bg.7b9ca65ddd.wasm".to_string(),
                max_retry_count: 3,
                retry_delay_ms: 5000,
                access_token_expires: 3600,
                authorization: None,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut config = Config::default();
        
        // 从环境变量加载配置
        if let Ok(port) = env::var("PORT") {
            config.server.port = port.parse()?;
        }
        
        if let Ok(host) = env::var("HOST") {
            config.server.host = host;
        }
        
        if let Ok(env_type) = env::var("ENVIRONMENT") {
            config.environment = env_type;
        }
        
        // DeepSeek相关配置
        if let Ok(auth) = env::var("DEEP_SEEK_CHAT_AUTHORIZATION") {
            config.deepseek.authorization = Some(auth);
        }
        
        if let Ok(base_url) = env::var("DEEPSEEK_BASE_URL") {
            config.deepseek.base_url = base_url;
        }
        
        if let Ok(wasm_path) = env::var("WASM_PATH") {
            config.deepseek.wasm_path = wasm_path;
        }
        
        Ok(config)
    }
}
