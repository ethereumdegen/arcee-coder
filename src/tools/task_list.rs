use crate::toon::ToonValue;
use crate::tools::{PermissionClass, Tool, ToolBody, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct TaskListTool;

const DESCRIPTION: &str = "Use this tool to list all tasks in the task list.\n\n\
## When to Use This Tool\n\n\
- To see what tasks are available to work on (status: 'pending', no owner, not blocked)\n\
- To check overall progress on the project\n\
- To find tasks that are blocked and need dependencies resolved\n\
- After completing a task, to check for newly unblocked work or claim the next available task\n\
- **Prefer working on tasks in ID order** (lowest ID first) when multiple tasks are available, as earlier tasks often set up context for later ones\n\n\
## Output\n\n\
Returns a summary of each task:\n\
- **id**: Task identifier (use with TaskGet, TaskUpdate)\n\
- **subject**: Brief description of the task\n\
- **status**: 'pending', 'in_progress', or 'completed'\n\
- **owner**: Agent ID if assigned, empty if available\n\
- **blockedBy**: List of open task IDs that must be resolved first (tasks with blockedBy cannot be claimed until dependencies resolve)\n\n\
Use TaskGet with a specific task ID to view full details including description and comments.";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {}
        })
    })
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &'static str {
        "TaskList"
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

    async fn call(&self, _input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let store = context.task_store.lock().await;
        let tasks = store.list();

        if tasks.is_empty() {
            return Ok(ToolOutput::empty("No tasks found.")
                .with_summary("0 tasks")
                .with_next_step("Use TaskCreate to add a task"));
        }

        let mut pending = 0usize;
        let mut in_progress = 0usize;
        let mut completed = 0usize;

        let mut rows: Vec<Vec<String>> = Vec::with_capacity(tasks.len());
        for t in &tasks {
            match t.status.as_str() {
                "pending" => pending += 1,
                "in_progress" => in_progress += 1,
                "completed" => completed += 1,
                _ => {}
            }
            let blocked_by = if t.blocked_by.is_empty() {
                String::new()
            } else {
                t.blocked_by.join(",")
            };
            rows.push(vec![
                t.id.clone(),
                t.status.as_str().to_string(),
                t.subject.clone(),
                t.owner.clone().unwrap_or_default(),
                blocked_by,
            ]);
        }

        let summary = format!(
            "{} pending, {} in_progress, {} completed ({} total)",
            pending,
            in_progress,
            completed,
            tasks.len()
        );

        let body = ToolBody::Toon(ToonValue::Map(vec![(
            "tasks".into(),
            ToonValue::Table {
                columns: vec![
                    "id".into(),
                    "status".into(),
                    "subject".into(),
                    "owner".into(),
                    "blocked_by".into(),
                ],
                rows,
            },
        )]));

        Ok(ToolOutput::success()
            .with_summary(summary)
            .with_body(body)
            .with_next_step("Use TaskGet for full details on a specific task"))
    }
}
