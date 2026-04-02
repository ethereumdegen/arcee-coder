use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;

pub struct GrepTool;

const DEFAULT_HEAD_LIMIT: usize = 250;
const MAX_OUTPUT_CHARS: usize = 100_000;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> String {
        "Searches file contents using regex patterns. Supports filtering by glob pattern \
         or file type. Output modes: \"content\" shows matching lines, \"files_with_matches\" \
         shows only file paths (default), \"count\" shows match counts."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in (defaults to cwd)"
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g., \"*.rs\")"
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode (default: files_with_matches)"
                },
                "context": {
                    "type": "number",
                    "description": "Lines of context around matches (for content mode)"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive search"
                },
                "head_limit": {
                    "type": "number",
                    "description": "Max results to return (default: 250, 0=unlimited)"
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Enable multiline mode for cross-line patterns"
                }
            },
            "required": ["pattern"]
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let pattern = input["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?;

        if pattern.is_empty() {
            return Ok(ToolResult::error("Pattern cannot be empty"));
        }

        // Basic regex complexity check: reject obviously catastrophic patterns
        if pattern.len() > 1000 {
            return Ok(ToolResult::error("Pattern too long (max 1000 chars)"));
        }

        let search_path = match input["path"].as_str() {
            Some(p) => {
                let pb = PathBuf::from(p);
                if pb.is_absolute() {
                    pb
                } else {
                    context.cwd.join(pb)
                }
            }
            None => context.cwd.clone(),
        };

        let glob_filter = input["glob"].as_str().map(String::from);
        let output_mode = input["output_mode"]
            .as_str()
            .unwrap_or("files_with_matches");
        let context_lines = input["context"].as_u64().unwrap_or(0) as usize;
        let case_insensitive = input["case_insensitive"].as_bool().unwrap_or(false);
        let head_limit = input["head_limit"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_HEAD_LIMIT);
        let multiline = input["multiline"].as_bool().unwrap_or(false);

        // Try ripgrep first, fall back to system grep
        let have_rg = tokio::process::Command::new("rg")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .is_ok();

        let mut cmd;
        if have_rg {
            cmd = tokio::process::Command::new("rg");
            cmd.arg("--no-heading");
            cmd.arg("--max-columns").arg("500");
            cmd.arg("--max-columns-preview");

            match output_mode {
                "files_with_matches" => { cmd.arg("--files-with-matches"); }
                "count" => { cmd.arg("--count"); }
                "content" | _ => {
                    cmd.arg("--line-number");
                    if context_lines > 0 {
                        cmd.arg("-C").arg(context_lines.to_string());
                    }
                }
            }

            if case_insensitive { cmd.arg("-i"); }
            if multiline { cmd.arg("-U").arg("--multiline-dotall"); }
            if let Some(ref glob_pat) = glob_filter {
                cmd.arg("--glob").arg(glob_pat);
            }
            if pattern.starts_with('-') { cmd.arg("-e"); }
            cmd.arg(pattern).arg(&search_path);
        } else {
            // Fallback to system grep
            cmd = tokio::process::Command::new("grep");
            cmd.arg("-r"); // recursive

            match output_mode {
                "files_with_matches" => { cmd.arg("-l"); }
                "count" => { cmd.arg("-c"); }
                "content" | _ => {
                    cmd.arg("-n"); // line numbers
                    if context_lines > 0 {
                        cmd.arg("-C").arg(context_lines.to_string());
                    }
                }
            }

            if case_insensitive { cmd.arg("-i"); }
            if let Some(ref glob_pat) = glob_filter {
                cmd.arg("--include").arg(glob_pat);
            }
            if pattern.starts_with('-') { cmd.arg("-e"); }
            cmd.arg(pattern).arg(&search_path);
        }

        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            cmd.output(),
        )
        .await;

        let output = match output {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                return Ok(ToolResult::error(format!(
                    "Search command failed: {e}. Neither ripgrep (rg) nor grep are available."
                )));
            }
            Err(_) => {
                return Ok(ToolResult::error(
                    "Search timed out after 30 seconds. Try a more specific pattern or path.",
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.code() == Some(1) && stdout.is_empty() {
            return Ok(ToolResult::success(format!(
                "No matches found for pattern: {pattern}"
            )));
        }

        if !output.status.success() && output.status.code() != Some(1) {
            return Ok(ToolResult::error(format!(
                "Search failed: {}",
                if stderr.is_empty() {
                    "unknown error"
                } else {
                    stderr.trim()
                }
            )));
        }

        let result = stdout.trim().to_string();
        if result.is_empty() {
            return Ok(ToolResult::success(format!(
                "No matches found for pattern: {pattern}"
            )));
        }

        // Apply head_limit
        let mut lines: Vec<&str> = result.lines().collect();
        let total = lines.len();
        let was_limited = head_limit > 0 && total > head_limit;
        if was_limited {
            lines.truncate(head_limit);
        }
        let mut output_str = lines.join("\n");

        if was_limited {
            output_str.push_str(&format!(
                "\n\n... ({} results shown of {} total)",
                head_limit, total
            ));
        }

        // Final size guard
        if output_str.len() > MAX_OUTPUT_CHARS {
            let truncated =
                crate::tools::path_safety::safe_truncate(&output_str, MAX_OUTPUT_CHARS);
            output_str = format!(
                "{}\n\n... (output truncated, {} total bytes)",
                truncated,
                output_str.len()
            );
        }

        Ok(ToolResult::success(output_str))
    }
}
