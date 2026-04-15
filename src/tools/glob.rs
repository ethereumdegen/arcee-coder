use crate::toon::ToonValue;
use crate::tools::{
    PermissionClass, Tool, ToolBody, ToolContext, ToolOutput, Truncation,
};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

pub struct GlobTool;

const MAX_RESULTS: usize = 100;
const GLOB_TIMEOUT_SECS: u64 = 20;

const DESCRIPTION: &str = "- Fast file pattern matching tool that works with any codebase size\n\
- Supports glob patterns like \"**/*.js\" or \"src/**/*.ts\"\n\
- Returns matching file paths sorted by modification time\n\
- Use this tool when you need to find files by name patterns\n\
- When you are doing an open ended search that may require multiple rounds of globbing and grepping, use the Agent tool instead\n\
- You can call multiple tools in a single response. It is always better to speculatively perform multiple searches in parallel if they are potentially useful.";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. \"**/*.rs\")"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default cwd)"
                },
                "full": {
                    "type": "boolean",
                    "description": "When true, disable the 100-result cap"
                }
            },
            "required": ["pattern"]
        })
    })
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "Glob"
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

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let pattern = input["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?;
        let full = input["full"].as_bool().unwrap_or(false);

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

        if !base_path.exists() {
            return Ok(ToolOutput::error(format!(
                "Directory does not exist: {}",
                base_path.display()
            )));
        }

        let base = base_path.clone();
        let pat = pattern.to_string();
        let cwd = context.cwd.clone();

        let result = tokio::task::spawn_blocking(move || {
            find_files_rg(&base, &pat, &cwd).or_else(|_| find_files_glob(&base, &pat))
        })
        .await?;

        match result {
            Ok(files) => {
                if files.is_empty() {
                    return Ok(ToolOutput::empty(format!(
                        "Pattern {pattern:?} did not match any files."
                    ))
                    .with_summary("0 files matched")
                    .with_next_step(
                        "Try a broader pattern or verify the path; use Grep for content search",
                    ));
                }

                let total = files.len();
                let limit = if full { total } else { MAX_RESULTS };
                let shown = total.min(limit);

                let rows: Vec<Vec<String>> = files
                    .iter()
                    .take(shown)
                    .map(|f| {
                        let rel = relativize(f, &context.cwd);
                        let (size, modified) = stat_for(f);
                        vec![rel, size, modified]
                    })
                    .collect();

                let table = ToonValue::Map(vec![(
                    "files".into(),
                    ToonValue::Table {
                        columns: vec!["path".into(), "size".into(), "modified".into()],
                        rows,
                    },
                )]);

                let summary = format!("{total} file(s) matched {pattern:?}");
                let mut output = ToolOutput::success()
                    .with_summary(summary)
                    .with_body(ToolBody::Toon(table));

                if shown < total {
                    output = output
                        .with_truncation(Truncation {
                            shown,
                            total,
                            unit: "files",
                            how_to_see_more: format!(
                                "narrow the pattern (e.g. {pattern:?} → tighter subdir), or pass full=true"
                            ),
                        })
                        .with_next_step("Narrow with a more specific pattern or set full=true");
                } else {
                    output = output.with_next_step(
                        "Use Read on individual files or Grep to search their content",
                    );
                }

                Ok(output)
            }
            Err(e) => Ok(ToolOutput::error(format!("Glob error: {e}"))),
        }
    }
}

fn stat_for(path: &Path) -> (String, String) {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return (String::from("?"), String::from("?")),
    };
    let size = human_size(meta.len());
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| {
            chrono::DateTime::<chrono::Utc>::from(SystemTime::UNIX_EPOCH + d)
                .format("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_else(|| String::from("?"));
    (size, modified)
}

fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}

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

    files.reverse();
    Ok(files)
}

fn find_files_glob(base: &Path, pattern: &str) -> std::result::Result<Vec<PathBuf>, String> {
    let full_pattern = base.join(pattern);
    let pattern_str = full_pattern.to_string_lossy().to_string();

    let mut files = Vec::new();
    let start = std::time::Instant::now();

    for entry in glob::glob(&pattern_str).map_err(|e| format!("Invalid glob pattern: {e}"))? {
        if start.elapsed().as_secs() > GLOB_TIMEOUT_SECS {
            break;
        }
        match entry {
            Ok(path) if path.is_file() => {
                files.push(path);
                if files.len() > MAX_RESULTS * 2 {
                    break;
                }
            }
            _ => {}
        }
    }

    files.sort_by(|a, b| {
        let a_time = std::fs::metadata(a)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let b_time = std::fs::metadata(b)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        b_time.cmp(&a_time)
    });
    Ok(files)
}

fn relativize(path: &Path, cwd: &Path) -> String {
    path.strip_prefix(cwd).unwrap_or(path).display().to_string()
}
