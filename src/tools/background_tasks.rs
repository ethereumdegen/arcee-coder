use std::collections::HashMap;
use std::time::Instant;

/// Status of a background task.
#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundTaskStatus {
    Running,
    Completed,
    Failed,
}

impl BackgroundTaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

/// A background agent task.
#[derive(Debug, Clone)]
pub struct BackgroundTask {
    pub id: String,
    pub description: String,
    pub agent_type: String,
    pub status: BackgroundTaskStatus,
    pub result: Option<String>,
    pub started_at: Instant,
    pub completed_at: Option<Instant>,
}

impl BackgroundTask {
    pub fn format_summary(&self) -> String {
        let elapsed = match self.completed_at {
            Some(end) => end.duration_since(self.started_at),
            None => self.started_at.elapsed(),
        };
        format!(
            "#{} [{}] {} ({:.1}s)",
            self.id,
            self.status.as_str(),
            self.description,
            elapsed.as_secs_f64()
        )
    }
}

/// Store for tracking background agent tasks.
pub struct BackgroundTaskStore {
    tasks: HashMap<String, BackgroundTask>,
    next_id: u64,
    /// IDs of tasks that completed but haven't been notified to the model yet.
    pub completed_queue: Vec<String>,
}

impl BackgroundTaskStore {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_id: 1,
            completed_queue: Vec::new(),
        }
    }

    /// Register a new background task. Returns the task ID.
    pub fn register(&mut self, description: String, agent_type: String) -> String {
        let id = self.next_id.to_string();
        self.next_id += 1;

        let task = BackgroundTask {
            id: id.clone(),
            description,
            agent_type,
            status: BackgroundTaskStatus::Running,
            result: None,
            started_at: Instant::now(),
            completed_at: None,
        };

        self.tasks.insert(id.clone(), task);
        id
    }

    /// Mark a task as completed with a result.
    pub fn complete(&mut self, id: &str, result: String) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.status = BackgroundTaskStatus::Completed;
            task.result = Some(result);
            task.completed_at = Some(Instant::now());
            self.completed_queue.push(id.to_string());
        }
    }

    /// Mark a task as failed with an error message.
    pub fn fail(&mut self, id: &str, error: String) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.status = BackgroundTaskStatus::Failed;
            task.result = Some(error);
            task.completed_at = Some(Instant::now());
            self.completed_queue.push(id.to_string());
        }
    }

    /// Get a task by ID.
    pub fn get(&self, id: &str) -> Option<&BackgroundTask> {
        self.tasks.get(id)
    }

    /// List all tasks.
    pub fn list(&self) -> Vec<&BackgroundTask> {
        let mut tasks: Vec<_> = self.tasks.values().collect();
        tasks.sort_by_key(|t| &t.id);
        tasks
    }

    /// Drain the completed notification queue. Returns tasks that need notification.
    pub fn drain_completed(&mut self) -> Vec<BackgroundTask> {
        let ids: Vec<String> = self.completed_queue.drain(..).collect();
        ids.iter()
            .filter_map(|id| self.tasks.get(id).cloned())
            .collect()
    }
}
