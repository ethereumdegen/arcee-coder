use crate::toon::ToonValue;
use crate::tools::{PermissionClass, Tool, ToolBody, ToolContext, ToolOutput, Truncation};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct WebSearchTool;

const DEFAULT_LIMIT: usize = 10;
const FULL_LIMIT: usize = 25;

const DESCRIPTION: &str = "\
- Allows Claude to search the web and use the results to inform responses\n\
- Provides up-to-date information for current events and recent data\n\
- Returns search result information formatted as search result blocks, including links as markdown hyperlinks\n\
- Use this tool for accessing information beyond Claude's knowledge cutoff\n\
- Searches are performed automatically within a single API call\n\n\
CRITICAL REQUIREMENT - You MUST follow this:\n\
  - After answering the user's question, you MUST include a \"Sources:\" section at the end of your response\n\
  - In the Sources section, list all relevant URLs from the search results as markdown hyperlinks: [Title](URL)\n\
  - This is MANDATORY - never skip including sources in your response\n\n\
Usage notes:\n\
  - Domain filtering is supported to include or block specific websites\n\
  - Uses Brave Search API when BRAVE_API_KEY / ARCEE_SEARCH_API_KEY is set, otherwise falls back to DuckDuckGo (no key required)";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
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
                },
                "full": {
                    "type": "boolean",
                    "description": "Return up to 25 results instead of the default 10"
                }
            },
            "required": ["query"]
        })
    })
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "WebSearch"
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
        let query = input["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

        if query.len() < 2 {
            return Ok(ToolOutput::error("Query must be at least 2 characters long"));
        }

        let full = input["full"].as_bool().unwrap_or(false);
        let limit = if full { FULL_LIMIT } else { DEFAULT_LIMIT };

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

        let mut search_query = query.to_string();
        for domain in &allowed_domains {
            search_query.push_str(&format!(" site:{domain}"));
        }
        for domain in &blocked_domains {
            search_query.push_str(&format!(" -site:{domain}"));
        }

        let brave_key = std::env::var("BRAVE_API_KEY")
            .or_else(|_| std::env::var("ARCEE_SEARCH_API_KEY"))
            .ok();

        let raw_results: Result<Vec<SearchResult>> = if let Some(api_key) = brave_key {
            search_brave(&search_query, &api_key, limit).await
        } else {
            search_ddg(&search_query, limit).await
        };

        let results = match raw_results {
            Ok(r) => r,
            Err(e) => return Ok(ToolOutput::error(format!("Search failed: {e}"))),
        };

        if results.is_empty() {
            return Ok(ToolOutput::empty(format!("No results found for: {query}"))
                .with_summary(format!("0 results for {query:?}"))
                .with_next_step("Broaden the query or remove domain filters"));
        }

        let total = results.len();
        let shown_count = total.min(limit);
        let rows: Vec<Vec<String>> = results
            .iter()
            .take(shown_count)
            .map(|r| {
                vec![
                    r.title.clone(),
                    r.url.clone(),
                    truncate_snippet(&r.snippet, 200),
                ]
            })
            .collect();

        let body = ToolBody::Toon(ToonValue::Map(vec![(
            "results".into(),
            ToonValue::Table {
                columns: vec!["title".into(), "url".into(), "snippet".into()],
                rows,
            },
        )]));

        let summary = format!("{shown_count} result(s) for {query:?}");
        let mut out = ToolOutput::success().with_summary(summary).with_body(body);

        if !full && total > DEFAULT_LIMIT {
            out = out
                .with_truncation(Truncation {
                    shown: shown_count,
                    total,
                    unit: "results",
                    how_to_see_more: "pass full=true for up to 25 results".into(),
                })
                .with_next_step("Pass full=true for up to 25 results");
        }

        out = out.with_next_step("Refine with site:<domain> or -site:<domain> to narrow further");

        Ok(out)
    }
}

struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn truncate_snippet(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.replace('\n', " ").trim().to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{}…", truncated.replace('\n', " ").trim())
    }
}

async fn search_brave(
    search_query: &str,
    api_key: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let response = client
        .get("https://api.search.brave.com/res/v1/web/search")
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", api_key)
        .query(&[
            ("q", search_query.to_string()),
            ("count", limit.to_string()),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Brave API HTTP {status}: {body}");
    }

    let body: serde_json::Value = response.json().await?;
    let items = body["web"]["results"].as_array().cloned().unwrap_or_default();

    Ok(items
        .into_iter()
        .map(|r| SearchResult {
            title: r["title"].as_str().unwrap_or("(no title)").to_string(),
            url: r["url"].as_str().unwrap_or("").to_string(),
            snippet: r["description"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

async fn search_ddg(search_query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let response = client
        .get("https://html.duckduckgo.com/html/")
        .header("User-Agent", "Mozilla/5.0 (compatible; ArceeCode/1.0)")
        .query(&[("q", search_query)])
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("DuckDuckGo HTTP {}", response.status());
    }

    let html = response.text().await?;
    let results = parse_ddg_html(&html, limit);
    Ok(results
        .into_iter()
        .map(|(title, url, snippet)| SearchResult {
            title,
            url,
            snippet,
        })
        .collect())
}

fn parse_ddg_html(html: &str, limit: usize) -> Vec<(String, String, String)> {
    let mut results = Vec::new();

    for block in html.split("class=\"result__body") {
        if results.len() >= limit {
            break;
        }

        let (title, url) = if let Some(link_start) = block.find("class=\"result__a\"") {
            let after_link = &block[link_start..];
            let href = if let Some(href_start) = after_link.find("href=\"") {
                let href_content = &after_link[href_start + 6..];
                if let Some(href_end) = href_content.find('"') {
                    extract_ddg_url(&href_content[..href_end])
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
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

fn extract_ddg_url(raw: &str) -> String {
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
