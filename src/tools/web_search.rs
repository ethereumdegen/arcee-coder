use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "WebSearch"
    }

    fn description(&self) -> String {
        "Search the web and return results. Requires BRAVE_API_KEY or ARCEE_SEARCH_API_KEY \
         environment variable. Returns titles, URLs, and snippets."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query (minimum 2 characters)"
                },
                "allowed_domains": {
                    "type": "array",
                    "description": "Only include results from these domains",
                    "items": { "type": "string" }
                },
                "blocked_domains": {
                    "type": "array",
                    "description": "Exclude results from these domains",
                    "items": { "type": "string" }
                }
            },
            "required": ["query"]
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, _context: &ToolContext) -> Result<ToolResult> {
        let query = input["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

        if query.len() < 2 {
            return Ok(ToolResult::error(
                "Query must be at least 2 characters long",
            ));
        }

        let api_key = std::env::var("BRAVE_API_KEY")
            .or_else(|_| std::env::var("ARCEE_SEARCH_API_KEY"))
            .ok();

        let Some(api_key) = api_key else {
            return Ok(ToolResult::error(
                "No search API key found. Set BRAVE_API_KEY or ARCEE_SEARCH_API_KEY environment variable.\n\
                 Get a free API key at https://brave.com/search/api/"
            ));
        };

        let allowed_domains: Vec<String> = input["allowed_domains"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let blocked_domains: Vec<String> = input["blocked_domains"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Build search query with domain filters
        let mut search_query = query.to_string();
        for domain in &allowed_domains {
            search_query.push_str(&format!(" site:{domain}"));
        }
        for domain in &blocked_domains {
            search_query.push_str(&format!(" -site:{domain}"));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        let response = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", &api_key)
            .query(&[("q", &search_query), ("count", &"10".to_string())])
            .send()
            .await;

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolResult::error(format!("Search request failed: {e}")));
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Ok(ToolResult::error(format!(
                "Search API returned HTTP {status}: {body}"
            )));
        }

        let body: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to parse search response: {e}"
                )));
            }
        };

        // Parse Brave Search API response
        let results = body["web"]["results"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        if results.is_empty() {
            return Ok(ToolResult::success(format!(
                "No results found for: {query}"
            )));
        }

        let mut output = format!("Search results for: {query}\n\n");

        for (i, result) in results.iter().enumerate() {
            let title = result["title"].as_str().unwrap_or("(no title)");
            let url = result["url"].as_str().unwrap_or("");
            let description = result["description"].as_str().unwrap_or("");

            output.push_str(&format!(
                "{}. **{}**\n   {}\n   {}\n\n",
                i + 1,
                title,
                url,
                description
            ));
        }

        Ok(ToolResult::success(output))
    }
}
