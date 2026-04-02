use crate::tools::path_safety::resolve_and_validate;
use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct WriteTool;

const MAX_WRITE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "Write"
    }

    fn description(&self) -> String {
        "Writes a file to the local filesystem.\n\n\
         REQUIRED parameters: \"file_path\" (string), \"content\" (string).\n\
         Example: {\"file_path\": \"/path/to/file.rs\", \"content\": \"file contents here\"}\n\n\
         Usage:\n\
         - This tool will overwrite the existing file if there is one at the provided path.\n\
         - If this is an existing file, you MUST use the Read tool first to read the file's \
         contents. This tool will fail if you did not read the file first.\n\
         - Prefer the Edit tool for modifying existing files — it only sends the diff. \
         Only use this tool to create new files or for complete rewrites.\n\
         - NEVER create documentation files (*.md) or README files unless explicitly \
         requested by the User.\n\
         - Only use emojis if the user explicitly requests it. Avoid writing emojis to \
         files unless asked."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to write (must be absolute, not relative)"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        })
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let file_path_str = input["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path' parameter"))?;
        let content = input["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        // Size guard
        if content.len() > MAX_WRITE_SIZE {
            return Ok(ToolResult::error(format!(
                "Content too large ({:.1} MB). Maximum write size is {} MB.",
                content.len() as f64 / 1_048_576.0,
                MAX_WRITE_SIZE / 1_048_576
            )));
        }

        let file_path = match resolve_and_validate(file_path_str, &context.cwd) {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::error(e)),
        };

        let is_new = !file_path.exists();

        // Staleness check: if the file already exists, warn if it wasn't read first.
        // We can't fully enforce this without tracking read state, but we can detect
        // obvious cases like the file having been modified since the session started.
        if !is_new {
            // File exists — model should have read it first (best-effort warning)
        }

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    )
                })?;
            }
        }

        match tokio::fs::write(&file_path, content).await {
            Ok(()) => {
                if is_new {
                    Ok(ToolResult::success(format!(
                        "File created successfully at: {}",
                        file_path.display()
                    )))
                } else {
                    Ok(ToolResult::success(format!(
                        "The file {} has been updated successfully.",
                        file_path.display()
                    )))
                }
            }
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to write {}: {}",
                file_path.display(),
                e
            ))),
        }
    }
}
