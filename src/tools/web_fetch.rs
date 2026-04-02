use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn description(&self) -> String {
        "Fetches content from a URL and returns it as text. HTML is converted to \
         readable text. Use this to retrieve web content for analysis."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                },
                "prompt": {
                    "type": "string",
                    "description": "What to extract from the page content"
                }
            },
            "required": ["url", "prompt"]
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, _context: &ToolContext) -> Result<ToolResult> {
        let url = input["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("arcee-code/0.1")
            .build()?;

        let response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolResult::error(format!("Failed to fetch {url}: {e}")));
            }
        };

        let status = response.status();
        if !status.is_success() {
            return Ok(ToolResult::error(format!(
                "HTTP {status} fetching {url}"
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to read response body: {e}"
                )));
            }
        };

        // Convert HTML to text if needed
        let text = if content_type.contains("html") {
            html2text::from_read(body.as_bytes(), 120)
        } else {
            body
        };

        // Truncate if very long
        let max_len = 50_000;
        let output = if text.len() > max_len {
            format!(
                "{}\n\n... (truncated, {} total chars)",
                &text[..max_len],
                text.len()
            )
        } else {
            text
        };

        Ok(ToolResult::success(output))
    }
}
