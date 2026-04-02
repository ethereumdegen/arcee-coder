use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct TaskGetTool;

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "TaskGet"
    }

    fn description(&self) -> String {
        "Get full details of a specific task by its ID.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
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
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let task_id = input["taskId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'taskId' parameter"))?;

        let store = context.task_store.lock().await;
        match store.get(task_id) {
            Some(task) => Ok(ToolResult::success(task.format_detail())),
            None => Ok(ToolResult::error(format!("Task #{task_id} not found"))),
        }
    }
}
