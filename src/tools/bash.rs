use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

pub struct BashTool;

const MAX_OUTPUT_SIZE: usize = 1_000_000; // 1 MB
const MIN_TIMEOUT_MS: u64 = 1000;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn description(&self) -> String {
        "Executes a bash command and returns its output. The working directory \
         persists between commands. Use this for system operations, running tests, \
         git commands, and other terminal tasks."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional timeout in milliseconds (default: 120000, max: 600000)"
                },
                "description": {
                    "type": "string",
                    "description": "Brief description of what this command does"
                }
            },
            "required": ["command"]
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        false
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

        if command.trim().is_empty() {
            return Ok(ToolResult::error("Command cannot be empty"));
        }

        let timeout_ms = input["timeout"]
            .as_u64()
            .unwrap_or(120_000)
            .clamp(MIN_TIMEOUT_MS, 600_000);

        // Determine shell: prefer SHELL env, fall back to bash
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| s.ends_with("bash") || s.ends_with("zsh"))
            .unwrap_or_else(|| "bash".to_string());

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            execute_command(command, &context.cwd, &shell),
        )
        .await;

        match result {
            Ok(Ok((stdout, stderr, exit_code))) => {
                let mut output = String::new();

                if !stdout.is_empty() {
                    output.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&stderr);
                }

                if output.is_empty() {
                    output = "(no output)".to_string();
                }

                // Truncate large output
                if output.len() > MAX_OUTPUT_SIZE {
                    let truncated =
                        crate::tools::path_safety::safe_truncate(&output, MAX_OUTPUT_SIZE);
                    output = format!(
                        "{}\n\n... (output truncated, {} total bytes)",
                        truncated,
                        output.len()
                    );
                }

                if exit_code != 0 {
                    Ok(ToolResult::error(format!(
                        "Exit code {exit_code}\n{output}"
                    )))
                } else {
                    Ok(ToolResult::success(output))
                }
            }
            Ok(Err(e)) => Ok(ToolResult::error(format!("Failed to execute command: {e}"))),
            Err(_) => Ok(ToolResult::error(format!(
                "Command timed out after {timeout_ms}ms"
            ))),
        }
    }
}

async fn execute_command(
    command: &str,
    cwd: &std::path::Path,
    shell: &str,
) -> Result<(String, String, i32)> {
    let output = Command::new(shell)
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Isolate subprocess environment
        .env("TERM", std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()))
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, exit_code))
}
