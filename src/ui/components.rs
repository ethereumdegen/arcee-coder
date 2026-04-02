/// iocraft UI components for the terminal interface.

use crate::ui::bridge::UiHandle;
use crate::ui::events::{StatusLevel, UiCommand, UiEvent};
use iocraft::prelude::*;
use std::time::{Duration, Instant};

const SPINNER_FRAMES: &[&str] = &[
    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",
];
fn rain_frames() -> &'static Vec<String> {
    use std::sync::OnceLock;
    static FRAMES: OnceLock<Vec<String>> = OnceLock::new();
    FRAMES.get_or_init(|| {
        crate::ui::thinking::gen_rain_frames()
            .into_iter()
            .map(|braille| format!("thinking {braille}"))
            .collect()
    })
}

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/help", "Show available commands"),
    ("/clear", "Clear the conversation"),
    ("/compact", "Compress conversation context"),
    ("/cost", "Show token usage and cost"),
    ("/model", "Show or switch model"),
    ("/intensity", "Set model routing intensity"),
    ("/strictness", "Set permission strictness"),
    ("/tokens", "Show estimated token count"),
    ("/history", "Show conversation summary"),
    ("/quit", "Exit arcee-code"),
];

const MAX_PERSISTED_HISTORY: usize = 50;

/// Load prompt history from ~/.arcee/history (one entry per line).
fn load_prompt_history() -> Vec<String> {
    let path = crate::config::paths::history_file();
    match std::fs::read_to_string(&path) {
        Ok(content) => content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Append a single entry to the history file, keeping at most MAX_PERSISTED_HISTORY entries.
fn save_prompt_history(history: &[String]) {
    let path = crate::config::paths::history_file();
    // Keep only the last MAX_PERSISTED_HISTORY entries
    let start = history.len().saturating_sub(MAX_PERSISTED_HISTORY);
    let lines = history[start..].join("\n");
    let _ = std::fs::write(&path, format!("{lines}\n"));
}

/// Shared state updated by polling UiEvents.
/// Non-Copy, so we use State<AppState> with .read() / .write().
pub struct AppState {
    pub is_streaming: bool,
    pub has_streamed_text: bool,
    pub active_tool_name: String,
    pub active_tool_start: Option<Instant>,
    pub bg_display_lines: Vec<String>,
    pub model: String,
    pub turn: usize,
    pub max_turns: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub waiting_for_input: bool,
    pub input_buffer: String,
    pub input_history: Vec<String>,
    pub history_index: Option<usize>, // None = editing new input, Some(i) = browsing history[i]
    pub history_stash: String,        // saves current input when browsing history
    pub show_slash_menu: bool,
    pub perm_prompt: Option<super::events::PermissionDetail>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            input_history: load_prompt_history(),
            is_streaming: false,
            has_streamed_text: false,
            active_tool_name: String::new(),
            active_tool_start: None,
            bg_display_lines: Vec::new(),
            model: String::new(),
            turn: 0,
            max_turns: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: 0.0,
            waiting_for_input: false,
            input_buffer: String::new(),
            history_index: None,
            history_stash: String::new(),
            show_slash_menu: false,
            perm_prompt: None,
        }
    }
}

// ─── Event processing helper ──────────────────────────────────────────────

/// Write text directly to stdout, bypassing iocraft's print pipeline.
/// This avoids iocraft's per-render `cursor::position()` round-trip and
/// canvas clear+redraw overhead during streaming.
fn raw_stdout_write(text: &str) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
}

