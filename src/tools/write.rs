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
        "Writes content to a file, creating it if it doesn't exist or overwriting if it does. \
         Creates parent directories as needed."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file to write"
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
                let line_count = content.lines().count();
                let byte_count = content.len();
                Ok(ToolResult::success(format!(
                    "Wrote {} lines ({} bytes) to {}",
                    line_count,
                    byte_count,
                    file_path.display()
                )))
            }
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to write {}: {}",
                file_path.display(),
                e
            ))),
        }
    }
}
