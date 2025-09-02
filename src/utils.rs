use chrono::{DateTime, Utc};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// 生成Unix时间戳（秒）
pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// 生成Unix时间戳（毫秒）
pub fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// 生成随机字符串
pub fn generate_random_string(length: usize, charset: &str) -> String {
    let mut rng = thread_rng();
    match charset {
        "hex" => {
            let chars: Vec<char> = "0123456789abcdef".chars().collect();
            (0..length)
                .map(|_| chars[rng.gen_range(0..chars.len())])
                .collect()
        }
        "alphanumeric" => {
            rng.sample_iter(&Alphanumeric)
                .take(length)
                .map(char::from)
                .collect()
        }
        _ => {
            let chars: Vec<char> = charset.chars().collect();
            (0..length)
                .map(|_| chars[rng.gen_range(0..chars.len())])
                .collect()
        }
    }
}

/// 生成UUID
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// 生成UUID（不带连字符）
pub fn generate_uuid_simple() -> String {
    Uuid::new_v4().simple().to_string()
}

/// 生成Cookie字符串
pub fn generate_cookie() -> String {
    let timestamp = unix_timestamp_ms();
    let uuid1 = generate_uuid_simple();
    let uuid2 = generate_uuid_simple();
    let uuid3 = generate_uuid_simple();
    let uuid4 = generate_uuid_simple();
    let session_id = generate_random_string(18, "hex");
    let unix_ts = unix_timestamp();
    
    format!(
        "intercom-HWWAFSESTIME={}; HWWAFSESID={}; Hm_lvt_{}={},{},{}; Hm_lpvt_{}={}; _frid={}; _fr_ssid={}; _fr_pvid={}",
        timestamp,
        session_id,
        uuid1,
        unix_ts,
        unix_ts,
        unix_ts,
        uuid2,
        unix_ts,
        uuid3,
        uuid4,
        generate_uuid_simple()
    )
}

/// 分割Token字符串
pub fn split_tokens(authorization: &str) -> Vec<String> {
    let token_part = authorization.replace("Bearer ", "");
    token_part
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 随机选择Token
pub fn select_random_token(tokens: &[String]) -> Option<&String> {
    if tokens.is_empty() {
        return None;
    }
    let mut rng = thread_rng();
    let index = rng.gen_range(0..tokens.len());
    Some(&tokens[index])
}

/// 解析对话ID
pub fn parse_conversation_id(conv_id: &str) -> Option<(String, String)> {
    let regex = regex::Regex::new(r"^([0-9a-z\-]{36})@([0-9]+)$").unwrap();
    if let Some(captures) = regex.captures(conv_id) {
        let session_id = captures.get(1)?.as_str().to_string();
        let parent_msg_id = captures.get(2)?.as_str().to_string();
        Some((session_id, parent_msg_id))
    } else {
        None
    }
}

/// 检查模型类型
pub fn is_search_model(model: &str) -> bool {
    model.contains("search")
}

pub fn is_thinking_model(model: &str) -> bool {
    model.contains("think") || model.contains("r1")
}

pub fn is_silent_model(model: &str) -> bool {
    model.contains("silent")
}

pub fn is_fold_model(model: &str) -> bool {
    model.contains("fold")
}

/// 格式化时间
pub fn format_timestamp(timestamp: u64) -> String {
    let datetime = DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_else(|| Utc::now());
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_string() {
        let hex_str = generate_random_string(16, "hex");
        assert_eq!(hex_str.len(), 16);
        assert!(hex_str.chars().all(|c| "0123456789abcdef".contains(c)));
    }

    #[test]
    fn test_split_tokens() {
        let auth = "Bearer token1,token2,token3";
        let tokens = split_tokens(auth);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], "token1");
        assert_eq!(tokens[1], "token2");
        assert_eq!(tokens[2], "token3");
    }

    #[test]
    fn test_parse_conversation_id() {
        let conv_id = "12345678-1234-1234-1234-123456789012@123";
        let (session_id, parent_id) = parse_conversation_id(conv_id).unwrap();
        assert_eq!(session_id, "12345678-1234-1234-1234-123456789012");
        assert_eq!(parent_id, "123");
    }

    #[test]
    fn test_model_checks() {
        assert!(is_search_model("deepseek-search"));
        assert!(is_thinking_model("deepseek-think"));
        assert!(is_thinking_model("deepseek-r1"));
        assert!(is_silent_model("deepseek-think-silent"));
        assert!(is_fold_model("deepseek-think-fold"));
    }
}
