use crate::tools::path_safety::{resolve_and_validate, safe_truncate};
use crate::tools::{PermissionClass, Tool, ToolBody, ToolContext, ToolOutput, Truncation};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct ReadTool;

const MAX_LINE_LENGTH: usize = 2000;
const DEFAULT_LIMIT: usize = 2000;
const MAX_FILE_SIZE: u64 = 256 * 1024 * 1024; // 256 MB

const DESCRIPTION: &str = "Reads a file from the local filesystem. You can access any file directly by using this tool.\n\
Assume this tool is able to read all files on the machine. If the User provides a path to a file assume that path is valid. \
It is okay to read a file that does not exist; an error will be returned.\n\n\
Usage:\n\
- The file_path parameter must be an absolute path, not a relative path\n\
- By default, it reads up to 2000 lines starting from the beginning of the file\n\
- You can optionally specify a line offset and limit (especially handy for long files), but it's recommended to read the whole file by not providing these parameters\n\
- Any lines longer than 2000 characters will be truncated\n\
- Results are returned with line numbers starting at 1, in the format: line_number→content (the → arrow separates the line number from file content)\n\
- This tool allows reading images (eg PNG, JPG, etc). When reading an image file the contents are presented visually.\n\
- This tool can read PDF files (.pdf). For large PDFs (more than 10 pages), you MUST provide the pages parameter to read specific page ranges (e.g., pages: \"1-5\"). Maximum 20 pages per request.\n\
- This tool can read Jupyter notebooks (.ipynb files) and returns all cells with their outputs, combining code, text, and visualizations.\n\
- This tool can only read files, not directories. To read a directory, use an ls command via the Bash tool.\n\
- You can call multiple tools in a single response. It is always better to speculatively read multiple potentially useful files in parallel.\n\
- You will regularly be asked to read screenshots. If the user provides a path to a screenshot, ALWAYS use this tool to view the file at the path.\n\
- If you read a file that exists but has empty contents you will receive a system reminder warning in place of file contents.";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to read"
                },
                "offset": {
                    "type": "number",
                    "description": "The line number to start reading from (1-based)."
                },
                "limit": {
                    "type": "number",
                    "description": "The number of lines to read. Defaults to 2000."
                },
                "full": {
                    "type": "boolean",
                    "description": "When true, disable truncation and return the whole file."
                },
                "pages": {
                    "type": "string",
                    "description": "Page range for PDF files (e.g., \"1-5\", \"3\", \"10-20\"). Only applicable to PDF files. Maximum 20 pages per request."
                }
            },
            "required": ["file_path"]
        })
    })
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "Read"
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

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolOutput> {
        let file_path_str = input["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path' parameter"))?;
        let full = input["full"].as_bool().unwrap_or(false);

        let file_path = match resolve_and_validate(file_path_str, &context.cwd) {
            Ok(p) => p,
            Err(e) => return Ok(ToolOutput::error(e)),
        };

        if !file_path.exists() {
            return Ok(ToolOutput::error(format!(
                "File not found: {}",
                file_path.display()
            )));
        }
        if file_path.is_dir() {
            return Ok(ToolOutput::error(format!(
                "{} is a directory, not a file. Use Bash with 'ls' to list directory contents.",
                file_path.display()
            )));
        }

        let mut file_bytes: u64 = 0;
        if let Ok(meta) = tokio::fs::metadata(&file_path).await {
            file_bytes = meta.len();
            if meta.len() > MAX_FILE_SIZE {
                return Ok(ToolOutput::error(format!(
                    "File too large ({:.1} MB). Use offset/limit or Bash with 'head'/'tail'.",
                    meta.len() as f64 / 1_048_576.0
                )));
            }
        }

        let content = match tokio::fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                return Ok(ToolOutput::error(format!(
                    "{} appears to be a binary file and cannot be read as text.",
                    file_path.display()
                )));
            }
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to read {}: {}",
                    file_path.display(),
                    e
                )));
            }
        };

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        if total_lines == 0 {
            return Ok(ToolOutput::empty(format!(
                "{} is empty (0 lines, {file_bytes} bytes).",
                file_path.display()
            ))
            .with_summary(format!("{}: empty file", file_path.display())));
        }

        let offset = input["offset"].as_u64().unwrap_or(1).max(1) as usize;
        let limit = if full {
            total_lines
        } else {
            input["limit"].as_u64().unwrap_or(DEFAULT_LIMIT as u64) as usize
        };

        let start = (offset - 1).min(total_lines);
        let end = (start + limit).min(total_lines);

        if start >= total_lines {
            return Ok(ToolOutput::error(format!(
                "Offset {} exceeds file length ({} lines).",
                offset, total_lines
            )));
        }

        let mut rendered = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_num = start + i + 1;
            let display = if line.len() > MAX_LINE_LENGTH {
                format!("{}... (truncated)", safe_truncate(line, MAX_LINE_LENGTH))
            } else {
                line.to_string()
            };
            rendered.push_str(&format!("{line_num:>5}→{display}\n"));
        }

        let shown = end - start;
        let size_kb = (file_bytes as f64) / 1024.0;
        let summary = format!(
            "{}: {} lines ({:.1} KB)",
            file_path.display(),
            total_lines,
            size_kb
        );

        let mut output = ToolOutput::success()
            .with_summary(summary)
            .with_body(ToolBody::Text(rendered));

        if end < total_lines && !full {
            let next_offset = end + 1;
            output = output.with_truncation(Truncation {
                shown,
                total: total_lines,
                unit: "lines",
                how_to_see_more: format!(
                    "call Read again with offset={next_offset}, or full=true for the whole file"
                ),
            });
            output = output.with_next_step(format!(
                "Read more with offset={next_offset} limit={DEFAULT_LIMIT}"
            ));
        }

        Ok(output)
    }
}
