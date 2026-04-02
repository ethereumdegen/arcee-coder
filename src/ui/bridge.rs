/// Bridge between the tokio main thread and the smol/iocraft UI thread.
///
/// Uses crossbeam-channel for thread-safe, runtime-agnostic communication.

use crate::ui::events::{StatusLevel, UiCommand, UiEvent};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Handle used by the main (tokio) thread to send events to the UI and
/// receive commands back.
#[derive(Clone)]
pub struct UiBridge {
    event_tx: Sender<UiEvent>,
    command_rx: Receiver<UiCommand>,
}

impl UiBridge {
    /// Create a new bridge pair: (main-side handle, ui-side handle).
    pub fn new(escape_flag: Arc<AtomicBool>) -> (Self, UiHandle) {
        let (event_tx, event_rx) = crossbeam_channel::unbounded();
        let (command_tx, command_rx) = crossbeam_channel::unbounded();

        let bridge = UiBridge {
            event_tx,
            command_rx,
        };

        let handle = UiHandle {
            event_rx,
            command_tx,
            escape_flag,
        };

        (bridge, handle)
    }

    /// Send a UI event (non-blocking).
    pub fn send(&self, event: UiEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Send streamed text to the UI.
    pub fn stream_text(&self, text: &str) {
        self.send(UiEvent::StreamText(text.to_string()));
    }

    /// Notify UI that a tool call started during streaming.
    pub fn stream_tool_start(&self, id: &str, name: &str) {
        self.send(UiEvent::StreamToolStart {
            id: id.to_string(),
            name: name.to_string(),
        });
    }

    /// Notify UI that streaming ended.
    pub fn stream_end(&self) {
        self.send(UiEvent::StreamEnd);
    }

    /// Notify UI that a tool started executing.
    pub fn tool_exec_start(&self, name: &str) {
        self.send(UiEvent::ToolExecStart {
            name: name.to_string(),
        });
    }

    /// Notify UI of a tool result.
    pub fn tool_result(&self, name: &str, preview: &str, is_error: bool, duration_ms: u64) {
        self.send(UiEvent::ToolResult {
            name: name.to_string(),
            preview: preview.to_string(),
            is_error,
            duration_ms,
        });
    }

    /// Send a status/info message.
    pub fn status(&self, text: &str, level: StatusLevel) {
        self.send(UiEvent::StatusMessage {
            text: text.to_string(),
            level,
        });
    }

    /// Send cost update.
    pub fn cost_update(&self, input_tokens: u64, output_tokens: u64, cost_usd: f64) {
        self.send(UiEvent::CostUpdate {
            input_tokens,
            output_tokens,
            cost_usd,
        });
    }

    /// Send turn info.
    pub fn turn_info(&self, turn: usize, max_turns: usize) {
        self.send(UiEvent::TurnInfo { turn, max_turns });
    }

    /// Send model info.
    pub fn model_info(&self, model: &str) {
        self.send(UiEvent::ModelInfo(model.to_string()));
    }

    /// Try to receive a command from the UI (non-blocking).
    pub fn try_recv_command(&self) -> Option<UiCommand> {
        match self.command_rx.try_recv() {
            Ok(cmd) => Some(cmd),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Check if ESC was pressed (drains the command queue looking for EscapePressed).
    pub fn check_escape(&self) -> bool {
        // Non-destructively check — but crossbeam doesn't support peek,
        // so we drain and re-check. Since ESC is an immediate action,
        // we just check try_recv.
        match self.command_rx.try_recv() {
            Ok(UiCommand::EscapePressed) => true,
            Ok(other) => {
                // Put it back? No, crossbeam doesn't support that.
                // For now, non-escape commands during streaming are dropped.
                // This is acceptable since user input only matters at the prompt.
                drop(other);
                false
            }
            Err(_) => false,
        }
    }

    /// Request the UI to show the input prompt.
    pub fn show_prompt(&self) {
        self.send(UiEvent::ShowPrompt);
    }

    /// Request the UI to exit.
    pub fn request_exit(&self) {
        self.send(UiEvent::Exit);
    }

    /// Block until we receive a command from the UI thread.
    /// Used for waiting for user input at the prompt.
    pub fn recv_command(&self) -> Option<UiCommand> {
        self.command_rx.recv().ok()
    }

    /// Send a permission prompt to the UI and block until the user responds.
    pub fn prompt_permission(&self, tool: &str, description: &str) -> bool {
        self.send(UiEvent::PermissionPrompt {
            tool: tool.to_string(),
            description: description.to_string(),
        });
        // Block waiting for the permission response
        loop {
            match self.command_rx.recv() {
                Ok(UiCommand::PermissionResponse(allowed)) => return allowed,
                Ok(_) => continue, // ignore other commands while waiting
                Err(_) => return false, // channel closed
            }
        }
    }
}

/// Handle used by the UI (smol/iocraft) thread to receive events and send
/// commands back to the main thread.
#[derive(Clone)]
pub struct UiHandle {
    pub event_rx: Receiver<UiEvent>,
    pub command_tx: Sender<UiCommand>,
    pub escape_flag: Arc<AtomicBool>,
}

impl Default for UiHandle {
    fn default() -> Self {
        let (_, event_rx) = crossbeam_channel::unbounded();
        let (command_tx, _) = crossbeam_channel::unbounded();
        Self {
            event_rx,
            command_tx,
            escape_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl UiHandle {
    /// Try to receive an event (non-blocking).
    pub fn try_recv_event(&self) -> Option<UiEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Send a command back to the main thread.
    pub fn send_command(&self, cmd: UiCommand) {
        let _ = self.command_tx.send(cmd);
    }
}
