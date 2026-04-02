use crate::api::types::{ContentBlock, StopReason, Usage};
use serde::{Deserialize, Serialize};

/// Internal message representation used throughout the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: Vec<UserContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserContent {
    Text(String),
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub content: String,
    pub message_type: SystemMessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemMessageType {
    CompactBoundary,
    Info,
    Error,
}

impl Message {
    pub fn user_text(text: impl Into<String>) -> Self {
        Message::User(UserMessage {
            content: vec![UserContent::Text(text.into())],
        })
    }

    pub fn tool_results(results: Vec<(String, String, bool)>) -> Self {
        Message::User(UserMessage {
            content: results
                .into_iter()
                .map(|(id, content, is_error)| UserContent::ToolResult {
                    tool_use_id: id,
                    content,
                    is_error,
                })
                .collect(),
        })
    }
}
