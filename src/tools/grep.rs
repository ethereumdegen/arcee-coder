use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;

pub struct GrepTool;

const DEFAULT_HEAD_LIMIT: usize = 250;
const MAX_OUTPUT_CHARS: usize = 20_000;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> String {
        "A powerful search tool built on ripgrep.\n\n\
         REQUIRED parameter: \"pattern\" (string) — the regex pattern to search for.\n\
         Example call: {\"pattern\": \"fn main\", \"glob\": \"*.rs\"}\n\n\
         Usage:\n\
         - ALWAYS use Grep for search tasks. NEVER invoke `grep` or `rg` as a Bash command.\n\
         - Supports full regex syntax (e.g., \"log.*Error\", \"function\\s+\\w+\")\n\
         - Filter files with glob parameter (e.g., \"*.js\", \"**/*.tsx\") or type parameter \
         (e.g., \"js\", \"py\", \"rust\")\n\
         - Output modes: \"content\" shows matching lines, \"files_with_matches\" shows only \
         file paths (default), \"count\" shows match counts\n\
         - Pattern syntax: Uses ripgrep (not grep) - literal braces need escaping \
         (use `interface\\{\\}` to find `interface{}` in Go code)\n\
         - Multiline matching: By default patterns match within single lines only. \
         For cross-line patterns like `struct \\{[\\s\\S]*?field`, use `multiline: true`"
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regular expression pattern to search for in file contents"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in (rg PATH). Defaults to current working directory."
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. \"*.js\", \"*.{ts,tsx}\") - maps to rg --glob"
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode: \"content\" shows matching lines (supports -A/-B/-C context, -n line numbers, head_limit), \"files_with_matches\" shows file paths (supports head_limit), \"count\" shows match counts (supports head_limit). Defaults to \"files_with_matches\"."
                },
                "-B": {
                    "type": "number",
                    "description": "Number of lines to show before each match (rg -B). Requires output_mode: \"content\", ignored otherwise."
                },
                "-A": {
                    "type": "number",
                    "description": "Number of lines to show after each match (rg -A). Requires output_mode: \"content\", ignored otherwise."
                },
                "-C": {
                    "type": "number",
                    "description": "Alias for context."
                },
                "context": {
                    "type": "number",
                    "description": "Number of lines to show before and after each match (rg -C). Requires output_mode: \"content\", ignored otherwise."
                },
                "-n": {
                    "type": "boolean",
                    "description": "Show line numbers in output (rg -n). Requires output_mode: \"content\", ignored otherwise. Defaults to true."
                },
                "-i": {
                    "type": "boolean",
                    "description": "Case insensitive search (rg -i)"
                },
                "type": {
                    "type": "string",
                    "description": "File type to search (rg --type). Common types: js, py, rust, go, java, etc. More efficient than include for standard file types."
                },
                "head_limit": {
                    "type": "number",
                    "description": "Limit output to first N lines/entries, equivalent to \"| head -N\". Works across all output modes: content (limits output lines), files_with_matches (limits file paths), count (limits count entries). Defaults to 250 when unspecified. Pass 0 for unlimited."
                },
                "offset": {
                    "type": "number",
                    "description": "Skip first N lines/entries before applying head_limit, equivalent to \"| tail -n +N | head -N\". Works across all output modes. Defaults to 0."
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Enable multiline mode where . matches newlines and patterns can span lines (rg -U --multiline-dotall). Default: false."
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
        let type_filter = input["type"].as_str().map(String::from);
        let output_mode = input["output_mode"]
            .as_str()
            .unwrap_or("files_with_matches");

        // Context lines: support -C, -B, -A aliases
        let context_lines = input["context"].as_u64()
            .or_else(|| input["-C"].as_u64())
            .unwrap_or(0) as usize;
        let before_context = input["-B"].as_u64().unwrap_or(0) as usize;
        let after_context = input["-A"].as_u64().unwrap_or(0) as usize;

        let case_insensitive = input["-i"].as_bool()
            .or_else(|| input["case_insensitive"].as_bool())
            .unwrap_or(false);
        let show_line_numbers = input["-n"].as_bool().unwrap_or(true);
        let head_limit = input["head_limit"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_HEAD_LIMIT);
        let offset = input["offset"].as_u64().unwrap_or(0) as usize;
        let multiline = input["multiline"].as_bool().unwrap_or(false);

        let mut cmd = tokio::process::Command::new("rg");
        cmd.arg("--no-heading");
        cmd.arg("--max-columns").arg("500");
        cmd.arg("--max-columns-preview");

        match output_mode {
            "files_with_matches" => {
                cmd.arg("--files-with-matches");
                cmd.arg("--sort=modified");
            }
            "count" => {
                cmd.arg("--count");
            }
            "content" | _ => {
                if show_line_numbers {
                    cmd.arg("--line-number");
                }
                if context_lines > 0 {
                    cmd.arg("-C").arg(context_lines.to_string());
                } else {
                    if before_context > 0 {
                        cmd.arg("-B").arg(before_context.to_string());
                    }
                    if after_context > 0 {
                        cmd.arg("-A").arg(after_context.to_string());
                    }
                }
            }
        }

        if case_insensitive {
            cmd.arg("-i");
        }
        if multiline {
            cmd.arg("-U").arg("--multiline-dotall");
        }
        if let Some(ref glob_pat) = glob_filter {
            cmd.arg("--glob").arg(glob_pat);
        }
        if let Some(ref type_name) = type_filter {
            cmd.arg("--type").arg(type_name);
        }
        if pattern.starts_with('-') {
            cmd.arg("-e");
        }
        cmd.arg(pattern).arg(&search_path);

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
                    "Search command failed: {e}. Is ripgrep (rg) installed?"
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

        // Relativize paths to save tokens
        let result = relativize_paths(&result, &context.cwd);

        // Apply offset + head_limit
        let mut lines: Vec<&str> = result.lines().collect();
        let total = lines.len();

        // Apply offset first
        if offset > 0 && offset < lines.len() {
            lines = lines[offset..].to_vec();
        } else if offset >= lines.len() {
            return Ok(ToolResult::success(format!(
                "No results at offset {offset} (total: {total})"
            )));
        }

        let was_limited = head_limit > 0 && lines.len() > head_limit;
        if was_limited {
            lines.truncate(head_limit);
        }
        let mut output_str = lines.join("\n");

        if was_limited || offset > 0 {
            let showing = lines.len();
            output_str.push_str(&format!(
                "\n\n({showing} results shown, {total} total{})",
                if offset > 0 { format!(", offset {offset}") } else { String::new() }
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

/// Convert absolute paths to relative paths to save tokens.
fn relativize_paths(text: &str, cwd: &std::path::Path) -> String {
    let cwd_str = format!("{}/", cwd.display());
    text.replace(&cwd_str, "")
}