/// Process a single UI event.
///
/// `text_accum`: if `Some(&mut buf)`, StreamText content is appended to
/// the buffer for batched flushing.  If `None`, the token is written to
/// stdout immediately.
fn handle_event(
    event: UiEvent,
    is_streaming_text: &mut bool,
    is_showing_thinking: &mut bool,
    state: &mut State<AppState>,
    should_exit: &mut State<bool>,
    text_accum: &mut Option<&mut String>,
) {
    match event {
        // ── Streaming text: direct stdout (bypasses iocraft) ──────
        UiEvent::StreamText(text) => {
            if !*is_streaming_text {
                // Clear thinking animation line before first token
                if *is_showing_thinking {
                    raw_stdout_write("\r\x1b[K");
                    *is_showing_thinking = false;
                }
                // First token — tell iocraft we're streaming so the component
                // shows nothing (minimal canvas).
                let mut s = state.write();
                s.is_streaming = true;
                s.has_streamed_text = true;
                *is_streaming_text = true;
            }
            let converted = text.replace('\n', "\r\n");
            match text_accum {
                Some(buf) => buf.push_str(&converted),
                None => raw_stdout_write(&converted),
            }
        }

        // ── Other print events: also go direct to stdout ──────────
        UiEvent::StreamToolStart { name, .. } => {
            // Clear thinking animation if it was showing
            if *is_showing_thinking {
                raw_stdout_write("\r\x1b[K");
                *is_showing_thinking = false;
            }
            *is_streaming_text = true;
            raw_stdout_write(&format!(
                "\r\n\x1b[1;36mTool:\x1b[0m \x1b[36m{name}\x1b[0m\r\n"
            ));
        }
        UiEvent::StatusMessage { text, level } => {
            let colored = match level {
                StatusLevel::Info => text,
                StatusLevel::Warning => format!("\x1b[33m{text}\x1b[0m"),
                StatusLevel::Error => format!("\x1b[31m{text}\x1b[0m"),
                StatusLevel::Dim => format!("\x1b[2m{text}\x1b[0m"),
            };
            // Replace bare \n with \r\n for raw terminal mode
            let colored = colored.replace('\n', "\r\n");
            raw_stdout_write(&format!("{colored}\r\n"));
        }
        UiEvent::ToolResult {
            preview,
            is_error,
            duration_ms,
            ..
        } => {
            {
                let mut s = state.write();
                s.active_tool_name.clear();
                s.active_tool_start = None;
            }
            if is_error {
                raw_stdout_write(&format!("\x1b[31m  Error: {preview}\x1b[0m\r\n"));
            } else {
                raw_stdout_write(&format!("\x1b[2m  {preview}\x1b[0m\r\n"));
            }
            if duration_ms > 100 {
                raw_stdout_write(&format!("\x1b[2m  ({duration_ms}ms)\x1b[0m\r\n"));
            }
        }

        // ── State-mutating events ─────────────────────────────────
        UiEvent::InferenceStart => {
            let mut s = state.write();
            s.is_streaming = true;
            s.has_streamed_text = false;
            // Note: is_streaming_text stays false until first StreamText arrives.
        }
        UiEvent::StreamEnd => {
            // Clear thinking animation if it was still showing
            if *is_showing_thinking {
                raw_stdout_write("\r\x1b[K");
                *is_showing_thinking = false;
            }
            let mut s = state.write();
            s.is_streaming = false;
            s.has_streamed_text = false;
            *is_streaming_text = false;
            s.active_tool_name.clear();
            s.active_tool_start = None;
            // Write a newline to separate streamed text from whatever comes next.
            raw_stdout_write("\r\n");
        }
        UiEvent::ToolExecStart { name } => {
            let mut s = state.write();
            s.active_tool_name = name;
            s.active_tool_start = Some(Instant::now());
        }
        UiEvent::BackgroundTaskStarted { id, description } => {
            state.write().bg_display_lines
                .push(format!("  #{id} ⠙ {description}..."));
        }
        UiEvent::BackgroundTaskCompleted {
            id,
            status,
            duration_secs,
        } => {
            let mut s = state.write();
            if let Some(line) = s
                .bg_display_lines
                .iter_mut()
                .find(|l| l.contains(&format!("#{id} ")))
            {
                *line = format!("  #{id} ✓ [{status}] ({duration_secs:.1}s)");
            }
            if s.bg_display_lines.len() > 10 {
                let all_completed = s.bg_display_lines.iter().all(|l| l.contains('✓'));
                if all_completed {
                    let drain_count = s.bg_display_lines.len() - 5;
                    s.bg_display_lines.drain(..drain_count);
                }
            }
        }
        UiEvent::ModelInfo(model) => {
            state.write().model = model;
        }
        UiEvent::CostUpdate {
            input_tokens,
            output_tokens,
            cost_usd,
        } => {
            let mut s = state.write();
            s.input_tokens = input_tokens;
            s.output_tokens = output_tokens;
            s.cost_usd = cost_usd;
        }
        UiEvent::TurnInfo { turn, max_turns } => {
            let mut s = state.write();
            s.turn = turn;
            s.max_turns = max_turns;
        }
        UiEvent::ShowPrompt => {
            let mut s = state.write();
            s.waiting_for_input = true;
            s.input_buffer.clear();
        }
        UiEvent::PermissionPrompt(detail) => {
            state.write().perm_prompt = Some(detail);
        }
        UiEvent::Exit => {
            should_exit.set(true);
        }
    }
}

