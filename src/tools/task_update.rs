use crate::tools::task_store::TaskStatus;
use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct TaskUpdateTool;

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "TaskUpdate"
    }

    fn description(&self) -> String {
        r#"Use this tool to update a task in the task list.

## When to Use This Tool

**Mark tasks as in_progress:**
- When you START working on a task, mark it in_progress BEFORE beginning work

**Mark tasks as completed:**
- When you have FULLY completed the work described in a task
- IMPORTANT: Always mark your tasks as completed when you finish them
- After completing, call TaskList to find your next task

- ONLY mark a task as completed when you have FULLY accomplished it
- If you encounter errors, blockers, or cannot finish, keep the task as in_progress
- When blocked, create a new task describing what needs to be resolved
- Never mark a task as completed if:
  - Tests are failing
  - Implementation is partial
  - You encountered unresolved errors

**Delete tasks:**
- When a task is no longer relevant or was created in error

## Status Workflow

Status progresses: `pending` → `in_progress` → `completed`
Use `deleted` to permanently remove a task."#
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "taskId": {
                    "type": "string",
                    "description": "The ID of the task to update"
                },
                "status": {
                    "type": "string",
                    "description": "New status: pending, in_progress, completed, or deleted"
                },
                "subject": {
                    "type": "string",
                    "description": "New subject/title for the task"
                },
                "description": {
                    "type": "string",
                    "description": "New description for the task"
                },
                "activeForm": {
                    "type": "string",
                    "description": "Present continuous form for spinner display"
                },
                "owner": {
                    "type": "string",
                    "description": "New owner for the task"
                },
                "addBlocks": {
                    "type": "array",
                    "description": "Task IDs that this task blocks",
                    "items": { "type": "string" }
                },
                "addBlockedBy": {
                    "type": "array",
                    "description": "Task IDs that block this task",
                    "items": { "type": "string" }
                },
                "metadata": {
                    "type": "object",
                    "description": "Metadata keys to merge into the task"
                }
            },
            "required": ["taskId"]
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true // Task management doesn't need permission
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let task_id = input["taskId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'taskId' parameter"))?;

        let mut store = context.task_store.lock().await;

        // Handle deletion separately since it removes the task
        if let Some(status_str) = input["status"].as_str() {
            if status_str == "deleted" {
                if store.delete(task_id) {
                    return Ok(ToolResult::success(format!("Task #{task_id} deleted")));
                } else {
                    return Ok(ToolResult::error(format!("Task #{task_id} not found")));
                }
            }
        }

        let task = match store.get_mut(task_id) {
            Some(t) => t,
            None => return Ok(ToolResult::error(format!("Task #{task_id} not found"))),
        };

        let mut changes = Vec::new();

        if let Some(status_str) = input["status"].as_str() {
            if let Some(status) = TaskStatus::from_str(status_str) {
                task.status = status;
                changes.push(format!("status → {status_str}"));
            } else {
                return Ok(ToolResult::error(format!(
                    "Invalid status '{status_str}'. Must be: pending, in_progress, completed, or deleted"
                )));
            }
        }

        if let Some(subject) = input["subject"].as_str() {
            task.subject = subject.to_string();
            changes.push("subject".to_string());
        }

        if let Some(description) = input["description"].as_str() {
            task.description = description.to_string();
            changes.push("description".to_string());
        }

        if let Some(active_form) = input["activeForm"].as_str() {
            task.active_form = Some(active_form.to_string());
            changes.push("activeForm".to_string());
        }

        if let Some(owner) = input["owner"].as_str() {
            task.owner = Some(owner.to_string());
            changes.push("owner".to_string());
        }

        if let Some(add_blocks) = input["addBlocks"].as_array() {
            for id in add_blocks {
                if let Some(id_str) = id.as_str() {
                    if !task.blocks.contains(&id_str.to_string()) {
                        task.blocks.push(id_str.to_string());
                    }
                }
            }
            changes.push("blocks".to_string());
        }

        if let Some(add_blocked_by) = input["addBlockedBy"].as_array() {
            for id in add_blocked_by {
                if let Some(id_str) = id.as_str() {
                    if !task.blocked_by.contains(&id_str.to_string()) {
                        task.blocked_by.push(id_str.to_string());
                    }
                }
            }
            changes.push("blockedBy".to_string());
        }

        if let Some(metadata) = input["metadata"].as_object() {
            for (key, value) in metadata {
                if value.is_null() {
                    task.metadata.remove(key);
                } else {
                    task.metadata.insert(key.clone(), value.clone());
                }
            }
            changes.push("metadata".to_string());
        }

        if changes.is_empty() {
            return Ok(ToolResult::success(format!(
                "No changes applied to task #{task_id}"
            )));
        }

        Ok(ToolResult::success(format!(
            "Updated task #{task_id} {}",
            changes.join(", ")
        )))
    }
}
