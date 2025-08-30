use serde::{Deserialize, Serialize};

// OpenAI兼容的聊天请求结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    pub conversation_id: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatMessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatMessageContent {
    Text(String),
    Array(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub image_url: Option<ImageUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    pub detail: Option<String>,
}

// OpenAI兼容的响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<ChatUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: Option<ChatMessage>,
    pub delta: Option<ChatMessageDelta>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// DeepSeek API相关结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekResponse<T> {
    pub code: Option<u32>,
    pub data: Option<T>,
    pub biz_data: Option<T>,
    pub msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub token: String,
    pub id: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub character_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeRequest {
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge: Challenge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub algorithm: String,
    pub challenge: String,
    pub salt: String,
    pub difficulty: u32,
    pub expire_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeAnswer {
    pub algorithm: String,
    pub challenge: String,
    pub salt: String,
    pub answer: String,
    pub signature: String,
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub chat_session_id: String,
    pub parent_message_id: Option<String>,
    pub prompt: String,
    pub ref_file_ids: Vec<String>,
    pub search_enabled: bool,
    pub thinking_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingQuota {
    pub quota: u32,
    pub used: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureQuota {
    pub thinking: ThinkingQuota,
}

// Token状态检查
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCheckRequest {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCheckResponse {
    pub live: bool,
}

// 登录相关
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user_token: String,
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekLoginRequest {
    pub email: String,
    pub password: String,
    pub captcha_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekLoginResponse {
    pub code: Option<u32>,
    pub data: Option<LoginData>,
    pub msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginData {
    pub token: String,
    pub user: Option<UserProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

// API密钥管理
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub key: String,
    pub name: String,
    pub user_tokens: Vec<String>, // 关联的DeepSeek userToken列表
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub usage_count: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_days: Option<u32>, // 过期天数，None表示永不过期
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub api_key: String,
    pub name: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAccountRequest {
    pub api_key: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAccountResponse {
    pub success: bool,
    pub message: String,
    pub accounts_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub accounts_count: usize,
    pub usage_count: u64,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub is_active: bool,
}

// 流式响应数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: ChatMessageDelta,
    pub finish_reason: Option<String>,
}

// DeepSeek 流式响应解析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekStreamData {
    pub message_id: Option<String>,
    pub choices: Option<Vec<DeepSeekChoice>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekChoice {
    pub delta: DeepSeekDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekDelta {
    #[serde(rename = "type")]
    pub delta_type: Option<String>,
    pub content: Option<String>,
    pub search_results: Option<Vec<SearchResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
}

impl Default for ChatCompletionRequest {
    fn default() -> Self {
        Self {
            model: Some("deepseek".to_string()),
            messages: vec![],
            stream: Some(false),
            conversation_id: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
        }
    }
}
