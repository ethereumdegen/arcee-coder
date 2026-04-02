/// Event types for channel communication between the tokio main thread and
/// the smol/iocraft UI thread.

/// Events sent from the main (tokio) thread to the UI (smol/iocraft) thread.
#[derive(Debug, Clone)]
pub enum UiEvent {
    // --- Streaming ---
    /// A chunk of streamed text from the API.
    StreamText(String),
    /// The model started invoking a tool during streaming.
    StreamToolStart { id: String, name: String },
    /// Streaming finished (model returned end_turn or all tool calls done).
    StreamEnd,

    // --- Tool execution ---
    /// A tool started executing.
    ToolExecStart { name: String },
    /// A tool finished executing.
    ToolResult {
        name: String,
        preview: String,
        is_error: bool,
        duration_ms: u64,
    },

    // --- Background tasks ---
    BackgroundTaskStarted { id: String, description: String },
    BackgroundTaskCompleted {
        id: String,
        status: String,
        duration_secs: f64,
    },

    // --- Status ---
    ModelInfo(String),
    CostUpdate {
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
    },
    TurnInfo {
        turn: usize,
        max_turns: usize,
    },

    // --- Control ---
    /// Show a permission prompt to the user.
    PermissionPrompt {
        tool: String,
        description: String,
    },
    /// A system/status message (warnings, info, verbose messages).
    StatusMessage {
        text: String,
        level: StatusLevel,
    },
    /// Request the UI to show the input prompt (ready for next user input).
    ShowPrompt,
    /// Request the UI to exit.
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Warning,
    Error,
    Dim,
}

/// Commands sent from the UI (smol/iocraft) thread back to the main (tokio) thread.
#[derive(Debug, Clone)]
pub enum UiCommand {
    /// User submitted a line of input.
    UserInput(String),
    /// User pressed ESC to interrupt.
    EscapePressed,
    /// User responded to a permission prompt.
    PermissionResponse(bool),
}
