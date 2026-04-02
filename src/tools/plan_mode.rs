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
        r#"Use this tool proactively when you're about to start a non-trivial implementation task. Getting user sign-off on your approach before writing code prevents wasted effort and ensures alignment. This tool transitions you into plan mode where you can explore the codebase and design an implementation approach for user approval.

## When to Use This Tool

**Prefer using EnterPlanMode** for implementation tasks unless they're simple. Use it when ANY of these conditions apply:

1. **New Feature Implementation**: Adding meaningful new functionality
   - Example: "Add a logout button" - where should it go? What should happen on click?
   - Example: "Add form validation" - what rules? What error messages?

2. **Multiple Valid Approaches**: The task can be solved in several different ways
   - Example: "Add caching to the API" - could use Redis, in-memory, file-based, etc.
   - Example: "Improve performance" - many optimization strategies possible

3. **Code Modifications**: Changes that affect existing behavior or structure
   - Example: "Update the login flow" - what exactly should change?
   - Example: "Refactor this component" - what's the target architecture?

4. **Architectural Decisions**: The task requires choosing between patterns or technologies

5. **Multi-File Changes**: The task will likely touch more than 2-3 files

6. **Unclear Requirements**: You need to explore before understanding the full scope

7. **User Preferences Matter**: The implementation could reasonably go multiple ways
   - If you would use AskUserQuestion to clarify the approach, use EnterPlanMode instead
   - Plan mode lets you explore first, then present options with context

## When NOT to Use This Tool

Only skip EnterPlanMode for simple tasks:
- Single-line or few-line fixes (typos, obvious bugs, small tweaks)
- Adding a single function with clear requirements
- Tasks where the user has given very specific, detailed instructions
- Pure research/exploration tasks (use the Agent tool with explore agent instead)

## What Happens in Plan Mode

In plan mode, you'll:
1. Thoroughly explore the codebase using Glob, Grep, and Read tools
2. Understand existing patterns and architecture
3. Design an implementation approach
4. Present your plan to the user for approval
5. Use AskUserQuestion if you need to clarify approaches
6. Exit plan mode with ExitPlanMode when ready to implement

## Important Notes

- If unsure whether to use it, err on the side of planning - it's better to get alignment upfront than to redo work
- Users appreciate being consulted before significant changes are made to their codebase"#
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
        r#"Exit plan mode and signal that your plan is ready for user review. The plan file will be presented to the user for approval.

## How This Tool Works
- You should have already written your plan to the plan file (.arcee/plan.md)
- This tool reads the plan from the file and presents it to the user
- The user will review and can approve or provide feedback

## Before Using This Tool
Ensure your plan is complete and unambiguous:
- If you have unresolved questions about requirements or approach, use AskUserQuestion first
- Once your plan is finalized, use THIS tool to request approval

**Important:** Do NOT use AskUserQuestion to ask "Is this plan okay?" — that's exactly what THIS tool does."#
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
                    "Exited plan mode. Plan is ready for review.\n\n\
                     Your plan has been saved to: {}\n\
                     You can refer back to it during implementation.\n\n\
                     ## Plan:\n{}\n\n\
                     Start with creating tasks (TaskCreate) to track your implementation progress if applicable.",
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
