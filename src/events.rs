//! Multi-consumer agent event bus built on `tokio::sync::broadcast`.
//!
//! Replaces the single-consumer crossbeam channel from `ui/bridge.rs` so that
//! multiple components — the iocraft UI, a file logger, a cost guard, hooks —
//! can all observe the same event stream. Lagged subscribers are dropped from
//! the buffer tail; lag is logged at warn level but never propagates an error.

use crate::api::types::StopReason;
use crate::ui::events::{DiffLine, PermissionDetail};
use tokio::sync::broadcast;

/// Status severity for [`AgentEvent::Status`] messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Warning,
    Error,
    Dim,
}

impl From<crate::ui::events::StatusLevel> for StatusLevel {
    fn from(v: crate::ui::events::StatusLevel) -> Self {
        match v {
            crate::ui::events::StatusLevel::Info => StatusLevel::Info,
            crate::ui::events::StatusLevel::Warning => StatusLevel::Warning,
            crate::ui::events::StatusLevel::Error => StatusLevel::Error,
            crate::ui::events::StatusLevel::Dim => StatusLevel::Dim,
        }
    }
}

impl From<StatusLevel> for crate::ui::events::StatusLevel {
    fn from(v: StatusLevel) -> Self {
        match v {
            StatusLevel::Info => crate::ui::events::StatusLevel::Info,
            StatusLevel::Warning => crate::ui::events::StatusLevel::Warning,
            StatusLevel::Error => crate::ui::events::StatusLevel::Error,
            StatusLevel::Dim => crate::ui::events::StatusLevel::Dim,
        }
    }
}

/// Canonical agent event. Cloneable so the broadcast channel can fan it out.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Inference about to start (pre-token).
    InferenceStart,
    /// Streaming text delta from the model.
    TextDelta(String),
    /// Streaming thinking delta from the model (if supported).
    ThinkingDelta(String),
    /// A tool call was detected in the stream.
    ToolUseStart { id: String, name: String },
    /// Streaming ended.
    StreamEnd,

    /// A tool began executing.
    ToolStart {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// A tool finished executing.
    ToolComplete {
        id: String,
        name: String,
        preview: String,
        is_error: bool,
        duration_ms: u64,
    },

    /// Turn lifecycle.
    TurnStart { turn: u32 },
    TurnEnd { turn: u32, stop_reason: StopReason },

    /// Cost update after a completed API call.
    CostUpdate {
        usd: f64,
        input_tokens: u64,
        output_tokens: u64,
    },

    /// Generic status message (verbose/info/warn/error).
    Status { level: StatusLevel, message: String },

    /// Model info (which model is about to be used).
    ModelInfo(String),

    /// Background task finished.
    BackgroundTaskCompleted {
        id: String,
        status: String,
        duration_secs: f64,
    },

    /// Permission prompt request. Carries a correlation id for the
    /// caller to await a response on.
    PermissionRequest {
        id: String,
        detail: PermissionDetail,
    },

    /// Compact diff block for UI preview.
    DiffPreview { lines: Vec<DiffLine> },

    /// Engine is shutting down.
    Exit,
}

/// Broadcast-backed event bus. Clone-able handle.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AgentEvent>,
}

impl EventBus {
    /// Create a new bus with the given buffer capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publish an event. Ignores errors from lagged/dropped subscribers.
    pub fn publish(&self, event: AgentEvent) {
        // Error here means no active receivers — that's fine, events just get dropped.
        let _ = self.tx.send(event);
    }

    /// Subscribe to the event stream.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.tx.subscribe()
    }

    // ── Convenience helpers ──────────────────────────────────────────────
    pub fn status(&self, level: StatusLevel, msg: impl Into<String>) {
        self.publish(AgentEvent::Status {
            level,
            message: msg.into(),
        });
    }

    pub fn info(&self, msg: impl Into<String>) {
        self.status(StatusLevel::Info, msg);
    }

    pub fn warn(&self, msg: impl Into<String>) {
        self.status(StatusLevel::Warning, msg);
    }

    pub fn error(&self, msg: impl Into<String>) {
        self.status(StatusLevel::Error, msg);
    }

    pub fn dim(&self, msg: impl Into<String>) {
        self.status(StatusLevel::Dim, msg);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        // 512 is plenty for realtime agent UI throughput. Lagged receivers
        // drop oldest events but never block the publisher.
        Self::new(512)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn multi_subscriber_receives_same_event() {
        let bus = EventBus::new(16);
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();

        bus.info("hello");

        let ea = a.recv().await.unwrap();
        let eb = b.recv().await.unwrap();
        match (ea, eb) {
            (AgentEvent::Status { message: m1, .. }, AgentEvent::Status { message: m2, .. }) => {
                assert_eq!(m1, "hello");
                assert_eq!(m2, "hello");
            }
            _ => panic!("unexpected event"),
        }
    }
}
