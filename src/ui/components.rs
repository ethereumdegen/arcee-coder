/// iocraft UI components for the terminal interface.

use crate::ui::bridge::UiHandle;
use crate::ui::events::{StatusLevel, UiCommand, UiEvent};
use iocraft::prelude::*;
use std::time::{Duration, Instant};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Shared state updated by polling UiEvents.
/// Non-Copy, so we use State<AppState> with .read() / .write().
#[derive(Default)]
pub struct AppState {
    pub is_streaming: bool,
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
    pub perm_prompt: Option<(String, String)>, // (tool, description)
}

// ─── Root App component ────────────────────────────────────────────────────

#[derive(Default, Props)]
pub struct AppProps {
    pub ui_handle: UiHandle,
}

#[component]
pub fn App(props: &AppProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let (stdout, _stderr) = hooks.use_output();

    let mut state: State<AppState> = hooks.use_state(AppState::default);
    let mut spinner_frame = hooks.use_state(|| 0usize);
    let mut should_exit = hooks.use_state(|| false);

    // Poll for events from the main thread
    let handle = props.ui_handle.clone();
    let out_handle = stdout.clone();
    hooks.use_future(async move {
        let mut is_streaming = false;
        loop {
            // Poll faster during streaming for smooth output, slower when idle
            let poll_ms = if is_streaming { 5 } else { 50 };
            smol::Timer::after(Duration::from_millis(poll_ms)).await;

            let mut changed = false;

            // Drain all pending events
            while let Some(event) = handle.try_recv_event() {
                let mut s = state.write();
                match event {
                    UiEvent::StreamText(text) => {
                        // Convert \n→\r\n for raw terminal mode, then use iocraft's
                        // print() so text persists above the rendered component.
                        s.is_streaming = true;
                        is_streaming = true;
                        drop(s);
                        let converted = text.replace('\n', "\r\n");
                        out_handle.print(&converted);
                        continue; // re-enter while loop
                    }
                    UiEvent::StreamToolStart { name, .. } => {
                        s.is_streaming = true;
                        is_streaming = true;
                        drop(s);
                        out_handle.println(
                            format!("\r\n\x1b[1;36mTool:\x1b[0m \x1b[36m{name}\x1b[0m")
                        );
                        continue;
                    }
                    UiEvent::StreamEnd => {
                        s.is_streaming = false;
                        is_streaming = false;
                        s.active_tool_name.clear();
                        s.active_tool_start = None;
                        changed = true;
                    }
                    UiEvent::ToolExecStart { name } => {
                        s.active_tool_name = name;
                        s.active_tool_start = Some(Instant::now());
                        changed = true;
                    }
                    UiEvent::ToolResult {
                        preview,
                        is_error,
                        duration_ms,
                        ..
                    } => {
                        s.active_tool_name.clear();
                        s.active_tool_start = None;
                        drop(s);
                        if is_error {
                            out_handle.println(format!("\x1b[31m  Error: {preview}\x1b[0m"));
                        } else {
                            out_handle.println(format!("\x1b[2m  {preview}\x1b[0m"));
                        }
                        if duration_ms > 100 {
                            out_handle.println(format!("\x1b[2m  ({duration_ms}ms)\x1b[0m"));
                        }
                        changed = true;
                        continue;
                    }
                    UiEvent::BackgroundTaskStarted { id, description } => {
                        s.bg_display_lines
                            .push(format!("  #{id} ⠙ {description}..."));
                        changed = true;
                    }
                    UiEvent::BackgroundTaskCompleted {
                        id,
                        status,
                        duration_secs,
                    } => {
                        // Replace the running line with completed line
                        if let Some(line) = s
                            .bg_display_lines
                            .iter_mut()
                            .find(|l| l.contains(&format!("#{id} ")))
                        {
                            *line = format!(
                                "  #{id} ✓ [{status}] ({duration_secs:.1}s)"
                            );
                        }
                        // Prune old completed lines (keep last 10)
                        if s.bg_display_lines.len() > 10 {
                            let all_completed = s.bg_display_lines.iter().all(|l| l.contains('✓'));
                            if all_completed {
                                let drain_count = s.bg_display_lines.len() - 5;
                                s.bg_display_lines.drain(..drain_count);
                            }
                        }
                        changed = true;
                    }
                    UiEvent::ModelInfo(model) => {
                        s.model = model;
                        changed = true;
                    }
                    UiEvent::CostUpdate {
                        input_tokens,
                        output_tokens,
                        cost_usd,
                    } => {
                        s.input_tokens = input_tokens;
                        s.output_tokens = output_tokens;
                        s.cost_usd = cost_usd;
                        changed = true;
                    }
                    UiEvent::TurnInfo { turn, max_turns } => {
                        s.turn = turn;
                        s.max_turns = max_turns;
                        changed = true;
                    }
                    UiEvent::StatusMessage { text, level } => {
                        drop(s);
                        let colored = match level {
                            StatusLevel::Info => text,
                            StatusLevel::Warning => format!("\x1b[33m{text}\x1b[0m"),
                            StatusLevel::Error => format!("\x1b[31m{text}\x1b[0m"),
                            StatusLevel::Dim => format!("\x1b[2m{text}\x1b[0m"),
                        };
                        out_handle.println(&colored);
                        continue;
                    }
                    UiEvent::ShowPrompt => {
                        s.waiting_for_input = true;
                        s.input_buffer.clear();
                        changed = true;
                    }
                    UiEvent::PermissionPrompt { tool, description } => {
                        s.perm_prompt = Some((tool, description));
                        changed = true;
                    }
                    UiEvent::Exit => {
                        drop(s);
                        should_exit.set(true);
                        return;
                    }
                }
            }

            // Always advance spinner for animation (also triggers re-render on state changes)
            spinner_frame.set(spinner_frame.get().wrapping_add(1));
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
                            s.input_buffer.clear();
                            s.waiting_for_input = false;
                            drop(s);
                            cmd_handle.send_command(UiCommand::UserInput(input));
                        }
                        KeyCode::Char(c) => {
                            s.input_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            s.input_buffer.pop();
                        }
                        KeyCode::Esc => {
                            // In input mode, ESC clears the buffer
                            s.input_buffer.clear();
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
    } else if s.is_streaming {
        format!("{spinner} streaming...")
    } else {
        String::new()
    };

    let has_bg_tasks = !s.bg_display_lines.is_empty();
    let bg_lines: Vec<String> = s.bg_display_lines.clone();

    let has_perm = s.perm_prompt.is_some();
    let perm_tool = s
        .perm_prompt
        .as_ref()
        .map(|(t, _)| t.clone())
        .unwrap_or_default();
    let perm_desc = s
        .perm_prompt
        .as_ref()
        .map(|(_, d)| d.clone())
        .unwrap_or_default();

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

    // Drop read before element! to avoid potential deadlock
    drop(s);

    element! {
        View(flex_direction: FlexDirection::Column) {
            // Active tool spinner or streaming indicator
            #(if !tool_spinner_text.is_empty() {
                element! {
                    View {
                        Text(color: Color::Cyan, content: tool_spinner_text.clone())
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

            // Permission prompt
            #(if has_perm {
                element! {
                    View(
                        border_style: BorderStyle::Round,
                        border_color: Color::Yellow,
                        flex_direction: FlexDirection::Column,
                        padding_left: 1u32,
                        padding_right: 1u32,
                    ) {
                        Text(
                            color: Color::Yellow,
                            weight: Weight::Bold,
                            content: format!("Permission required: {}", perm_tool),
                        )
                        Text(content: perm_desc.clone())
                        Text(color: Color::Yellow, content: "Allow? (y/n)")
                    }
                }
            } else {
                element! { View }
            })

            // Status bar
            #(if has_model {
                element! {
                    View(border_style: BorderStyle::Single, border_color: Color::DarkGrey) {
                        Text(color: Color::DarkGrey, content: status_line.clone())
                    }
                }
            } else {
                element! { View }
            })

            // Input prompt
            #(if is_waiting {
                element! {
                    View {
                        Text(color: Color::Cyan, weight: Weight::Bold, content: "arcee> ")
                        Text(content: input_buf.clone())
                        Text(color: Color::DarkGrey, content: "█")
                    }
                }
            } else {
                element! { View }
            })
        }
    }
}
