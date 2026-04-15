use crate::config::paths;
use crate::messages::types::Message;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub cwd: PathBuf,
    pub model: String,
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

impl Session {
    pub fn new(cwd: PathBuf, model: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            cwd,
            model,
            total_cost_usd: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
        }
    }

    fn file_path(&self) -> PathBuf {
        paths::sessions_dir().join(format!("{}.json", self.id))
    }

    pub fn save(&self) -> Result<()> {
        paths::ensure_dirs()?;
        let content = serde_json::to_string_pretty(self)?;
        let path = self.file_path();
        // Atomic write: write to temp file then rename (POSIX atomic).
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &content)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn load(id: &str) -> Result<Self> {
        let path = paths::sessions_dir().join(format!("{id}.json"));
        let content = std::fs::read_to_string(&path)?;
        let session: Self = serde_json::from_str(&content)?;
        Ok(session)
    }

    /// Load the most recent session.
    pub fn load_latest() -> Result<Option<Self>> {
        let sessions_dir = paths::sessions_dir();
        if !sessions_dir.exists() {
            return Ok(None);
        }

        let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;

        for entry in std::fs::read_dir(&sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let modified = entry.metadata()?.modified()?;
                if latest.as_ref().is_none_or(|(t, _)| modified > *t) {
                    latest = Some((modified, path));
                }
            }
        }

        if let Some((_, path)) = latest {
            let content = std::fs::read_to_string(&path)?;
            let session: Self = serde_json::from_str(&content)?;
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }
}
