use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use tokio::io::AsyncWriteExt;

/// Hook configuration loaded from settings.json.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksConfig {
    #[serde(rename = "PreToolUse", default)]
    pub pre_tool_use: Vec<HookGroup>,

    #[serde(rename = "PostToolUse", default)]
    pub post_tool_use: Vec<HookGroup>,
}

/// A group of hooks that share a matcher pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookGroup {
    /// Tool name matcher: exact match or pipe-separated alternatives (e.g. "Write|Edit").
    /// None or empty matches all tools.
    pub matcher: Option<String>,

    /// Hook actions to run when matcher matches.
    pub hooks: Vec<HookAction>,
}

/// A single hook action (currently only "command" type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookAction {
    #[serde(rename = "type")]
    pub action_type: String,

    /// Shell command to execute.
    pub command: String,

    /// Timeout in seconds (default 120).
    pub timeout: Option<u64>,
}

/// JSON sent to hook command via stdin.
#[derive(Debug, Serialize)]
pub struct HookInput {
    pub hook_event_name: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_response: Option<String>,
    pub cwd: String,
}

/// JSON expected from hook command via stdout.
#[derive(Debug, Deserialize)]
pub struct HookOutput {
    /// Whether to continue execution (true) or block (false).
    #[serde(rename = "continue", default = "default_true")]
    pub should_continue: bool,

    /// Additional context to inject into tool result.
    #[serde(rename = "additionalContext")]
    pub additional_context: Option<String>,

    /// Decision override (e.g. "block", "allow").
    pub decision: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for HookOutput {
    fn default() -> Self {
        Self {
            should_continue: true,
            additional_context: None,
            decision: None,
        }
    }
}

/// Check if a tool name matches a matcher pattern.
/// Matcher can be: None (matches all), exact name, or pipe-separated alternatives.
fn matches(matcher: &Option<String>, tool_name: &str) -> bool {
    match matcher {
        None => true,
        Some(m) if m.is_empty() => true,
        Some(m) => m.split('|').any(|part| part.trim() == tool_name),
    }
}

/// Result of running a single hook command.
struct HookResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

/// Run a single hook command, piping input_json via stdin.
async fn run_hook_command(
    command: &str,
    input_json: &str,
    timeout_secs: u64,
    cwd: &Path,
) -> HookResult {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());

    let child = Command::new(&shell)
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return HookResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Failed to spawn hook command: {e}"),
                timed_out: false,
            };
        }
    };

    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input_json.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    // Capture stdout/stderr handles before waiting (wait_with_output consumes child)
    let timeout = tokio::time::Duration::from_secs(timeout_secs);

    let wait_fut = async {
        let output = child.wait_with_output().await?;
        Ok::<_, std::io::Error>(output)
    };

    match tokio::time::timeout(timeout, wait_fut).await {
        Ok(Ok(output)) => HookResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            timed_out: false,
        },
        Ok(Err(e)) => HookResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Hook command error: {e}"),
            timed_out: false,
        },
        Err(_) => {
            // Timeout — child is consumed by wait_fut so it gets dropped/killed
            HookResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Hook command timed out after {timeout_secs}s"),
                timed_out: true,
            }
        }
    }
}

/// Run PreToolUse hooks. Returns Some(block_reason) if the tool should be blocked.
pub async fn run_pre_tool_hooks(
    config: &HooksConfig,
    tool_name: &str,
    tool_input: &serde_json::Value,
    cwd: &Path,
) -> Option<String> {
    if config.pre_tool_use.is_empty() {
        return None;
    }

    let input = HookInput {
        hook_event_name: "PreToolUse".to_string(),
        tool_name: tool_name.to_string(),
        tool_input: tool_input.clone(),
        tool_response: None,
        cwd: cwd.display().to_string(),
    };
    let input_json = serde_json::to_string(&input).unwrap_or_default();

    for group in &config.pre_tool_use {
        if !matches(&group.matcher, tool_name) {
            continue;
        }
        for action in &group.hooks {
            if action.action_type != "command" {
                continue;
            }
            let timeout = action.timeout.unwrap_or(120);
            let result = run_hook_command(&action.command, &input_json, timeout, cwd).await;

            if result.timed_out {
                return Some(format!("Hook timed out: {}", action.command));
            }

            // Exit code 2 = blocking error
            if result.exit_code == 2 {
                let reason = if !result.stderr.is_empty() {
                    result.stderr.trim().to_string()
                } else if !result.stdout.is_empty() {
                    result.stdout.trim().to_string()
                } else {
                    "Hook returned blocking exit code 2".to_string()
                };
                return Some(reason);
            }

            // Parse stdout for structured response
            if !result.stdout.trim().is_empty() {
                if let Ok(output) = serde_json::from_str::<HookOutput>(result.stdout.trim()) {
                    if !output.should_continue {
                        let reason = output
                            .additional_context
                            .unwrap_or_else(|| "Hook blocked execution".to_string());
                        return Some(reason);
                    }
                }
            }
        }
    }

    None
}

/// Run PostToolUse hooks. Returns additional context to append to tool result.
pub async fn run_post_tool_hooks(
    config: &HooksConfig,
    tool_name: &str,
    tool_input: &serde_json::Value,
    tool_response: &str,
    cwd: &Path,
) -> String {
    if config.post_tool_use.is_empty() {
        return String::new();
    }

    let input = HookInput {
        hook_event_name: "PostToolUse".to_string(),
        tool_name: tool_name.to_string(),
        tool_input: tool_input.clone(),
        tool_response: Some(tool_response.to_string()),
        cwd: cwd.display().to_string(),
    };
    let input_json = serde_json::to_string(&input).unwrap_or_default();

    let mut context_parts = Vec::new();

    for group in &config.post_tool_use {
        if !matches(&group.matcher, tool_name) {
            continue;
        }
        for action in &group.hooks {
            if action.action_type != "command" {
                continue;
            }
            let timeout = action.timeout.unwrap_or(120);
            let result = run_hook_command(&action.command, &input_json, timeout, cwd).await;

            if result.timed_out {
                context_parts.push(format!("[hook timeout: {}]", action.command));
                continue;
            }

            // Exit code 2 = blocking error — show stderr
            if result.exit_code == 2 {
                let msg = if !result.stderr.is_empty() {
                    result.stderr.trim().to_string()
                } else {
                    "Hook returned blocking exit code 2".to_string()
                };
                context_parts.push(format!("[hook error: {msg}]"));
                continue;
            }

            // Non-zero exit (not 2) = warning, include stderr
            if result.exit_code != 0 && !result.stderr.is_empty() {
                context_parts.push(result.stderr.trim().to_string());
            }

            // Collect stdout — either structured or raw
            let stdout = result.stdout.trim();
            if !stdout.is_empty() {
                if let Ok(output) = serde_json::from_str::<HookOutput>(stdout) {
                    if let Some(ctx) = output.additional_context {
                        if !ctx.is_empty() {
                            context_parts.push(ctx);
                        }
                    }
                } else {
                    // Raw text output — include as-is
                    context_parts.push(stdout.to_string());
                }
            }
        }
    }

    context_parts.join("\n")
}

/// Merge project-level hooks into existing config (additive).
pub fn merge(base: &mut HooksConfig, overlay: HooksConfig) {
    base.pre_tool_use.extend(overlay.pre_tool_use);
    base.post_tool_use.extend(overlay.post_tool_use);
}
