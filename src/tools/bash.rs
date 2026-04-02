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
        "Executes a given bash command and returns its output.\n\n\
         REQUIRED parameter: \"command\" (string) — the bash command to execute.\n\
         Example: {\"command\": \"ls -la /home/user/project\"}\n\n\
         The working directory persists between commands, but shell state does not. \
         The shell environment is initialized from the user's profile (bash or zsh).\n\n\
         IMPORTANT: Avoid using this tool to run `find`, `grep`, `cat`, `head`, `tail`, \
         `sed`, `awk`, or `echo` commands, unless explicitly instructed or after you have \
         verified that a dedicated tool cannot accomplish your task. Instead, use the \
         appropriate dedicated tool as this will provide a much better experience for the user:\n\n\
         - File search: Use Glob (NOT find or ls)\n\
         - Content search: Use Grep (NOT grep or rg)\n\
         - Read files: Use Read (NOT cat/head/tail)\n\
         - Edit files: Use Edit (NOT sed/awk)\n\
         - Write files: Use Write (NOT echo >/cat <<EOF)\n\
         - Communication: Output text directly (NOT echo/printf)\n\
         While the Bash tool can do similar things, it's better to use the built-in tools as \
         they provide a much better experience for the user.\n\n\
         # Instructions\n\
         - If your command will create new directories or files, first use this tool to run \
         `ls` to verify the parent directory exists and is the correct location.\n\
         - Always quote file paths that contain spaces with double quotes in your command.\n\
         - Try to maintain your current working directory throughout the session by using \
         absolute paths and avoiding usage of `cd`.\n\
         - You may specify an optional timeout in milliseconds (up to 600000ms / 10 minutes). \
         By default, your command will timeout after 120000ms (2 minutes).\n\
         - You can use the `run_in_background` parameter to run the command in the background. \
         Only use this if you don't need the result immediately and are OK being notified when \
         the command completes later.\n\
         - Write a clear, concise description of what your command does.\n\
         - When issuing multiple commands: if independent, make multiple Bash calls in parallel. \
         If dependent, use && to chain them. Use ; only when you don't care if earlier commands fail.\n\
         - For git commands: prefer creating new commits over amending. Never skip hooks \
         (--no-verify) unless explicitly asked."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional timeout in milliseconds (max 600000)"
                },
                "description": {
                    "type": "string",
                    "description": "Clear, concise description of what this command does in active voice"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Set to true to run this command in the background. Use TaskOutput to read the output later."
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

        let run_in_background = input["run_in_background"].as_bool().unwrap_or(false);
        let description = input["description"]
            .as_str()
            .unwrap_or(command)
            .to_string();

        // Determine shell: prefer SHELL env, fall back to bash
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| s.ends_with("bash") || s.ends_with("zsh"))
            .unwrap_or_else(|| "bash".to_string());

        if run_in_background {
            return self
                .launch_background(command, &description, &shell, context)
                .await;
        }

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

impl BashTool {
    async fn launch_background(
        &self,
        command: &str,
        description: &str,
        shell: &str,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        let task_id = {
            let mut bg = context.background_tasks.lock().await;
            bg.register(description.to_string(), "bash".to_string())
        };

        let cmd_owned = command.to_string();
        let cwd = context.cwd.clone();
        let shell_owned = shell.to_string();
        let bg_store = context.background_tasks.clone();
        let task_id_clone = task_id.clone();

        tokio::spawn(async move {
            let result = execute_command(&cmd_owned, &cwd, &shell_owned).await;
            let mut bg = bg_store.lock().await;

            match result {
                Ok((stdout, stderr, exit_code)) => {
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
                    if output.len() > MAX_OUTPUT_SIZE {
                        let truncated =
                            crate::tools::path_safety::safe_truncate(&output, MAX_OUTPUT_SIZE);
                        output = format!("{truncated}\n\n... (truncated)");
                    }

                    if exit_code != 0 {
                        bg.fail(&task_id_clone, format!("Exit code {exit_code}\n{output}"));
                    } else {
                        bg.complete(&task_id_clone, output);
                    }
                }
                Err(e) => {
                    bg.fail(&task_id_clone, format!("Command failed: {e}"));
                }
            }
        });

        Ok(ToolResult::success(format!(
            "Command launched in background as task #{task_id}. \
             You will be automatically notified when it completes. \
             Continue with other work — do NOT poll or check on it. \
             Use TaskOutput with task_id=\"{task_id}\" to retrieve results after notification."
        )))
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
        .env(
            "TERM",
            std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()),
        )
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, exit_code))
}
