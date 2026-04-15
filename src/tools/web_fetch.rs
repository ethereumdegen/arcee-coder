use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput, Truncation};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct WebFetchTool;

const DEFAULT_MAX_CHARS: usize = 50_000;

const DESCRIPTION: &str = "IMPORTANT: WebFetch WILL FAIL for authenticated or private URLs. Before using this tool, check if the URL points to an \
authenticated service (e.g. Google Docs, Confluence, Jira, GitHub). If so, you MUST use a specialized tool that provides authenticated access.\n\n\
- Fetches content from a specified URL and processes it using an AI model\n\
- Takes a URL and a prompt as input\n\
- Fetches the URL content, converts HTML to markdown\n\
- Processes the content with the prompt using a small, fast model\n\
- Returns the model's response about the content\n\
- Use this tool when you need to retrieve and analyze web content\n\n\
Usage notes:\n\
  - The URL must be a fully-formed valid URL\n\
  - HTTP URLs will be automatically upgraded to HTTPS\n\
  - The prompt should describe what information you want to extract from the page\n\
  - This tool is read-only and does not modify any files\n\
  - Results may be summarized if the content is very large\n\
  - Includes a self-cleaning 15-minute cache for faster responses when repeatedly accessing the same URL\n\
  - When a URL redirects to a different host, the tool will inform you and provide the redirect URL in a special format.\n\
  - For GitHub URLs, prefer using the gh CLI via Bash instead (e.g., gh pr view, gh issue view, gh api).";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
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
                },
                "full": {
                    "type": "boolean",
                    "description": "Disable the 50k char truncation cap and return the full page"
                }
            },
            "required": ["url", "prompt"]
        })
    })
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "WebFetch"
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        schema()
    }

    fn permission(&self, _input: &serde_json::Value) -> PermissionClass {
        PermissionClass::ReadOnly
    }

    async fn call(&self, input: serde_json::Value, _context: &ToolContext) -> Result<ToolOutput> {
        let url = input["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;
        let full = input["full"].as_bool().unwrap_or(false);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("arcee-code/0.1")
            .build()?;

        let response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolOutput::error(format!("Failed to fetch {url}: {e}")));
            }
        };

        let status = response.status();
        if !status.is_success() {
            return Ok(ToolOutput::error(format!("HTTP {status} fetching {url}")));
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
                return Ok(ToolOutput::error(format!(
                    "Failed to read response body: {e}"
                )));
            }
        };

        let text = if content_type.contains("html") {
            html2text::from_read(body.as_bytes(), 120)
        } else {
            body
        };

        let total_chars = text.chars().count();
        let domain = url
            .split("://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or(url);
        let summary = format!("{domain} — {total_chars} chars");

        let (shown_text, truncated) = if !full && text.len() > DEFAULT_MAX_CHARS {
            let truncated_str = crate::tools::path_safety::safe_truncate(&text, DEFAULT_MAX_CHARS);
            (truncated_str.to_string(), true)
        } else {
            (text, false)
        };

        let mut out = ToolOutput::success()
            .with_summary(summary)
            .with_text(shown_text);

        if truncated {
            out = out
                .with_truncation(Truncation {
                    shown: DEFAULT_MAX_CHARS,
                    total: total_chars,
                    unit: "chars",
                    how_to_see_more: "pass full=true to return the entire page".into(),
                })
                .with_next_step("Pass full=true to return the entire page");
        }

        Ok(out)
    }
}