// ─── Root App component ────────────────────────────────────────────────────

#[derive(Default, Props)]
pub struct AppProps {
    pub ui_handle: UiHandle,
}

#[component]
pub fn App(props: &AppProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    // Note: we intentionally do NOT use hooks.use_output() for streaming text.
    // iocraft's print pipeline triggers cursor::position() + canvas clear+redraw
    // on every flush, which is too expensive for per-token streaming.
    // Instead, streaming text goes directly to stdout via raw_stdout_write().

    let mut state: State<AppState> = hooks.use_state(AppState::default);
    let mut spinner_frame = hooks.use_state(|| 0usize);
    let mut should_exit = hooks.use_state(|| false);

    // Process events from the main thread via async channel.
    //
    // Streaming text bypasses iocraft entirely (direct stdout writes) to avoid
    // iocraft's per-render cursor::position() round-trip and canvas
    // clear+redraw overhead.  Everything else uses iocraft's print pipeline.
    let handle = props.ui_handle.clone();
    hooks.use_future(async move {
        let mut is_streaming_text = false;
        let mut needs_animation = false;
        let mut is_showing_thinking = false; // tracks if thinking line is on screen
        let mut thinking_frame_idx: usize = 0;

        loop {
            // ── Wait for the next event ──────────────────────────────
            // When actively streaming text, use a short timer to batch tokens.
            // When animating (thinking/tool spinner), use a ~100ms timer so
            // the spinner advances even when no events arrive.
            // Otherwise, block on the async channel (zero CPU).
            if is_streaming_text {
                smol::Timer::after(Duration::from_millis(8)).await;
            } else if needs_animation {
                // Tick at ~10 fps so spinner animates even with no events
                smol::Timer::after(Duration::from_millis(100)).await;
            } else {
                match handle.recv_event().await {
                    Some(ev) => {
                        // Process this first event immediately (see drain below).
                        handle_event(
                            ev,
                            &mut is_streaming_text,
                            &mut is_showing_thinking,
                            &mut state,
                            &mut should_exit,
                            &mut None,
                        );
                    }
                    None => return, // channel closed
                }
            }

            // Drain everything that accumulated and batch StreamText together.
            let mut text_buf = String::new();
            while let Some(ev) = handle.try_recv_event() {
                handle_event(
                    ev,
                    &mut is_streaming_text,
                    &mut is_showing_thinking,
                    &mut state,
                    &mut should_exit,
                    &mut Some(&mut text_buf),
                );
            }

            // Flush accumulated streaming text directly to stdout — no iocraft.
            if !text_buf.is_empty() {
                raw_stdout_write(&text_buf);
            }

            // Advance spinner only when NOT streaming text (avoids pointless
            // canvas redraws while tokens are flowing).
            if !is_streaming_text {
                spinner_frame.set(spinner_frame.get().wrapping_add(1));
            }

            // Check if we need animation ticking (tool running or waiting for first token)
            let s = state.read();
            let is_thinking = s.is_streaming && !s.has_streamed_text && s.active_tool_name.is_empty();
            needs_animation = !s.active_tool_name.is_empty() || is_thinking;
            drop(s);

            // Write thinking animation directly to stdout (bypasses iocraft)
            if is_thinking && !is_streaming_text {
                let frames = rain_frames();
                let frame = &frames[thinking_frame_idx % frames.len()];
                thinking_frame_idx = thinking_frame_idx.wrapping_add(1);
                raw_stdout_write(&format!("\r\x1b[2;3mthinking\x1b[0m \x1b[36m{frame}\x1b[0m\x1b[K"));
                is_showing_thinking = true;
            }
        }
    });

    if should_exit.get() {
        system.exit();
    }

    // Handle keyboard events
    let cmd_handle = props.ui_handle.clone();
    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, modifiers, .. }) if kind != KeyEventKind::Release => {
                // Ctrl+D exits from anywhere
                if code == KeyCode::Char('d') && modifiers.contains(KeyModifiers::CONTROL) {
                    should_exit.set(true);
                    return;
                }

                let mut s = state.write();

                // Handle permission prompt first
                if s.perm_prompt.is_some() {
                    match code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            s.perm_prompt = None;
                            drop(s);
                            cmd_handle.send_command(UiCommand::PermissionResponse(true));
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            s.perm_prompt = None;
                            drop(s);
                            cmd_handle.send_command(UiCommand::PermissionResponse(false));
                        }
                        _ => {}
                    }
                    return;
                }

                // Handle input mode
                if s.waiting_for_input {
                    match code {
                        KeyCode::Enter => {
                            let input = s.input_buffer.clone();
                            // Save non-empty input to history (in-memory + disk)
                            if !input.trim().is_empty() {
                                // Avoid consecutive duplicates
                                if s.input_history.last().map(|h| h.as_str()) != Some(input.trim()) {
                                    s.input_history.push(input.trim().to_string());
                                    save_prompt_history(&s.input_history);
                                }
                            }
                            s.input_buffer.clear();
                            s.history_index = None;
                            s.history_stash.clear();
                            s.show_slash_menu = false;
                            s.waiting_for_input = false;
                            drop(s);
                            cmd_handle.send_command(UiCommand::UserInput(input));
                        }
                        KeyCode::Up => {
                            // Navigate history (older)
                            if !s.input_history.is_empty() {
                                match s.history_index {
                                    None => {
                                        // Start browsing — stash current input
                                        s.history_stash = s.input_buffer.clone();
                                        let idx = s.input_history.len() - 1;
                                        s.history_index = Some(idx);
                                        s.input_buffer = s.input_history[idx].clone();
                                    }
                                    Some(idx) if idx > 0 => {
                                        let new_idx = idx - 1;
                                        s.history_index = Some(new_idx);
                                        s.input_buffer = s.input_history[new_idx].clone();
                                    }
                                    _ => {} // Already at oldest
                                }
                                s.show_slash_menu = s.input_buffer.starts_with('/');
                            }
                        }
                        KeyCode::Down => {
                            // Navigate history (newer)
                            if let Some(idx) = s.history_index {
                                if idx + 1 < s.input_history.len() {
                                    let new_idx = idx + 1;
                                    s.history_index = Some(new_idx);
                                    s.input_buffer = s.input_history[new_idx].clone();
                                } else {
                                    // Back to the stashed input
                                    s.history_index = None;
                                    s.input_buffer = s.history_stash.clone();
                                    s.history_stash.clear();
                                }
                                s.show_slash_menu = s.input_buffer.starts_with('/');
                            }
                        }
                        KeyCode::Char(c) => {
                            s.input_buffer.push(c);
                            s.history_index = None; // typing resets history browse
                            // Show slash menu when buffer starts with /
                            s.show_slash_menu = s.input_buffer.starts_with('/');
                        }
                        KeyCode::Backspace => {
                            s.input_buffer.pop();
                            s.show_slash_menu = s.input_buffer.starts_with('/');
                        }
                        KeyCode::Esc => {
                            // In input mode, ESC clears the buffer
                            s.input_buffer.clear();
                            s.history_index = None;
                            s.show_slash_menu = false;
                        }
                        _ => {}
                    }
                    return;
                }

                drop(s);

                // Not in input mode — check for ESC
                if code == KeyCode::Esc {
                    cmd_handle
                        .escape_flag
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    cmd_handle.send_command(UiCommand::EscapePressed);
                }
            }
            _ => {}
        }
    });

    // Render
    let s = state.read();
    let frame_idx = spinner_frame.get();
    let spinner = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];

    let tool_spinner_text = if !s.active_tool_name.is_empty() {
        let elapsed = s
            .active_tool_start
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        format!("{spinner} Running: {} ({:.1}s)", s.active_tool_name, elapsed)
    } else {
        // Thinking animation is handled via direct stdout writes in use_future,
        // bypassing iocraft to avoid canvas positioning issues.
        String::new()
    };

    let has_bg_tasks = !s.bg_display_lines.is_empty();
    let bg_lines: Vec<String> = s.bg_display_lines.clone();

    let has_perm = s.perm_prompt.is_some();
    let perm_detail = s.perm_prompt.clone();

    let has_model = !s.model.is_empty();
    let status_line = if has_model {
        format!(
            "{} │ Turn {}/{} │ ${:.4} │ {}↓ {}↑",
            s.model, s.turn, s.max_turns, s.cost_usd, s.input_tokens, s.output_tokens,
        )
    } else {
        String::new()
    };

    let is_waiting = s.waiting_for_input;
    let input_buf = s.input_buffer.clone();
    let show_slash = s.show_slash_menu && s.waiting_for_input;

    // Filter slash commands based on current input
    let slash_commands: Vec<(&str, &str)> = if show_slash {
        let filter = s.input_buffer.as_str();
        SLASH_COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(filter) || filter == "/")
            .copied()
            .collect()
    } else {
        Vec::new()
    };

    // Drop read before element! to avoid potential deadlock
    drop(s);

    // During streaming, render an empty canvas so iocraft doesn't interfere
    // with direct stdout writes.
    let is_streaming_now = !tool_spinner_text.is_empty() && !is_waiting && !has_perm;

    if is_streaming_now && !has_perm {
        // Minimal canvas: just show tool spinner or thinking indicator
        // (one line, no borders, no boxes)
        element! {
            View {
                Text(color: Color::Cyan, content: tool_spinner_text.clone())
            }
        }
    } else {
        element! {
            View(flex_direction: FlexDirection::Column) {
                // Permission prompt (only when needed)
                #(if has_perm {
                    let pd = perm_detail.as_ref().unwrap();
                    let title_line = format!("{}", pd.tool);
                    let target_line = if pd.target.is_empty() {
                        pd.summary.clone()
                    } else {
                        pd.target.clone()
                    };
                    let summary_line = if !pd.target.is_empty() && !pd.summary.is_empty() && pd.summary != pd.target {
                        pd.summary.clone()
                    } else {
                        String::new()
                    };
                    // Pre-render diff lines as (color, text) pairs
                    let max_diff = 30usize;
                    let total_diff = pd.diff_lines.len();
                    let show_diff = total_diff.min(max_diff);
                    let diff_rendered: Vec<(Color, String)> = pd.diff_lines[..show_diff]
                        .iter()
                        .map(|dl| match dl {
                            super::events::DiffLine::Remove(s) => (Color::Red, format!("  - {s}")),
                            super::events::DiffLine::Add(s) => (Color::Green, format!("  + {s}")),
                            super::events::DiffLine::Context(s) => (Color::DarkGrey, format!("    {s}")),
                        })
                        .collect();
                    let has_diff = !diff_rendered.is_empty();
                    let truncated_msg = if total_diff > max_diff {
                        format!("    ... ({} more lines)", total_diff - max_diff)
                    } else {
                        String::new()
                    };
                    element! {
                        View(
                            border_style: BorderStyle::Round,
                            border_color: Color::Yellow,
                            flex_direction: FlexDirection::Column,
                            padding_left: 1u32,
                            padding_right: 1u32,
                            padding_top: 0u32,
                            padding_bottom: 0u32,
                            margin_bottom: 1u32,
                        ) {
                            // Title row: tool name
                            View(flex_direction: FlexDirection::Row) {
                                Text(
                                    color: Color::Yellow,
                                    weight: Weight::Bold,
                                    content: format!("⚠  {title_line}"),
                                )
                            }
                            // Target (file path or command summary)
                            View {
                                Text(
                                    color: Color::Cyan,
                                    content: target_line.clone(),
                                )
                            }
                            // Summary line (e.g. "Overwrite (42 lines, 1200 bytes)")
                            #(if !summary_line.is_empty() {
                                element! {
                                    View {
                                        Text(
                                            color: Color::DarkGrey,
                                            content: summary_line.clone(),
                                        )
                                    }
                                }
                            } else {
                                element! { View }
                            })
                            // Diff lines
                            #(if has_diff {
                                element! {
                                    View(
                                        flex_direction: FlexDirection::Column,
                                        margin_top: 0u32,
                                    ) {
                                        #(diff_rendered.iter().map(|(color, text)| {
                                            let c = *color;
                                            element! {
                                                Text(color: c, content: text.clone())
                                            }
                                        }))
                                        #(if !truncated_msg.is_empty() {
                                            element! {
                                                Text(color: Color::DarkGrey, content: truncated_msg.clone())
                                            }
                                        } else {
                                            element! { Text(content: "") }
                                        })
                                    }
                                }
                            } else {
                                element! { View }
                            })
                            // Action hint
                            View(margin_top: 0u32) {
                                Text(
                                    color: Color::DarkGrey,
                                    content: "y to allow · n to deny",
                                )
                            }
                        }
                    }
                } else {
                    element! { View }
                })

                // Background tasks
                #(if has_bg_tasks {
                    element! {
                        View(flex_direction: FlexDirection::Column) {
                            Text(color: Color::DarkGrey, content: "Background:")
                            #(bg_lines.iter().map(|line| {
                                element! { Text(content: line.clone()) }
                            }))
                        }
                    }
                } else {
                    element! { View }
                })

                // Slash command menu
                #(if !slash_commands.is_empty() {
                    element! {
                        View(
                            flex_direction: FlexDirection::Column,
                            border_style: BorderStyle::Round,
                            border_color: Color::DarkGrey,
                            padding_left: 1u32,
                            padding_right: 1u32,
                        ) {
                            #(slash_commands.iter().map(|(cmd, desc)| {
                                element! {
                                    View {
                                        Text(color: Color::Cyan, weight: Weight::Bold, content: format!("{cmd:<14}"))
                                        Text(color: Color::DarkGrey, content: desc.to_string())
                                    }
                                }
                            }))
                        }
                    }
                } else {
                    element! { View }
                })

                // Input prompt + status line below (Claude Code style)
                #(if is_waiting {
                    element! {
                        View(flex_direction: FlexDirection::Column) {
                            View {
                                Text(color: Color::Cyan, weight: Weight::Bold, content: "arcee> ")
                                Text(content: input_buf.clone())
                                Text(color: Color::DarkGrey, content: "█")
                            }
                            #(if has_model {
                                element! {
                                    View {
                                        Text(color: Color::DarkGrey, content: status_line.clone())
                                    }
                                }
                            } else {
                                element! { View }
                            })
                        }
                    }
                } else {
                    element! { View }
                })
            }
        }
    }
}
