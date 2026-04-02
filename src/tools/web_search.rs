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
        "Search the web and return results. Uses Brave Search API if BRAVE_API_KEY is set, \
         otherwise falls back to DuckDuckGo (no key required)."
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

        // Try Brave if key is available, otherwise use DuckDuckGo
        let brave_key = std::env::var("BRAVE_API_KEY")
            .or_else(|_| std::env::var("ARCEE_SEARCH_API_KEY"))
            .ok();

        if let Some(api_key) = brave_key {
            search_brave(&search_query, &api_key, query).await
        } else {
            search_ddg(&search_query, query).await
        }
    }
}

/// Search using Brave Search API (requires API key).
async fn search_brave(search_query: &str, api_key: &str, display_query: &str) -> Result<ToolResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let response = client
        .get("https://api.search.brave.com/res/v1/web/search")
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", api_key)
        .query(&[("q", search_query), ("count", "10")])
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

    let results = body["web"]["results"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if results.is_empty() {
        return Ok(ToolResult::success(format!(
            "No results found for: {display_query}"
        )));
    }

    let mut output = format!("Search results for: {display_query}\n\n");
    for (i, result) in results.iter().enumerate() {
        let title = result["title"].as_str().unwrap_or("(no title)");
        let url = result["url"].as_str().unwrap_or("");
        let description = result["description"].as_str().unwrap_or("");
        output.push_str(&format!(
            "{}. **{}**\n   {}\n   {}\n\n",
            i + 1, title, url, description
        ));
    }

    Ok(ToolResult::success(output))
}

/// Search using DuckDuckGo HTML (no API key required).
async fn search_ddg(search_query: &str, display_query: &str) -> Result<ToolResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let response = client
        .get("https://html.duckduckgo.com/html/")
        .header("User-Agent", "Mozilla/5.0 (compatible; ArceeCode/1.0)")
        .query(&[("q", search_query)])
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
        return Ok(ToolResult::error(format!(
            "DuckDuckGo returned HTTP {status}"
        )));
    }

    let html = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            return Ok(ToolResult::error(format!(
                "Failed to read search response: {e}"
            )));
        }
    };

    // Parse DuckDuckGo HTML results
    let results = parse_ddg_html(&html);

    if results.is_empty() {
        return Ok(ToolResult::success(format!(
            "No results found for: {display_query}"
        )));
    }

    let mut output = format!("Search results for: {display_query}\n\n");
    for (i, (title, url, snippet)) in results.iter().enumerate().take(10) {
        output.push_str(&format!(
            "{}. **{}**\n   {}\n   {}\n\n",
            i + 1, title, url, snippet
        ));
    }

    Ok(ToolResult::success(output))
}

/// Parse DuckDuckGo HTML search results page.
fn parse_ddg_html(html: &str) -> Vec<(String, String, String)> {
    let mut results = Vec::new();

    // DuckDuckGo HTML results have class="result__a" for links and class="result__snippet" for snippets
    // We do simple string parsing to avoid adding an HTML parser dependency.
    for block in html.split("class=\"result__body") {
        if results.len() >= 10 {
            break;
        }

        // Extract title and URL from result__a link
        let (title, url) = if let Some(link_start) = block.find("class=\"result__a\"") {
            let after_link = &block[link_start..];

            // Get href
            let href = if let Some(href_start) = after_link.find("href=\"") {
                let href_content = &after_link[href_start + 6..];
                if let Some(href_end) = href_content.find('"') {
                    let raw_url = &href_content[..href_end];
                    // DDG wraps URLs; extract the actual URL from redirect
                    extract_ddg_url(raw_url)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // Get title text (between > and </a>)
            let title = if let Some(tag_end) = after_link.find('>') {
                let after_tag = &after_link[tag_end + 1..];
                if let Some(close) = after_tag.find("</a>") {
                    strip_html_tags(&after_tag[..close])
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            (title, href)
        } else {
            continue;
        };

        if url.is_empty() || title.is_empty() {
            continue;
        }

        // Extract snippet
        let snippet = if let Some(snip_start) = block.find("class=\"result__snippet\"") {
            let after_snip = &block[snip_start..];
            if let Some(tag_end) = after_snip.find('>') {
                let after_tag = &after_snip[tag_end + 1..];
                if let Some(close_pos) = after_tag.find("</") {
                    strip_html_tags(&after_tag[..close_pos])
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        results.push((title, url, snippet));
    }

    results
}

/// Extract actual URL from DuckDuckGo's redirect URL.
fn extract_ddg_url(raw: &str) -> String {
    // DDG URLs look like: //duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&...
    if let Some(uddg_start) = raw.find("uddg=") {
        let encoded = &raw[uddg_start + 5..];
        let encoded = if let Some(amp) = encoded.find('&') {
            &encoded[..amp]
        } else {
            encoded
        };
        url_decode(encoded)
    } else if raw.starts_with("http") {
        raw.to_string()
    } else if raw.starts_with("//") {
        format!("https:{raw}")
    } else {
        raw.to_string()
    }
}

/// Simple percent-decoding for URLs.
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().unwrap_or(b'0');
            let lo = chars.next().unwrap_or(b'0');
            let hex = [hi, lo];
            if let Ok(s) = std::str::from_utf8(&hex) {
                if let Ok(val) = u8::from_str_radix(s, 16) {
                    result.push(val as char);
                    continue;
                }
            }
            result.push('%');
            result.push(hi as char);
            result.push(lo as char);
        } else {
            result.push(b as char);
        }
    }
    result
}

/// Strip HTML tags from a string, decode basic entities.
fn strip_html_tags(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    // Decode common HTML entities
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}
