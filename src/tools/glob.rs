use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> String {
        "Finds files matching a glob pattern. Returns matching file paths sorted by \
         modification time. Use patterns like \"**/*.rs\" or \"src/**/*.ts\"."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files (e.g., \"**/*.rs\")"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (defaults to cwd)"
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

        let base_path = match input["path"].as_str() {
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

        // Run glob in blocking task since it does filesystem I/O
        let base = base_path.clone();
        let pat = pattern.to_string();

        let result = tokio::task::spawn_blocking(move || find_matching_files(&base, &pat)).await?;

        match result {
            Ok(mut files) => {
                // Sort by modification time (newest first)
                files.sort_by(|a, b| {
                    let a_time = std::fs::metadata(a)
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let b_time = std::fs::metadata(b)
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    b_time.cmp(&a_time)
                });

                if files.is_empty() {
                    Ok(ToolResult::success(format!(
                        "No files found matching pattern: {pattern}"
                    )))
                } else {
                    let count = files.len();
                    let listing: String = files
                        .iter()
                        .map(|f| f.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(ToolResult::success(format!(
                        "{count} file(s) found:\n{listing}"
                    )))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("Glob error: {e}"))),
        }
    }
}

fn find_matching_files(
    base: &std::path::Path,
    pattern: &str,
) -> Result<Vec<PathBuf>, String> {
    let full_pattern = base.join(pattern);
    let pattern_str = full_pattern.to_string_lossy().to_string();

    let mut files = Vec::new();
    for entry in glob::glob(&pattern_str).map_err(|e| format!("Invalid glob pattern: {e}"))? {
        match entry {
            Ok(path) => files.push(path),
            Err(e) => {
                // Skip permission errors, etc.
                eprintln!("Glob entry error: {e}");
            }
        }
    }

    Ok(files)
}
