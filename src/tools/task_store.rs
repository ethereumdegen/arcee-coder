use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Deleted,
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TaskStatus::Pending),
            "in_progress" => Some(TaskStatus::InProgress),
            "completed" => Some(TaskStatus::Completed),
            "deleted" => Some(TaskStatus::Deleted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub status: TaskStatus,
    pub active_form: Option<String>,
    pub owner: Option<String>,
    pub blocks: Vec<String>,
    pub blocked_by: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Task {
    pub fn format_summary(&self) -> String {
        let blocked = if self.blocked_by.is_empty() {
            String::new()
        } else {
            format!(" (blocked by: {})", self.blocked_by.join(", "))
        };
        let owner = self.owner.as_deref().unwrap_or("");
        let owner_str = if owner.is_empty() {
            String::new()
        } else {
            format!(" [{}]", owner)
        };
        format!(
            "#{id}. [{status}] {subject}{owner}{blocked}",
            id = self.id,
            status = self.status.as_str(),
            subject = self.subject,
            owner = owner_str,
            blocked = blocked,
        )
    }

    pub fn format_detail(&self) -> String {
        let mut out = format!(
            "Task #{}\nSubject: {}\nStatus: {}\nDescription: {}",
            self.id,
            self.subject,
            self.status.as_str(),
            self.description,
        );
        if let Some(ref af) = self.active_form {
            out.push_str(&format!("\nActive Form: {af}"));
        }
        if let Some(ref owner) = self.owner {
            out.push_str(&format!("\nOwner: {owner}"));
        }
        if !self.blocks.is_empty() {
            out.push_str(&format!("\nBlocks: {}", self.blocks.join(", ")));
        }
        if !self.blocked_by.is_empty() {
            out.push_str(&format!("\nBlocked By: {}", self.blocked_by.join(", ")));
        }
        if !self.metadata.is_empty() {
            out.push_str(&format!(
                "\nMetadata: {}",
                serde_json::to_string(&self.metadata).unwrap_or_default()
            ));
        }
        out
    }
}

pub struct TaskStore {
    tasks: Vec<Task>,
    next_id: u64,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    pub fn create(
        &mut self,
        subject: String,
        description: String,
        active_form: Option<String>,
    ) -> &Task {
        let id = self.next_id.to_string();
        self.next_id += 1;
        let task = Task {
            id,
            subject,
            description,
            status: TaskStatus::Pending,
            active_form,
            owner: None,
            blocks: Vec::new(),
            blocked_by: Vec::new(),
            metadata: HashMap::new(),
        };
        self.tasks.push(task);
        self.tasks.last().unwrap()
    }

    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|t| t.id == id && t.status != TaskStatus::Deleted)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks
            .iter_mut()
            .find(|t| t.id == id && t.status != TaskStatus::Deleted)
    }

    pub fn list(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.status != TaskStatus::Deleted)
            .collect()
    }

    /// Delete a task (mark as deleted) and clean up dangling references.
    pub fn delete(&mut self, id: &str) -> bool {
        let found = self.tasks.iter().any(|t| t.id == id);
        if !found {
            return false;
        }

        // Mark the task as deleted
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.status = TaskStatus::Deleted;
        }

        // Remove this task from other tasks' blocks/blocked_by lists
        let id_owned = id.to_string();
        for task in &mut self.tasks {
            if task.status != TaskStatus::Deleted {
                task.blocks.retain(|b| b != &id_owned);
                task.blocked_by.retain(|b| b != &id_owned);
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_list() {
        let mut store = TaskStore::new();
        store.create("Task 1".into(), "Desc 1".into(), None);
        store.create("Task 2".into(), "Desc 2".into(), Some("Working".into()));

        let list = store.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "1");
        assert_eq!(list[1].id, "2");
        assert_eq!(list[1].active_form.as_deref(), Some("Working"));
    }

    #[test]
    fn test_get_and_update() {
        let mut store = TaskStore::new();
        store.create("Task 1".into(), "Desc 1".into(), None);

        let task = store.get_mut("1").unwrap();
        task.status = TaskStatus::InProgress;
        task.owner = Some("agent-1".into());

        let task = store.get("1").unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.owner.as_deref(), Some("agent-1"));
    }

    #[test]
    fn test_delete() {
        let mut store = TaskStore::new();
        store.create("Task 1".into(), "Desc 1".into(), None);
        assert!(store.delete("1"));
        assert!(store.get("1").is_none());
        assert!(store.list().is_empty());
    }

    #[test]
    fn test_delete_cleans_up_references() {
        let mut store = TaskStore::new();
        store.create("Task 1".into(), "Desc 1".into(), None);
        store.create("Task 2".into(), "Desc 2".into(), None);
        store.create("Task 3".into(), "Desc 3".into(), None);

        // Task 2 blocks task 3, task 3 is blocked by task 2
        let t2 = store.get_mut("2").unwrap();
        t2.blocks.push("3".to_string());
        let t3 = store.get_mut("3").unwrap();
        t3.blocked_by.push("2".to_string());

        // Delete task 2
        store.delete("2");

        // Task 3 should no longer be blocked by task 2
        let t3 = store.get("3").unwrap();
        assert!(t3.blocked_by.is_empty());
    }

    #[test]
    fn test_task_status_roundtrip() {
        for status in &[
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::Completed,
            TaskStatus::Deleted,
        ] {
            assert_eq!(TaskStatus::from_str(status.as_str()).as_ref(), Some(status));
        }
    }
}
