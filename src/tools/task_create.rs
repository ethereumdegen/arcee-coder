use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct TaskCreateTool;

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "TaskCreate"
    }

    fn description(&self) -> String {
        "Create a new task to track progress on multi-step work. Returns the created task with its ID.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "subject": {
                    "type": "string",
                    "description": "Brief imperative title for the task (e.g., 'Fix authentication bug')"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed description of what needs to be done"
                },
                "activeForm": {
                    "type": "string",
                    "description": "Present continuous form shown while task is in progress (e.g., 'Fixing authentication bug')"
                }
            },
            "required": ["subject", "description"]
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true // Task management doesn't need permission
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
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
        let task = store.create(subject, description, active_form);
        let result = format!("Task #{} created successfully: {}", task.id, task.subject);

        Ok(ToolResult::success(result))
    }
}
