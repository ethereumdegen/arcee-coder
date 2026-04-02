use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct TaskOutputTool;

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "TaskOutput"
    }

    fn description(&self) -> String {
        r#"Retrieve output from a background agent task.

- Takes a task_id parameter identifying the background task
- Returns the task output along with status information
- Use block=true (default) to wait for task completion
- Use block=false for non-blocking check of current status
- Task IDs are returned when launching agents with run_in_background=true
- You will be automatically notified when background tasks complete — prefer waiting for the notification over polling"#
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the background task to get output from"
                },
                "block": {
                    "type": "boolean",
                    "description": "Whether to wait for completion (default: true). Set to false for non-blocking status check."
                }
            },
            "required": ["task_id"]
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'task_id' parameter"))?;
        let block = input["block"].as_bool().unwrap_or(true);

        if block {
            // Wait for completion with a timeout
            let timeout = std::time::Duration::from_secs(300); // 5 min max
            let start = std::time::Instant::now();
            loop {
                {
                    let bg = context.background_tasks.lock().await;
                    if let Some(task) = bg.get(task_id) {
                        if task.status != crate::tools::background_tasks::BackgroundTaskStatus::Running {
                            return Ok(format_task_result(task));
                        }
                    } else {
                        return Ok(ToolResult::error(format!(
                            "No background task found with id '{task_id}'"
                        )));
                    }
                }
                if start.elapsed() > timeout {
                    return Ok(ToolResult::error(format!(
                        "Timed out waiting for background task #{task_id} to complete"
                    )));
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        } else {
            // Non-blocking check
            let bg = context.background_tasks.lock().await;
            if let Some(task) = bg.get(task_id) {
                Ok(format_task_result(task))
            } else {
                Ok(ToolResult::error(format!(
                    "No background task found with id '{task_id}'"
                )))
            }
        }
    }
}

fn format_task_result(task: &crate::tools::background_tasks::BackgroundTask) -> ToolResult {
    let elapsed = match task.completed_at {
        Some(end) => end.duration_since(task.started_at),
        None => task.started_at.elapsed(),
    };

    match &task.result {
        Some(result) => {
            let header = format!(
                "Background task #{} [{}] — {} ({:.1}s)\n\n",
                task.id,
                task.status.as_str(),
                task.description,
                elapsed.as_secs_f64()
            );
            ToolResult::success(format!("{header}{result}"))
        }
        None => ToolResult::success(format!(
            "Background task #{} [{}] — {} ({:.1}s elapsed, still running)",
            task.id,
            task.status.as_str(),
            task.description,
            elapsed.as_secs_f64()
        )),
    }
}
