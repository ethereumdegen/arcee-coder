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
        "Performs exact string replacements in files.\n\n\
         REQUIRED parameters: \"file_path\" (string), \"old_string\" (string), \"new_string\" (string).\n\
         Example: {\"file_path\": \"/path/to/file.rs\", \"old_string\": \"old code\", \"new_string\": \"new code\"}\n\n\
         Usage:\n\
         - You must use your `Read` tool at least once in the conversation before editing. \
         This tool will error if you attempt an edit without reading the file. \n\
         - When editing text from Read tool output, ensure you preserve the exact indentation \
         (tabs/spaces) as it appears AFTER the line number prefix. The line number prefix \
         format is: spaces + line number + tab. Everything after that tab is the actual file \
         content to match. Never include any part of the line number prefix in the old_string \
         or new_string.\n\
         - ALWAYS prefer editing existing files in the codebase. NEVER write new files unless \
         explicitly required.\n\
         - Only use emojis if the user explicitly requests it. Avoid adding emojis to files \
         unless asked.\n\
         - The edit will FAIL if `old_string` is not unique in the file. Either provide a \
         larger string with more surrounding context to make it unique or use `replace_all` \
         to change every instance of `old_string`.\n\
         - Use `replace_all` for replacing and renaming strings across the file. This parameter \
         is useful if you want to rename a variable for instance."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to modify"
                },
                "old_string": {
                    "type": "string",
                    "description": "The text to replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace it with (must be different from old_string)"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences of old_string (default false)",
                    "default": false
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
                "File not found: {}. Read it again before attempting to write it.",
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

        // Try exact match first
        let occurrences = content.matches(old_string).count();

        if occurrences == 0 {
            // Try matching with curly quotes normalized to straight quotes
            let normalized_old = normalize_quotes(old_string);
            let normalized_content = normalize_quotes(&content);
            let norm_count = normalized_content.matches(&normalized_old).count();

            if norm_count > 0 {
                // Try to find the actual string in the file that matches after normalization
                if let Some(actual) = find_actual_string(&content, old_string) {
                    // Do the replacement using the actual string found in the file
                    let new_content = if replace_all {
                        content.replace(&actual, new_string)
                    } else {
                        content.replacen(&actual, new_string, 1)
                    };
                    return match tokio::fs::write(&file_path, &new_content).await {
                        Ok(()) => Ok(ToolResult::success(format!(
                            "The file {} has been updated successfully.",
                            file_path.display()
                        ))),
                        Err(e) => Ok(ToolResult::error(format!(
                            "Failed to write {}: {}", file_path.display(), e
                        ))),
                    };
                }

                return Ok(ToolResult::error(format!(
                    "old_string not found exactly, but {} match(es) found after normalizing \
                     curly quotes. Please use straight quotes in old_string.",
                    norm_count
                )));
            }

            // Check if file was modified since last read
            return Ok(ToolResult::error(format!(
                "old_string not found in {}. Make sure the string matches exactly \
                 (including whitespace and indentation). The file may have been modified \
                 since you last read it — use the Read tool to get the current contents.",
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
            Ok(()) => Ok(ToolResult::success(format!(
                "The file {} has been updated successfully.",
                file_path.display()
            ))),
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

/// Try to find the actual string in the file content that matches old_string
/// after curly quote normalization. Returns the actual (un-normalized) string.
fn find_actual_string(content: &str, old_string: &str) -> Option<String> {
    let normalized_old = normalize_quotes(old_string);
    let normalized_content = normalize_quotes(content);

    if let Some(pos) = normalized_content.find(&normalized_old) {
        // Map the byte position back to the original content
        // This is approximate but works for quote substitutions (same char count)
        let end = pos + normalized_old.len();
        if end <= content.len() {
            Some(content[pos..end].to_string())
        } else {
            None
        }
    } else {
        None
    }
}
