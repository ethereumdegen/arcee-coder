use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct GlobTool;

/// Maximum number of files to return from a single glob search.
const MAX_RESULTS: usize = 100;
/// Timeout for the glob search in seconds.
const GLOB_TIMEOUT_SECS: u64 = 20;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> String {
        "Fast file pattern matching tool that works with any codebase size.\n\n\
         REQUIRED parameter: \"pattern\" (string) — the glob pattern to match files against.\n\
         Example call: {\"pattern\": \"**/*.rs\"} or {\"pattern\": \"src/**/*.ts\", \"path\": \"/some/dir\"}\n\n\
         - Supports glob patterns like \"**/*.rs\" or \"src/**/*.ts\"\n\
         - Returns matching file paths sorted by modification time (newest first)\n\
         - Results are limited to 100 files — use a more specific pattern or path to narrow results\n\
         - Respects .gitignore\n\
         - Optional \"path\" parameter to restrict search to a specific directory"
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match files against (e.g., \"**/*.rs\", \"src/**/*.ts\")"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in. Defaults to the current working directory. Use this to narrow searches to a specific subtree."
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

        // Validate that the path exists
        if !base_path.exists() {
            return Ok(ToolResult::error(format!(
                "Directory does not exist: {}",
                base_path.display()
            )));
        }

        let base = base_path.clone();
        let pat = pattern.to_string();
        let cwd = context.cwd.clone();

        // Try ripgrep first (fast, respects .gitignore), fall back to glob crate
        let result = tokio::task::spawn_blocking(move || {
            find_files_rg(&base, &pat, &cwd)
                .or_else(|_| find_files_glob(&base, &pat))
        })
        .await?;

        match result {
            Ok(files) => {
                if files.is_empty() {
                    Ok(ToolResult::success(format!(
                        "No files found matching pattern: {pattern}"
                    )))
                } else {
                    let total = files.len();
                    let truncated = total > MAX_RESULTS;
                    // Relativize paths to save tokens
                    let listing: String = files
                        .iter()
                        .take(MAX_RESULTS)
                        .map(|f| relativize(f, &context.cwd))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if truncated {
                        Ok(ToolResult::success(format!(
                            "{total} file(s) found (showing first {MAX_RESULTS}; use a more specific pattern or path to narrow results):\n{listing}"
                        )))
                    } else {
                        Ok(ToolResult::success(format!(
                            "{total} file(s) found:\n{listing}"
                        )))
                    }
                }
            }
            Err(e) => Ok(ToolResult::error(format!("Glob error: {e}"))),
        }
    }
}

/// Use ripgrep to find files matching a glob pattern (fast, respects .gitignore).
fn find_files_rg(
    base: &Path,
    pattern: &str,
    _cwd: &Path,
) -> std::result::Result<Vec<PathBuf>, String> {
    let output = std::process::Command::new("rg")
        .args([
            "--files",
            "--glob",
            pattern,
            "--sort=modified",
            // Cap output to avoid huge results
            "--max-count",
            &(MAX_RESULTS + 1).to_string(),
        ])
        .current_dir(base)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn rg: {e}"))?
        .wait_with_output();

    let output = match output {
        Ok(o) => o,
        Err(e) => return Err(format!("rg failed: {e}")),
    };

    // rg returns exit code 1 for "no matches" — that's fine
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files: Vec<PathBuf> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let p = PathBuf::from(l);
            if p.is_absolute() {
                p
            } else {
                base.join(p)
            }
        })
        .collect();

    // rg --sort=modified gives oldest first; we want newest first
    files.reverse();

    Ok(files)
}

/// Fallback: use the glob crate (slower, doesn't respect .gitignore).
fn find_files_glob(
    base: &Path,
    pattern: &str,
) -> std::result::Result<Vec<PathBuf>, String> {
    let full_pattern = base.join(pattern);
    let pattern_str = full_pattern.to_string_lossy().to_string();

    let mut files = Vec::new();
    let start = std::time::Instant::now();

    for entry in glob::glob(&pattern_str).map_err(|e| format!("Invalid glob pattern: {e}"))? {
        // Timeout protection
        if start.elapsed().as_secs() > GLOB_TIMEOUT_SECS {
            break;
        }
        match entry {
            Ok(path) if path.is_file() => {
                files.push(path);
                // Stop collecting after we have enough for truncation detection
                if files.len() > MAX_RESULTS * 2 {
                    break;
                }
            }
            _ => {}
        }
    }

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

    Ok(files)
}

/// Make a path relative to cwd to save tokens in output.
fn relativize(path: &Path, cwd: &Path) -> String {
    path.strip_prefix(cwd)
        .unwrap_or(path)
        .display()
        .to_string()
}
