use crate::tools::path_safety::resolve_and_validate;
use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct EditTool;

const MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024; // 1 GB

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> String {
        "Performs exact string replacement in a file. The old_string must be unique \
         in the file unless replace_all is true. Use this for targeted edits to existing files."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact text to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement text"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false)"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let file_path_str = input["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path' parameter"))?;
        let old_string = input["old_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_string' parameter"))?;
        let new_string = input["new_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_string' parameter"))?;
        let replace_all = input["replace_all"].as_bool().unwrap_or(false);

        if old_string == new_string {
            return Ok(ToolResult::error(
                "old_string and new_string are identical",
            ));
        }

        if old_string.is_empty() {
            return Ok(ToolResult::error("old_string cannot be empty"));
        }

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

        // Size guard
        if let Ok(meta) = tokio::fs::metadata(&file_path).await {
            if meta.len() > MAX_FILE_SIZE {
                return Ok(ToolResult::error(format!(
                    "File too large ({:.1} MB). Edit is limited to files under 1 GB.",
                    meta.len() as f64 / 1_048_576.0
                )));
            }
        }

        let content = match tokio::fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to read {}: {}",
                    file_path.display(),
                    e
                )));
            }
        };

        // Try with curly-quote normalization if exact match fails
        let occurrences = content.matches(old_string).count();

        if occurrences == 0 {
            // Try normalizing curly quotes to straight quotes
            let normalized_old = normalize_quotes(old_string);
            let normalized_content = normalize_quotes(&content);
            let norm_count = normalized_content.matches(&normalized_old).count();

            if norm_count > 0 {
                return Ok(ToolResult::error(format!(
                    "old_string not found exactly, but {} match(es) found after normalizing \
                     curly quotes. Please use straight quotes in old_string.",
                    norm_count
                )));
            }

            return Ok(ToolResult::error(format!(
                "old_string not found in {}. Make sure the string matches exactly \
                 (including whitespace and indentation).",
                file_path.display()
            )));
        }

        if occurrences > 1 && !replace_all {
            return Ok(ToolResult::error(format!(
                "old_string found {occurrences} times in {}. Use replace_all: true to replace \
                 all occurrences, or provide more context to make the match unique.",
                file_path.display()
            )));
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        match tokio::fs::write(&file_path, &new_content).await {
            Ok(()) => {
                let replaced = if replace_all {
                    format!("Replaced {occurrences} occurrence(s)")
                } else {
                    "Replaced 1 occurrence".to_string()
                };
                Ok(ToolResult::success(format!(
                    "{replaced} in {}",
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

/// Normalize curly/smart quotes to straight quotes.
fn normalize_quotes(s: &str) -> String {
    s.replace('\u{2018}', "'")
        .replace('\u{2019}', "'")
        .replace('\u{201C}', "\"")
        .replace('\u{201D}', "\"")
}
