use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct TaskCreateTool;

const DESCRIPTION: &str = "Use this tool to create a structured task list for your current coding session. This helps you track progress, organize complex tasks, and demonstrate thoroughness to the user.\n\
It also helps the user understand the progress of the task and overall progress of their requests.\n\n\
## When to Use This Tool\n\n\
Use this tool proactively in these scenarios:\n\n\
- Complex multi-step tasks - When a task requires 3 or more distinct steps or actions\n\
- Non-trivial and complex tasks - Tasks that require careful planning or multiple operations\n\
- Plan mode - When using plan mode, create a task list to track the work\n\
- User explicitly requests todo list - When the user directly asks you to use the todo list\n\
- User provides multiple tasks - When users provide a list of things to be done (numbered or comma-separated)\n\
- After receiving new instructions - Immediately capture user requirements as tasks\n\
- When you start working on a task - Mark it as in_progress BEFORE beginning work\n\
- After completing a task - Mark it as completed and add any new follow-up tasks discovered during implementation\n\n\
## When NOT to Use This Tool\n\n\
Skip using this tool when:\n\
- There is only a single, straightforward task\n\
- The task is trivial and tracking it provides no organizational benefit\n\
- The task can be completed in less than 3 trivial steps\n\
- The task is purely conversational or informational\n\n\
NOTE that you should not use this tool if there is only one trivial task to do. In this case you are better off just doing the task directly.\n\n\
## Task Fields\n\n\
- **subject**: A brief, actionable title in imperative form (e.g., \"Fix authentication bug in login flow\")\n\
- **description**: Detailed description of what needs to be done, including context and acceptance criteria\n\
- **activeForm**: Present continuous form shown in spinner when task is in_progress (e.g., \"Fixing authentication bug\"). This is displayed to the user while you work on the task.\n\n\
**IMPORTANT**: Always provide activeForm when creating tasks. The subject should be imperative (\"Run tests\") while activeForm should be present continuous (\"Running tests\"). All tasks are created with status `pending`.\n\n\
## Tips\n\n\
- Create tasks with clear, specific subjects that describe the outcome\n\
- Include enough detail in the description for another agent to understand and complete the task\n\
- After creating tasks, use TaskUpdate to set up dependencies (blocks/blockedBy) if needed\n\
- Check TaskList first to avoid creating duplicate tasks";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "subject": {
                    "type": "string",
                    "description": "Brief imperative title for the task"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed description of what needs to be done"
                },
                "activeForm": {
                    "type": "string",
                    "description": "Present continuous form shown in spinner when in_progress (e.g., \"Running tests\")"
                },
                "metadata": {
                    "type": "object",
                    "description": "Arbitrary metadata to attach to the task",
                    "additionalProperties": {}
                }
            },
            "required": ["subject", "description"]
        })
    })
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &'static str {
        "TaskCreate"
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
        let subject = input["subject"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'subject' parameter"))?
            .to_string();
        let description = input["description"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'description' parameter"))?
            .to_string();
        let active_form = input["activeForm"].as_str().map(|s| s.to_string());

        let mut store = context.task_store.lock().await;
        let task = store.create(subject.clone(), description, active_form);

        Ok(ToolOutput::success()
            .with_summary(format!("Task #{} created", task.id))
            .with_text(format!("Created task #{}: {}", task.id, subject))
            .with_next_step("Use TaskUpdate to set status=in_progress when you start working on it"))
    }
}
