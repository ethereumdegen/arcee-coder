use crate::tools::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::process::Command;

pub struct EnterWorktreeTool;
pub struct ExitWorktreeTool;

const ENTER_DESCRIPTION: &str = "Use this tool ONLY when the user explicitly asks to work in a worktree. \
This tool creates an isolated git worktree and switches the current session into it.\n\n\
## When to Use\n\
- The user explicitly says \"worktree\" (e.g., \"start a worktree\", \"work in a worktree\", \"create a worktree\", \"use a worktree\")\n\n\
## When NOT to Use\n\
- The user asks to create a branch, switch branches, or work on a different branch — use git commands instead\n\
- The user asks to fix a bug or work on a feature — use normal git workflow unless they specifically mention worktrees\n\
- Never use this tool unless the user explicitly mentions \"worktree\"\n\n\
## Requirements\n\
- Must be in a git repository\n\
- Must not already be in a worktree\n\n\
## Behavior\n\
- Creates a new git worktree inside `.arcee/worktrees/` with a new branch based on HEAD\n\
- Switches the session's working directory to the new worktree\n\
- On session exit, the user will be prompted to keep or remove the worktree\n\n\
## Parameters\n\
- `name` (optional): A name for the worktree. If not provided, a random name is generated.";

const EXIT_DESCRIPTION: &str = "Exit a worktree session created by EnterWorktree and return to original working directory.\n\n\
Only use this tool when:\n\
- The user explicitly asks to exit or remove a worktree\n\
- You need to clean up a worktree after completing work\n\n\
If the worktree has uncommitted changes, they are preserved and the worktree is left intact. \
Commit or stash changes before removing.";

fn enter_schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Optional name for the worktree"
                }
            }
        })
    })
}

fn exit_schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the worktree to remove"
                }
            },
            "required": ["path"]
        })
    })
}

#[async_trait]
impl Tool for EnterWorktreeTool {
    fn name(&self) -> &'static str {
        "EnterWorktree"
    }

    fn description(&self) -> &'static str {
        ENTER_DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        enter_schema()
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let git_check = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(&context.cwd)
            .output()
            .await?;

        if !git_check.status.success() {
            return Ok(ToolOutput::error(
                "Not inside a git repository. EnterWorktree requires a git repo.",
            ));
        }

        let name = input["name"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("arcee-{}", &uuid::Uuid::new_v4().to_string()[..8]));

        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Ok(ToolOutput::error(
                "Worktree name must contain only alphanumeric characters, hyphens, and underscores.",
            ));
        }

        let worktree_dir = context.cwd.join(".arcee").join("worktrees").join(&name);
        let branch_name = format!("arcee-worktree-{name}");

        tokio::fs::create_dir_all(worktree_dir.parent().unwrap()).await?;

        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                &branch_name,
                &worktree_dir.to_string_lossy(),
                "HEAD",
            ])
            .current_dir(&context.cwd)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(ToolOutput::error(format!(
                "Failed to create worktree: {stderr}"
            )));
        }

        Ok(ToolOutput::success()
            .with_summary(format!("Worktree {branch_name} created"))
            .with_text(format!(
                "Created git worktree:\n - Path: {}\n - Branch: {}",
                worktree_dir.display(),
                branch_name
            ))
            .with_next_step("Use ExitWorktree with path=... to clean up when done"))
    }
}

#[async_trait]
impl Tool for ExitWorktreeTool {
    fn name(&self) -> &'static str {
        "ExitWorktree"
    }

    fn description(&self) -> &'static str {
        EXIT_DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        exit_schema()
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let worktree_path_str = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let worktree_path = PathBuf::from(worktree_path_str);

        if !worktree_path.exists() {
            return Ok(ToolOutput::error(format!(
                "Worktree path does not exist: {worktree_path_str}"
            )));
        }

        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&worktree_path)
            .output()
            .await?;

        let has_changes = !String::from_utf8_lossy(&status.stdout).trim().is_empty();

        if has_changes {
            return Ok(ToolOutput::success()
                .with_summary("Worktree kept (uncommitted changes)")
                .with_text(format!(
                    "Worktree at {} has uncommitted changes. \
                     Commit or stash changes before removing.",
                    worktree_path.display()
                )));
        }

        let output = Command::new("git")
            .args(["worktree", "remove", &worktree_path.to_string_lossy()])
            .current_dir(&context.cwd)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(ToolOutput::error(format!(
                "Failed to remove worktree: {stderr}"
            )));
        }

        Ok(ToolOutput::success()
            .with_summary(format!("Removed worktree at {}", worktree_path.display()))
            .with_text(format!("Worktree {} removed.", worktree_path.display())))
    }
}
