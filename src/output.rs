use serde::{Deserialize, Serialize};

/// Output format for print mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Plain text output (default, same as current oneshot behavior).
    #[default]
    Text,
    /// Single JSON object at the end with the complete result.
    Json,
    /// Newline-delimited JSON events streamed as they happen.
    StreamJson,
}

impl OutputFormat {
    pub fn from_str_opt(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => Self::Json,
            "stream-json" | "stream_json" | "streamjson" => Self::StreamJson,
            _ => Self::Text,
        }
    }
}

/// A streaming JSON event emitted in StreamJson mode.
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl StreamEvent {
    pub fn text(content: &str) -> Self {
        Self {
            event_type: "text".to_string(),
            content: Some(content.to_string()),
            name: None,
            input: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            is_error: None,
        }
    }

    pub fn tool_use(name: &str, input: &serde_json::Value) -> Self {
        Self {
            event_type: "tool_use".to_string(),
            content: None,
            name: Some(name.to_string()),
            input: Some(input.clone()),
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            is_error: None,
        }
    }

    pub fn tool_result(name: &str, content: &str, is_error: bool) -> Self {
        Self {
            event_type: "tool_result".to_string(),
            content: Some(content.to_string()),
            name: Some(name.to_string()),
            input: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            is_error: Some(is_error),
        }
    }

    pub fn done(cost_usd: f64, input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            event_type: "done".to_string(),
            content: None,
            name: None,
            input: None,
            cost_usd: Some(cost_usd),
            input_tokens: Some(input_tokens),
            output_tokens: Some(output_tokens),
            is_error: None,
        }
    }

    pub fn emit(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            println!("{json}");
        }
    }
}
