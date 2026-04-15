use crate::tools::path_safety::resolve_and_validate;
use crate::tools::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct EditTool;

const MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024; // 1 GB

const DESCRIPTION: &str = "Performs exact string replacements in files.\n\n\
Usage:\n\
- You must use your `Read` tool at least once in the conversation before editing. This tool will error if you attempt an edit without reading the file.\n\
- When editing text from Read tool output, ensure you preserve the exact indentation (tabs/spaces) as it appears AFTER the line number prefix. \
The line number prefix format is: spaces + line number + tab. Everything after that tab is the actual file content to match. \
Never include any part of the line number prefix in the old_string or new_string.\n\
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.\n\
- Only use emojis if the user explicitly requests it. Avoid adding emojis to files unless asked.\n\
- The edit will FAIL if `old_string` is not unique in the file. Either provide a larger string with more surrounding context to make it unique or use `replace_all` to change every instance of `old_string`.\n\
- Use `replace_all` for replacing and renaming strings across the file. This parameter is useful if you want to rename a variable for instance.";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string", "description": "The absolute path to the file to modify" },
                "old_string": { "type": "string", "description": "The text to replace" },
                "new_string": { "type": "string", "description": "The text to replace it with (must differ from old_string)" },
                "replace_all": { "type": "boolean", "description": "Replace all occurrences (default false)", "default": false }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    })
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "Edit"
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
        let old_string = input["old_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_string' parameter"))?;
        let new_string = input["new_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_string' parameter"))?;
        let replace_all = input["replace_all"].as_bool().unwrap_or(false);

        if old_string == new_string {
            return Ok(ToolOutput::error("old_string and new_string are identical"));
        }
        if old_string.is_empty() {
            return Ok(ToolOutput::error("old_string cannot be empty"));
        }

        let file_path = match resolve_and_validate(file_path_str, &context.cwd) {
            Ok(p) => p,
            Err(e) => return Ok(ToolOutput::error(e)),
        };

        if !file_path.exists() {
            return Ok(ToolOutput::error(format!(
                "File not found: {}. Read it again before attempting to write it.",
                file_path.display()
            )));
        }

        if let Ok(meta) = tokio::fs::metadata(&file_path).await {
            if meta.len() > MAX_FILE_SIZE {
                return Ok(ToolOutput::error(format!(
                    "File too large ({:.1} MB). Edit limited to files under 1 GB.",
                    meta.len() as f64 / 1_048_576.0
                )));
            }
        }

        let content = match tokio::fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to read {}: {}",
                    file_path.display(),
                    e
                )));
            }
        };

        let occurrences = content.matches(old_string).count();

        if occurrences == 0 {
            // Try curly-quote normalization.
            let normalized_old = normalize_quotes(old_string);
            let normalized_content = normalize_quotes(&content);
            let norm_count = normalized_content.matches(&normalized_old).count();
            if norm_count > 0 {
                if let Some(actual) = find_actual_string(&content, old_string) {
                    let new_content = if replace_all {
                        content.replace(&actual, new_string)
                    } else {
                        content.replacen(&actual, new_string, 1)
                    };
                    return match tokio::fs::write(&file_path, &new_content).await {
                        Ok(()) => Ok(success_output(
                            &file_path,
                            if replace_all { norm_count } else { 1 },
                        )),
                        Err(e) => Ok(ToolOutput::error(format!(
                            "Failed to write {}: {}",
                            file_path.display(),
                            e
                        ))),
                    };
                }
                return Ok(ToolOutput::error(format!(
                    "old_string not found exactly, but {norm_count} match(es) found after normalizing curly quotes. Use straight quotes."
                )));
            }
            return Ok(ToolOutput::error(format!(
                "old_string not found in {}. Make sure it matches exactly. The file may have been modified since you last read it.",
                file_path.display()
            )));
        }

        if occurrences > 1 && !replace_all {
            return Ok(ToolOutput::error(format!(
                "old_string found {occurrences} times in {}. Use replace_all: true or add more context.",
                file_path.display()
            )));
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        match tokio::fs::write(&file_path, &new_content).await {
            Ok(()) => Ok(success_output(
                &file_path,
                if replace_all { occurrences } else { 1 },
            )),
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to write {}: {}",
                file_path.display(),
                e
            ))),
        }
    }
}

fn success_output(file_path: &std::path::Path, replacements: usize) -> ToolOutput {
    let summary = format!(
        "{replacements} replacement{} in {}",
        if replacements == 1 { "" } else { "s" },
        file_path.display()
    );
    ToolOutput::success()
        .with_summary(summary)
        .with_text(format!(
            "The file {} has been updated successfully.",
            file_path.display()
        ))
        .with_next_step("Use Read to verify the edit applied as expected")
}

fn normalize_quotes(s: &str) -> String {
    s.replace('\u{2018}', "'")
        .replace('\u{2019}', "'")
        .replace('\u{201C}', "\"")
        .replace('\u{201D}', "\"")
}

fn find_actual_string(content: &str, old_string: &str) -> Option<String> {
    let normalized_old = normalize_quotes(old_string);
    let normalized_content = normalize_quotes(content);

    if let Some(norm_byte_pos) = normalized_content.find(&normalized_old) {
        let norm_char_start = normalized_content[..norm_byte_pos].chars().count();
        let norm_char_len = normalized_old.chars().count();

        let mut chars = content.char_indices();
        let start_byte = chars.nth(norm_char_start).map(|(i, _)| i)?;
        let end_byte = if norm_char_len == 0 {
            start_byte
        } else {
            let (i, ch) = chars.nth(norm_char_len - 1)?;
            i + ch.len_utf8()
        };

        Some(content[start_byte..end_byte].to_string())
    } else {
        None
    }
}
