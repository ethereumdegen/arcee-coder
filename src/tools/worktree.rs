use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;
use tokio::process::Command;

pub struct EnterWorktreeTool;
pub struct ExitWorktreeTool;

#[async_trait]
impl Tool for EnterWorktreeTool {
    fn name(&self) -> &str {
        "EnterWorktree"
    }

    fn description(&self) -> String {
        "Create an isolated git worktree for working on changes without affecting \
         the main working directory. Creates a new branch based on HEAD."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Optional name for the worktree. A random name is generated if not provided."
                }
            }
        })
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        // Check we're in a git repo
        let git_check = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(&context.cwd)
            .output()
            .await?;

        if !git_check.status.success() {
            return Ok(ToolResult::error(
                "Not inside a git repository. EnterWorktree requires a git repo.",
            ));
        }

        let name = input["name"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("arcee-{}", &uuid::Uuid::new_v4().to_string()[..8]));

        // Sanitize name: only allow alphanumeric, hyphens, underscores
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Ok(ToolResult::error(
                "Worktree name must contain only alphanumeric characters, hyphens, and underscores.",
            ));
        }

        let worktree_dir = context.cwd.join(".arcee").join("worktrees").join(&name);
        let branch_name = format!("arcee-worktree-{name}");

        // Create the worktrees directory
        tokio::fs::create_dir_all(worktree_dir.parent().unwrap()).await?;

        // Create the worktree with a new branch
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
            return Ok(ToolResult::error(format!(
                "Failed to create worktree: {stderr}"
            )));
        }

        Ok(ToolResult::success(format!(
            "Created git worktree:\n\
             - Path: {}\n\
             - Branch: {}\n\
             \n\
             Use this path for file operations in the isolated worktree.\n\
             When done, use ExitWorktree to clean up.",
            worktree_dir.display(),
            branch_name
        )))
    }
}

#[async_trait]
impl Tool for ExitWorktreeTool {
    fn name(&self) -> &str {
        "ExitWorktree"
    }

    fn description(&self) -> String {
        "Remove a git worktree. If the worktree has uncommitted changes, \
         they will be preserved and the worktree path will be reported."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
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
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let worktree_path_str = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let worktree_path = PathBuf::from(worktree_path_str);

        if !worktree_path.exists() {
            return Ok(ToolResult::error(format!(
                "Worktree path does not exist: {worktree_path_str}"
            )));
        }

        // Check for uncommitted changes
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&worktree_path)
            .output()
            .await?;

        let has_changes = !String::from_utf8_lossy(&status.stdout).trim().is_empty();

        if has_changes {
            return Ok(ToolResult::success(format!(
                "Worktree at {} has uncommitted changes. \
                 Keeping worktree intact. Commit or stash changes before removing.",
                worktree_path.display()
            )));
        }

        // Remove the worktree
        let output = Command::new("git")
            .args(["worktree", "remove", &worktree_path.to_string_lossy()])
            .current_dir(&context.cwd)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(ToolResult::error(format!(
                "Failed to remove worktree: {stderr}"
            )));
        }

        Ok(ToolResult::success(format!(
            "Removed worktree at {}",
            worktree_path.display()
        )))
    }
}
