use crate::permissions::PermissionMode;
use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct EnterPlanModeTool;
pub struct ExitPlanModeTool;

#[async_trait]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        "EnterPlanMode"
    }

    fn description(&self) -> String {
        "Switch to plan mode for designing an implementation approach. \
         In plan mode, you can explore the codebase with read-only tools \
         and write a plan for user approval before implementing."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn call(&self, _input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let mut mode = context.permission_mode.lock().await;

        if *mode == PermissionMode::Plan {
            return Ok(ToolResult::error("Already in plan mode."));
        }

        *mode = PermissionMode::Plan;

        // Generate plan file path
        let plan_path = context.cwd.join(".arcee").join("plan.md");
        {
            let mut pf = context.plan_file_path.lock().await;
            *pf = Some(plan_path.clone());
        }

        Ok(ToolResult::success(format!(
            "Entered plan mode. You are now in plan mode.\n\
             \n\
             In this mode:\n\
             - Use read-only tools (Read, Glob, Grep, WebFetch) to explore the codebase\n\
             - Write your plan to: {}\n\
             - Use ExitPlanMode when your plan is ready for user review\n\
             - Do NOT make code changes until the plan is approved",
            plan_path.display()
        )))
    }
}

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        "ExitPlanMode"
    }

    fn description(&self) -> String {
        "Exit plan mode and signal that your plan is ready for user review. \
         The plan file will be presented to the user for approval."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn call(&self, _input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let mut mode = context.permission_mode.lock().await;

        if *mode != PermissionMode::Plan {
            return Ok(ToolResult::error("Not currently in plan mode."));
        }

        // Restore to default mode
        *mode = PermissionMode::Default;

        let plan_path = {
            let pf = context.plan_file_path.lock().await;
            pf.clone()
        };

        if let Some(ref path) = plan_path {
            if path.exists() {
                let content = tokio::fs::read_to_string(path).await.unwrap_or_default();
                if content.is_empty() {
                    return Ok(ToolResult::success(
                        "Exited plan mode. No plan file was written. Ready for implementation.",
                    ));
                }
                return Ok(ToolResult::success(format!(
                    "Exited plan mode. Plan is ready for review at: {}\n\nPlan contents:\n{}",
                    path.display(),
                    content
                )));
            }
        }

        Ok(ToolResult::success(
            "Exited plan mode. Ready for implementation.",
        ))
    }
}
