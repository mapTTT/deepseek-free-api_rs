use crate::models::{ChatMessage, ChatMessageContent};
use crate::utils::{is_fold_model, is_search_model, is_silent_model, is_thinking_model};
use regex::Regex;

/// 消息处理器
pub struct MessageProcessor;

impl MessageProcessor {
    /// 预处理聊天消息
    pub fn prepare_messages(messages: &[ChatMessage]) -> String {
        if messages.is_empty() {
            return String::new();
        }

        // 处理消息内容
        let processed_messages: Vec<ProcessedMessage> = messages
            .iter()
            .map(|message| {
                let text = Self::extract_text_content(&message.content);
                ProcessedMessage {
                    role: message.role.clone(),
                    text,
                }
            })
            .collect();

        // 合并连续相同角色的消息
        let merged_blocks = Self::merge_same_role_messages(processed_messages);

        // 添加标签并连接结果
        Self::format_messages_with_tags(&merged_blocks)
    }

    /// 从内容中提取文本
    fn extract_text_content(content: &ChatMessageContent) -> String {
        match content {
            ChatMessageContent::Text(text) => text.clone(),
            ChatMessageContent::Array(parts) => {
                parts
                    .iter()
                    .filter_map(|part| {
                        if part.content_type == "text" {
                            part.text.as_ref()
                        } else {
                            None
                        }
                    })
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }

    /// 合并连续相同角色的消息
    fn merge_same_role_messages(messages: Vec<ProcessedMessage>) -> Vec<ProcessedMessage> {
        if messages.is_empty() {
            return vec![];
        }

        let mut merged_blocks = Vec::new();
        let mut current_block = messages[0].clone();

        for message in messages.into_iter().skip(1) {
            if message.role == current_block.role {
                current_block.text = format!("{}\n\n{}", current_block.text, message.text);
            } else {
                merged_blocks.push(current_block);
                current_block = message;
            }
        }
        merged_blocks.push(current_block);

        merged_blocks
    }

    /// 使用标签格式化消息
    fn format_messages_with_tags(blocks: &[ProcessedMessage]) -> String {
        blocks
            .iter()
            .enumerate()
            .map(|(index, block)| {
                match block.role.as_str() {
                    "assistant" => {
                        format!("<｜Assistant｜>{}<｜end▁of▁sentence｜>", block.text)
                    }
                    "user" | "system" => {
                        if index > 0 {
                            format!("<｜User｜>{}", block.text)
                        } else {
                            block.text.clone()
                        }
                    }
                    _ => block.text.clone(),
                }
            })
            .collect::<Vec<_>>()
            .join("")
            .replace("![.*]\\(.*\\)", "") // 移除图片链接
    }

    /// 处理流式响应内容
    pub fn process_stream_content(
        content: &str,
        model: &str,
        thinking_active: &mut bool,
        ref_content: &mut String,
    ) -> Option<String> {
        let is_thinking = is_thinking_model(model);
        let is_search = is_search_model(model);
        let is_silent = is_silent_model(model);
        let is_fold = is_fold_model(model);

        // 处理搜索结果
        if is_search && !is_silent {
            // 搜索结果处理逻辑
            if content.contains("检索") {
                ref_content.push_str(content);
                ref_content.push('\n');
                return Some(content.to_string());
            }
        }

        // 处理思考模式
        if is_thinking {
            if is_fold {
                // 折叠模式的思考处理
                if !*thinking_active && content.contains("[思考") {
                    *thinking_active = true;
                    return Some("<details><summary>思考过程</summary><pre>".to_string());
                } else if *thinking_active && content.contains("[思考结束]") {
                    *thinking_active = false;
                    return Some("</pre></details>".to_string());
                }
            } else if is_silent {
                // 静默模式，不输出思考内容
                if content.contains("[思考") || content.contains("思考过程") {
                    return None;
                }
            } else {
                // 普通思考模式
                if !*thinking_active && content.contains("[思考") {
                    *thinking_active = true;
                    return Some("[思考开始]\n".to_string());
                } else if *thinking_active && content.contains("[思考结束]") {
                    *thinking_active = false;
                    return Some("\n\n[思考结束]\n".to_string());
                }
            }
        }

        // 移除引用标记
        let cleaned_content = Self::remove_citations(content);

        Some(cleaned_content)
    }

    /// 移除引用标记
    fn remove_citations(content: &str) -> String {
        let citation_regex = Regex::new(r"\[citation:\d+\]").unwrap();
        citation_regex.replace_all(content, "").to_string()
    }

    /// 添加搜索结果引用
    pub fn add_search_references(content: &str, ref_content: &str) -> String {
        if ref_content.is_empty() {
            content.to_string()
        } else {
            let trimmed_content = content.trim_start_matches('\n');
            let cleaned_ref = Self::remove_citations(ref_content);
            format!("{}\n\n搜索结果来自：\n{}", trimmed_content, cleaned_ref)
        }
    }
}

#[derive(Debug, Clone)]
struct ProcessedMessage {
    role: String,
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ContentPart;

    #[test]
    fn test_extract_text_content() {
        let text_content = ChatMessageContent::Text("Hello world".to_string());
        assert_eq!(
            MessageProcessor::extract_text_content(&text_content),
            "Hello world"
        );

        let array_content = ChatMessageContent::Array(vec![
            ContentPart {
                content_type: "text".to_string(),
                text: Some("Hello".to_string()),
                image_url: None,
            },
            ContentPart {
                content_type: "text".to_string(),
                text: Some("World".to_string()),
                image_url: None,
            },
        ]);
        assert_eq!(
            MessageProcessor::extract_text_content(&array_content),
            "Hello\nWorld"
        );
    }

    #[test]
    fn test_remove_citations() {
        let content = "This is a test [citation:1] with citations [citation:23].";
        let cleaned = MessageProcessor::remove_citations(content);
        assert_eq!(cleaned, "This is a test  with citations .");
    }

    #[test]
    fn test_prepare_messages() {
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: ChatMessageContent::Text("Hello".to_string()),
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: ChatMessageContent::Text("Hi there!".to_string()),
            },
        ];

        let result = MessageProcessor::prepare_messages(&messages);
        assert!(result.contains("Hello"));
        assert!(result.contains("<｜Assistant｜>Hi there!<｜end▁of▁sentence｜>"));
    }
}
