use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct TaskGetTool;

const DESCRIPTION: &str = "Use this tool to retrieve a task by its ID from the task list.\n\n\
## When to Use This Tool\n\n\
- When you need the full description and context before starting work on a task\n\
- To understand task dependencies (what it blocks, what blocks it)\n\
- After being assigned a task, to get complete requirements\n\n\
## Output\n\n\
Returns full task details:\n\
- **subject**: Task title\n\
- **description**: Detailed requirements and context\n\
- **status**: 'pending', 'in_progress', or 'completed'\n\
- **blocks**: Tasks waiting on this one to complete\n\
- **blockedBy**: Tasks that must complete before this one can start\n\n\
## Tips\n\n\
- After fetching a task, verify its blockedBy list is empty before beginning work.\n\
- Use TaskList to see all tasks in summary form.";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "taskId": {
                    "type": "string",
                    "description": "The ID of the task to retrieve"
                }
            },
            "required": ["taskId"]
        })
    })
}

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &'static str {
        "TaskGet"
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
        let task_id = input["taskId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'taskId' parameter"))?;

        let store = context.task_store.lock().await;
        match store.get(task_id) {
            Some(task) => Ok(ToolOutput::success()
                .with_summary(format!(
                    "Task #{} [{}]: {}",
                    task.id,
                    task.status.as_str(),
                    task.subject
                ))
                .with_text(task.format_detail())
                .with_next_step("Use TaskUpdate to change status or details")),
            None => Ok(ToolOutput::error(format!("Task #{task_id} not found"))),
        }
    }
}
