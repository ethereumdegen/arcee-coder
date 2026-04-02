use crate::tools::path_safety::{resolve_and_validate, safe_truncate};
use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct ReadTool;

const MAX_LINE_LENGTH: usize = 2000;
const DEFAULT_LIMIT: usize = 2000;
const MAX_FILE_SIZE: u64 = 256 * 1024 * 1024; // 256 MB

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "Read"
    }

    fn description(&self) -> String {
        "Reads a file from the filesystem. Returns the file contents with line numbers. \
         You can optionally specify an offset and limit for large files."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file to read"
                },
                "offset": {
                    "type": "number",
                    "description": "Line number to start reading from (1-based)"
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of lines to read (default: 2000)"
                }
            },
            "required": ["file_path"]
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let file_path_str = input["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path' parameter"))?;

        let file_path = match resolve_and_validate(file_path_str, &context.cwd) {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::error(e)),
        };

        if !file_path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                file_path.display()
            )));
        }

        if file_path.is_dir() {
            return Ok(ToolResult::error(format!(
                "{} is a directory, not a file. Use Bash with 'ls' to list directory contents.",
                file_path.display()
            )));
        }

        // Check file size before reading
        if let Ok(meta) = tokio::fs::metadata(&file_path).await {
            if meta.len() > MAX_FILE_SIZE {
                return Ok(ToolResult::error(format!(
                    "File too large ({:.1} MB). Use offset/limit to read portions, or Bash with 'head'/'tail'.",
                    meta.len() as f64 / 1_048_576.0
                )));
            }
        }

        let content = match tokio::fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                return Ok(ToolResult::error(format!(
                    "{} appears to be a binary file and cannot be read as text.",
                    file_path.display()
                )));
            }
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to read {}: {}",
                    file_path.display(),
                    e
                )));
            }
        };

        let offset = input["offset"].as_u64().unwrap_or(1).max(1) as usize;
        let limit = input["limit"].as_u64().unwrap_or(DEFAULT_LIMIT as u64) as usize;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let start = (offset - 1).min(total_lines);
        let end = (start + limit).min(total_lines);

        if start >= total_lines && total_lines > 0 {
            return Ok(ToolResult::error(format!(
                "Offset {} exceeds file length ({} lines).",
                offset, total_lines
            )));
        }

        let mut output = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_num = start + i + 1;
            let truncated = if line.len() > MAX_LINE_LENGTH {
                format!("{}... (truncated)", safe_truncate(line, MAX_LINE_LENGTH))
            } else {
                line.to_string()
            };
            output.push_str(&format!("{:>6}\t{}\n", line_num, truncated));
        }

        if end < total_lines {
            output.push_str(&format!(
                "\n... ({} more lines, {} total)",
                total_lines - end,
                total_lines
            ));
        }

        if output.is_empty() {
            output = "(empty file)".to_string();
        }

        Ok(ToolResult::success(output))
    }
}
