use crate::tools::background_tasks::{BackgroundTask, BackgroundTaskStatus};
use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct TaskOutputTool;

const DESCRIPTION: &str = "- Retrieves output from a running or completed task (background shell, agent, or remote session)\n\
- Takes a task_id parameter identifying the task\n\
- Returns the task output along with status information\n\
- Use block=true (default) to wait for task completion\n\
- Use block=false for non-blocking check of current status\n\
- Task IDs can be found using the /tasks command\n\
- Works with all task types: background shells, async agents, and remote sessions";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The ID of the background task to get output from"
                },
                "block": {
                    "type": "boolean",
                    "default": true,
                    "description": "Whether to wait for completion"
                },
                "timeout": {
                    "type": "number",
                    "default": 30000,
                    "description": "Max wait time in ms",
                    "minimum": 0,
                    "maximum": 600000
                }
            },
            "required": ["task_id", "block", "timeout"]
        })
    })
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &'static str {
        "TaskOutput"
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
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'task_id' parameter"))?;
        let block = input["block"].as_bool().unwrap_or(true);

        if block {
            let timeout = std::time::Duration::from_secs(300);
            let start = std::time::Instant::now();
            loop {
                {
                    let bg = context.background_tasks.lock().await;
                    if let Some(task) = bg.get(task_id) {
                        if task.status != BackgroundTaskStatus::Running {
                            return Ok(format_task_output(task));
                        }
                    } else {
                        return Ok(ToolOutput::error(format!(
                            "No background task found with id '{task_id}'"
                        )));
                    }
                }
                if start.elapsed() > timeout {
                    return Ok(ToolOutput::error(format!(
                        "Timed out waiting for background task #{task_id} to complete"
                    )));
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        } else {
            let bg = context.background_tasks.lock().await;
            match bg.get(task_id) {
                Some(task) => Ok(format_task_output(task)),
                None => Ok(ToolOutput::error(format!(
                    "No background task found with id '{task_id}'"
                ))),
            }
        }
    }
}

fn format_task_output(task: &BackgroundTask) -> ToolOutput {
    let elapsed = match task.completed_at {
        Some(end) => end.duration_since(task.started_at),
        None => task.started_at.elapsed(),
    };

    let summary = format!(
        "task #{} [{}] {:.1}s",
        task.id,
        task.status.as_str(),
        elapsed.as_secs_f64()
    );

    match &task.result {
        Some(result) => {
            let header = format!(
                "Background task #{} [{}] — {} ({:.1}s)\n\n",
                task.id,
                task.status.as_str(),
                task.description,
                elapsed.as_secs_f64()
            );
            let mut out = ToolOutput::success()
                .with_summary(summary)
                .with_text(format!("{header}{result}"));
            if task.status == BackgroundTaskStatus::Failed {
                out.is_error = true;
            }
            out
        }
        None => ToolOutput::success()
            .with_summary(summary)
            .with_text(format!(
                "Background task #{} [{}] — {} ({:.1}s elapsed, still running)",
                task.id,
                task.status.as_str(),
                task.description,
                elapsed.as_secs_f64()
            ))
            .with_next_step("Wait for the completion notification, or call again with block=true"),
    }
}
