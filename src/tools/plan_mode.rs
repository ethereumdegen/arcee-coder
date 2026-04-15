use crate::permissions::PermissionMode;
use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct EnterPlanModeTool;
pub struct ExitPlanModeTool;

const ENTER_DESCRIPTION: &str = "Use this tool proactively when you're about to start a non-trivial implementation task. \
Getting user sign-off on your approach before writing code prevents wasted effort and ensures alignment. \
This tool transitions you into plan mode where you can explore the codebase and design an implementation approach for user approval.\n\n\
## When to Use This Tool\n\n\
**Prefer using EnterPlanMode** for implementation tasks unless they're simple. Use it when ANY of these conditions apply:\n\n\
1. **New Feature Implementation**: Adding meaningful new functionality\n\
2. **Multiple Valid Approaches**: The task can be solved in several different ways\n\
3. **Code Modifications**: Changes that affect existing behavior or structure\n\
4. **Architectural Decisions**: The task requires choosing between patterns or technologies\n\
5. **Multi-File Changes**: The task will likely touch more than 2-3 files\n\
6. **Unclear Requirements**: You need to explore before understanding the full scope\n\
7. **User Preferences Matter**: The implementation could reasonably go multiple ways\n\n\
## When NOT to Use This Tool\n\n\
Only skip EnterPlanMode for simple tasks:\n\
- Single-line or few-line fixes (typos, obvious bugs, small tweaks)\n\
- Adding a single function with clear requirements\n\
- Tasks where the user has given very specific, detailed instructions\n\
- Pure research/exploration tasks (use the Agent tool with explore agent instead)\n\n\
## What Happens in Plan Mode\n\n\
In plan mode, you'll:\n\
1. Thoroughly explore the codebase using Glob, Grep, and Read tools\n\
2. Understand existing patterns and architecture\n\
3. Design an implementation approach\n\
4. Present your plan to the user for approval\n\
5. Use AskUserQuestion if you need to clarify approaches\n\
6. Exit plan mode with ExitPlanMode when ready to implement";

const EXIT_DESCRIPTION: &str = "Use this tool when you are in plan mode and have finished writing your plan to the plan file and are ready for user approval.\n\n\
## How This Tool Works\n\
- You should have already written your plan to the plan file specified in the plan mode system message\n\
- This tool does NOT take the plan content as a parameter - it will read the plan from the file you wrote\n\
- This tool simply signals that you're done planning and ready for the user to review and approve\n\
- The user will see the contents of your plan file when they review it\n\n\
## When to Use This Tool\n\
IMPORTANT: Only use this tool when the task requires planning the implementation steps of a task that requires writing code. \
For research tasks where you're gathering information, searching files, reading files or in general trying to understand the codebase - do NOT use this tool.\n\n\
## Before Using This Tool\n\
Ensure your plan is complete and unambiguous:\n\
- If you have unresolved questions about requirements or approach, use AskUserQuestion first (in earlier phases)\n\
- Once your plan is finalized, use THIS tool to request approval\n\n\
**Important:** Do NOT use AskUserQuestion to ask \"Is this plan okay?\" or \"Should I proceed?\" - that's exactly what THIS tool does. \
ExitPlanMode inherently requests user approval of your plan.";

fn enter_schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": {},
            "properties": {}
        })
    })
}

fn exit_schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": {},
            "properties": {
                "allowedPrompts": {
                    "type": "array",
                    "description": "Prompt-based permissions needed to implement the plan. These describe categories of actions rather than specific commands.",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "tool": {
                                "type": "string",
                                "description": "The tool this prompt applies to",
                                "enum": ["Bash"]
                            },
                            "prompt": {
                                "type": "string",
                                "description": "Semantic description of the action, e.g. \"run tests\", \"install dependencies\""
                            }
                        },
                        "required": ["tool", "prompt"]
                    }
                }
            }
        })
    })
}

#[async_trait]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &'static str {
        "EnterPlanMode"
    }

    fn description(&self) -> &'static str {
        ENTER_DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        enter_schema()
    }

    fn permission(&self, _input: &serde_json::Value) -> PermissionClass {
        PermissionClass::ReadOnly
    }

    async fn call(&self, _input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let mut mode = context.permission_mode.lock().await;

        if *mode == PermissionMode::Plan {
            return Ok(ToolOutput::error("Already in plan mode."));
        }

        *mode = PermissionMode::Plan;

        let plan_path = context.cwd.join(".arcee").join("plan.md");
        {
            let mut pf = context.plan_file_path.lock().await;
            *pf = Some(plan_path.clone());
        }

        Ok(ToolOutput::success()
            .with_summary("Entered plan mode")
            .with_text(format!(
                "You are now in plan mode.\n\
                 \n\
                 - Use read-only tools (Read, Glob, Grep, WebFetch) to explore the codebase\n\
                 - Write your plan to: {}\n\
                 - Do NOT make code changes until the plan is approved",
                plan_path.display()
            ))
            .with_next_step("Use ExitPlanMode when your plan is ready for user review"))
    }
}

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &'static str {
        "ExitPlanMode"
    }

    fn description(&self) -> &'static str {
        EXIT_DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        exit_schema()
    }

    fn permission(&self, _input: &serde_json::Value) -> PermissionClass {
        PermissionClass::ReadOnly
    }

    async fn call(&self, _input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let mut mode = context.permission_mode.lock().await;

        if *mode != PermissionMode::Plan {
            return Ok(ToolOutput::error("Not currently in plan mode."));
        }

        *mode = PermissionMode::Default;

        let plan_path = {
            let pf = context.plan_file_path.lock().await;
            pf.clone()
        };

        if let Some(ref path) = plan_path {
            if path.exists() {
                let content = tokio::fs::read_to_string(path).await.unwrap_or_default();
                if content.is_empty() {
                    return Ok(ToolOutput::success()
                        .with_summary("Exited plan mode (empty plan)")
                        .with_text("No plan file was written. Ready for implementation."));
                }
                return Ok(ToolOutput::success()
                    .with_summary(format!("Exited plan mode — plan at {}", path.display()))
                    .with_text(format!(
                        "Your plan has been saved to: {}\n\
                         You can refer back to it during implementation.\n\n\
                         ## Plan:\n{}",
                        path.display(),
                        content
                    ))
                    .with_next_step(
                        "Create tasks with TaskCreate to track implementation progress",
                    ));
            }
        }

        Ok(ToolOutput::success()
            .with_summary("Exited plan mode")
            .with_text("Ready for implementation."))
    }
}
