use crate::toon::ToonValue;
use crate::tools::{
    PermissionClass, Tool, ToolBody, ToolContext, ToolOutput, Truncation,
};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct GrepTool;

const DEFAULT_HEAD_LIMIT: usize = 250;
const MAX_OUTPUT_CHARS: usize = 20_000;

const DESCRIPTION: &str = "A powerful search tool built on ripgrep\n\n\
  Usage:\n\
  - ALWAYS use Grep for search tasks. NEVER invoke `grep` or `rg` as a Bash command. The Grep tool has been optimized for correct permissions and access.\n\
  - Supports full regex syntax (e.g., \"log.*Error\", \"function\\\\s+\\\\w+\")\n\
  - Filter files with glob parameter (e.g., \"*.js\", \"**/*.tsx\") or type parameter (e.g., \"js\", \"py\", \"rust\")\n\
  - Output modes: \"content\" shows matching lines, \"files_with_matches\" shows only file paths (default), \"count\" shows match counts\n\
  - Use Agent tool for open-ended searches requiring multiple rounds\n\
  - Pattern syntax: Uses ripgrep (not grep) - literal braces need escaping (use `interface\\\\{\\\\}` to find `interface{}` in Go code)\n\
  - Multiline matching: By default patterns match within single lines only. For cross-line patterns like `struct \\\\{[\\\\s\\\\S]*?field`, use `multiline: true`";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Regex pattern to search for" },
                "path": { "type": "string", "description": "File or directory to search in" },
                "glob": { "type": "string", "description": "Glob filter (e.g. \"*.rs\")" },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode (default: files_with_matches)"
                },
                "-B": { "type": "number", "description": "Lines of before-context" },
                "-A": { "type": "number", "description": "Lines of after-context" },
                "-C": { "type": "number", "description": "Lines of context (both sides)" },
                "context": { "type": "number", "description": "Alias for -C" },
                "-n": { "type": "boolean", "description": "Show line numbers (default true)" },
                "-i": { "type": "boolean", "description": "Case insensitive" },
                "type": { "type": "string", "description": "File type (e.g. \"rust\")" },
                "head_limit": { "type": "number", "description": "Max result lines (default 250)" },
                "offset": { "type": "number", "description": "Skip first N lines" },
                "multiline": { "type": "boolean", "description": "Enable multiline dotall" },
                "full": { "type": "boolean", "description": "Disable head_limit and return everything" }
            },
            "required": ["pattern"]
        })
    })
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "Grep"
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

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let pattern = input["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?;
        if pattern.is_empty() {
            return Ok(ToolOutput::error("Pattern cannot be empty"));
        }
        let full = input["full"].as_bool().unwrap_or(false);

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
            .unwrap_or("files_with_matches")
            .to_string();

        let context_lines = input["context"]
            .as_u64()
            .or_else(|| input["-C"].as_u64())
            .unwrap_or(0) as usize;
        let before_context = input["-B"].as_u64().unwrap_or(0) as usize;
        let after_context = input["-A"].as_u64().unwrap_or(0) as usize;
        let case_insensitive = input["-i"].as_bool().unwrap_or(false);
        let show_line_numbers = input["-n"].as_bool().unwrap_or(true);
        let head_limit = if full {
            0
        } else {
            input["head_limit"]
                .as_u64()
                .map(|n| n as usize)
                .unwrap_or(DEFAULT_HEAD_LIMIT)
        };
        let offset = input["offset"].as_u64().unwrap_or(0) as usize;
        let multiline = input["multiline"].as_bool().unwrap_or(false);

        let mut cmd = tokio::process::Command::new("rg");
        cmd.arg("--no-heading");
        cmd.arg("--max-columns").arg("500");
        cmd.arg("--max-columns-preview");

        match output_mode.as_str() {
            "files_with_matches" => {
                cmd.arg("--files-with-matches");
                cmd.arg("--sort=modified");
            }
            "count" => {
                cmd.arg("--count");
            }
            _ => {
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
        if let Some(ref g) = glob_filter {
            cmd.arg("--glob").arg(g);
        }
        if let Some(ref t) = type_filter {
            cmd.arg("--type").arg(t);
        }
        if pattern.starts_with('-') {
            cmd.arg("-e");
        }
        cmd.arg(pattern).arg(&search_path);

        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output_res = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            cmd.output(),
        )
        .await;

        let output = match output_res {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                return Ok(ToolOutput::error(format!(
                    "Search command failed: {e}. Is ripgrep (rg) installed?"
                )));
            }
            Err(_) => {
                return Ok(ToolOutput::error(
                    "Search timed out after 30s. Try a more specific pattern or path.",
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.code() == Some(1) && stdout.is_empty() {
            return Ok(ToolOutput::empty(format!(
                "No matches for {pattern:?}"
            ))
            .with_summary(format!("0 matches for {pattern:?}"))
            .with_next_step("Broaden the regex, try case-insensitive (-i), or search a wider path"));
        }

        if !output.status.success() && output.status.code() != Some(1) {
            return Ok(ToolOutput::error(format!(
                "Search failed: {}",
                if stderr.is_empty() { "unknown error" } else { stderr.trim() }
            )));
        }

        let result = relativize_paths(stdout.trim(), &context.cwd);
        if result.is_empty() {
            return Ok(ToolOutput::empty(format!(
                "No matches for {pattern:?}"
            )));
        }

        // Slice by offset + head_limit.
        let all_lines: Vec<&str> = result.lines().collect();
        let total = all_lines.len();
        if offset >= total && total > 0 {
            return Ok(ToolOutput::empty(format!(
                "No results at offset {offset} (total: {total})"
            )));
        }

        let sliced = &all_lines[offset..];
        let shown = if head_limit > 0 {
            sliced.len().min(head_limit)
        } else {
            sliced.len()
        };
        let lines: Vec<&&str> = sliced.iter().take(shown).collect();

        // Build body depending on output_mode.
        let body = match output_mode.as_str() {
            "files_with_matches" => {
                let rows: Vec<Vec<String>> =
                    lines.iter().map(|l| vec![l.to_string()]).collect();
                ToolBody::Toon(ToonValue::Map(vec![(
                    "files".into(),
                    ToonValue::Table {
                        columns: vec!["path".into()],
                        rows,
                    },
                )]))
            }
            "content" => {
                // Try to split "file:line:text" into columns when line numbers are on.
                if show_line_numbers {
                    let rows: Vec<Vec<String>> = lines
                        .iter()
                        .map(|l| parse_content_line(l))
                        .collect();
                    ToolBody::Toon(ToonValue::Map(vec![(
                        "matches".into(),
                        ToonValue::Table {
                            columns: vec!["file".into(), "line".into(), "text".into()],
                            rows,
                        },
                    )]))
                } else {
                    ToolBody::Text(lines.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("\n"))
                }
            }
            _ => ToolBody::Text(lines.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("\n")),
        };

        // Truncation bookkeeping: total minus offset is what was theoretically accessible.
        let accessible = total - offset;
        let was_truncated = !full && shown < accessible;

        let mut summary_count = shown;
        if output_mode == "count" {
            // In count mode, `shown` is already a files count.
            summary_count = shown;
        }
        let summary = match output_mode.as_str() {
            "files_with_matches" => format!("{summary_count} file(s) with matches"),
            "count" => format!("{summary_count} file(s) reported match counts"),
            _ => format!("{summary_count} match line(s)"),
        };

        let mut out = ToolOutput::success().with_summary(summary).with_body(body);

        if was_truncated {
            out = out
                .with_truncation(Truncation {
                    shown,
                    total: accessible,
                    unit: "lines",
                    how_to_see_more: "raise head_limit, bump offset, or pass full=true".into(),
                })
                .with_next_step("Pass full=true, or raise head_limit, or bump offset");
        }

        if output_mode != "files_with_matches" {
            out = out.with_next_step("Switch output_mode=files_with_matches for a file list");
        }

        // Final safety cap against runaway payloads (very long lines).
        let rendered = out.render();
        if rendered.len() > MAX_OUTPUT_CHARS {
            let safe =
                crate::tools::path_safety::safe_truncate(&rendered, MAX_OUTPUT_CHARS);
            return Ok(ToolOutput::success()
                .with_summary(format!("Grep (bytes-capped): {} total bytes", rendered.len()))
                .with_text(safe)
                .with_truncation(Truncation {
                    shown: MAX_OUTPUT_CHARS,
                    total: rendered.len(),
                    unit: "bytes",
                    how_to_see_more: "narrow the regex or filter with glob/type".into(),
                }));
        }

        Ok(out)
    }
}

fn parse_content_line(line: &str) -> Vec<String> {
    // Format typical: "file:line:text" but text may contain colons.
    let mut parts = line.splitn(3, ':');
    let file = parts.next().unwrap_or("").to_string();
    let line_no = parts.next().unwrap_or("").to_string();
    let text = parts.next().unwrap_or("").to_string();
    vec![file, line_no, text]
}

fn relativize_paths(text: &str, cwd: &std::path::Path) -> String {
    let cwd_str = format!("{}/", cwd.display());
    text.replace(&cwd_str, "")
}
