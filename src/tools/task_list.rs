use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct TaskListTool;

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "TaskList"
    }

    fn description(&self) -> String {
        "List all tasks in the task list with their status and summary.".to_string()
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
        let store = context.task_store.lock().await;
        let tasks = store.list();

        if tasks.is_empty() {
            return Ok(ToolResult::success("No tasks found."));
        }

        let output: Vec<String> = tasks.iter().map(|t| t.format_summary()).collect();
        Ok(ToolResult::success(output.join("\n")))
    }
}
