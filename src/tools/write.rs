use crate::tools::path_safety::resolve_and_validate;
use crate::tools::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct WriteTool;

const MAX_WRITE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

const DESCRIPTION: &str = "Writes a file to the local filesystem.\n\n\
Usage:\n\
- This tool will overwrite the existing file if there is one at the provided path.\n\
- If this is an existing file, you MUST use the Read tool first to read the file's contents. This tool will fail if you did not read the file first.\n\
- Prefer the Edit tool for modifying existing files — it only sends the diff. Only use this tool to create new files or for complete rewrites.\n\
- NEVER create documentation files (*.md) or README files unless explicitly requested by the User.\n\
- Only use emojis if the user explicitly requests it. Avoid writing emojis to files unless asked.";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        })
    })
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "Write"
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        schema()
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let file_path_str = input["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path' parameter"))?;
        let content = input["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        if content.len() > MAX_WRITE_SIZE {
            return Ok(ToolOutput::error(format!(
                "Content too large ({:.1} MB). Max write size is {} MB.",
                content.len() as f64 / 1_048_576.0,
                MAX_WRITE_SIZE / 1_048_576
            )));
        }

        let file_path = match resolve_and_validate(file_path_str, &context.cwd) {
            Ok(p) => p,
            Err(e) => return Ok(ToolOutput::error(e)),
        };
        let is_new = !file_path.exists();

        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    return Ok(ToolOutput::error(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    )));
                }
            }
        }

        match tokio::fs::write(&file_path, content).await {
            Ok(()) => {
                let line_count = content.lines().count().max(1);
                let verb = if is_new { "Created" } else { "Updated" };
                let summary = format!(
                    "{verb} {} ({line_count} lines, {} bytes)",
                    file_path.display(),
                    content.len()
                );
                Ok(ToolOutput::success()
                    .with_summary(summary)
                    .with_text(format!(
                        "File {} was {}.",
                        file_path.display(),
                        if is_new { "created" } else { "updated" }
                    ))
                    .with_next_step("Use Read to verify the new contents if needed"))
            }
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to write {}: {}",
                file_path.display(),
                e
            ))),
        }
    }
}
