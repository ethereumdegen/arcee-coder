use crate::tools::task_store::TaskStatus;
use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct TaskUpdateTool;

const DESCRIPTION: &str = "Use this tool to update a task in the task list.\n\n\
## When to Use This Tool\n\n\
**Mark tasks as resolved:**\n\
- When you have completed the work described in a task\n\
- When a task is no longer needed or has been superseded\n\
- IMPORTANT: Always mark your assigned tasks as resolved when you finish them\n\
- After resolving, call TaskList to find your next task\n\n\
- ONLY mark a task as completed when you have FULLY accomplished it\n\
- If you encounter errors, blockers, or cannot finish, keep the task as in_progress\n\
- When blocked, create a new task describing what needs to be resolved\n\
- Never mark a task as completed if:\n\
  - Tests are failing\n\
  - Implementation is partial\n\
  - You encountered unresolved errors\n\
  - You couldn't find necessary files or dependencies\n\n\
**Delete tasks:**\n\
- When a task is no longer relevant or was created in error\n\
- Setting status to `deleted` permanently removes the task\n\n\
**Update task details:**\n\
- When requirements change or become clearer\n\
- When establishing dependencies between tasks\n\n\
## Fields You Can Update\n\n\
- **status**: The task status (see Status Workflow below)\n\
- **subject**: Change the task title (imperative form, e.g., \"Run tests\")\n\
- **description**: Change the task description\n\
- **activeForm**: Present continuous form shown in spinner when in_progress (e.g., \"Running tests\")\n\
- **owner**: Change the task owner (agent name)\n\
- **metadata**: Merge metadata keys into the task (set a key to null to delete it)\n\
- **addBlocks**: Mark tasks that cannot start until this one completes\n\
- **addBlockedBy**: Mark tasks that must complete before this one can start\n\n\
## Status Workflow\n\n\
Status progresses: `pending` → `in_progress` → `completed`\n\n\
Use `deleted` to permanently remove a task.\n\n\
## Staleness\n\n\
Make sure to read a task's latest state using `TaskGet` before updating it.";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "taskId": { "type": "string", "description": "The ID of the task to update" },
                "status": {
                    "type": "string",
                    "description": "New status",
                    "enum": ["pending", "in_progress", "completed", "deleted"]
                },
                "subject": { "type": "string", "description": "New subject/title" },
                "description": { "type": "string", "description": "New description" },
                "activeForm": { "type": "string", "description": "Present continuous form" },
                "owner": { "type": "string", "description": "New owner" },
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
                    "description": "Metadata keys to merge (null to delete a key)"
                }
            },
            "required": ["taskId"]
        })
    })
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &'static str {
        "TaskUpdate"
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

        let mut store = context.task_store.lock().await;

        // Handle deletion separately since it removes the task
        if let Some(status_str) = input["status"].as_str() {
            if status_str == "deleted" {
                if store.delete(task_id) {
                    return Ok(ToolOutput::success()
                        .with_summary(format!("Task #{task_id} deleted"))
                        .with_text(format!("Task #{task_id} was removed.")));
                } else {
                    return Ok(ToolOutput::error(format!("Task #{task_id} not found")));
                }
            }
        }

        let task = match store.get_mut(task_id) {
            Some(t) => t,
            None => return Ok(ToolOutput::error(format!("Task #{task_id} not found"))),
        };

        let mut changes = Vec::new();

        if let Some(status_str) = input["status"].as_str() {
            if let Some(status) = TaskStatus::from_str(status_str) {
                task.status = status;
                changes.push(format!("status → {status_str}"));
            } else {
                return Ok(ToolOutput::error(format!(
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
            return Ok(ToolOutput::empty(format!(
                "No changes applied to task #{task_id}"
            )));
        }

        Ok(ToolOutput::success()
            .with_summary(format!("Task #{task_id} updated"))
            .with_text(format!(
                "Updated task #{task_id}: {}",
                changes.join(", ")
            )))
    }
}
