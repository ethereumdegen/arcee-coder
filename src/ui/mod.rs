pub mod bridge;
pub mod components;
pub mod events;
pub mod input_queue;
pub mod render;
pub mod renderer;
pub mod thinking;

use crate::commands;
use crate::config::Config;
use crate::engine;
use crate::engine::cost::CostTracker;
use crate::messages::types::Message;
use crate::session::Session;
use crate::tools;
use crate::tools::lsp::LspManager;
use crate::tools::task_store::TaskStore;
use crate::tools::ToolContext;
use crate::ui::bridge::UiBridge;
use crate::ui::events::{StatusLevel, UiCommand};
use anyhow::Result;
use colored::Colorize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Build a ToolContext from the current config and shared state.
fn build_tool_context(config: &Config, api_client: Arc<crate::api::ApiClient>) -> ToolContext {
    ToolContext {
        cwd: config.cwd.clone(),
        permission_mode: Arc::new(Mutex::new(config.permission_mode)),
        task_store: Arc::new(Mutex::new(TaskStore::new())),
        background_tasks: Arc::new(Mutex::new(
            crate::tools::background_tasks::BackgroundTaskStore::new(),
        )),
        api_client,
        config: config.clone(),
        lsp_manager: Arc::new(Mutex::new(LspManager::new())),
        plan_file_path: Arc::new(Mutex::new(None)),
    }
}

/// Fetch model pricing from the API and store in config. Warns on failure.
async fn fetch_pricing(client: &crate::api::ApiClient, config: &mut Config) {
    match client.fetch_models().await {
        Ok(models) => {
            let table = crate::engine::cost::build_pricing_table(&models);
            if config.verbose && !table.is_empty() {
                eprintln!(
                    "{}",
                    format!("[Fetched pricing for {} model(s)]", table.len()).dimmed()
                );
            }
            config.pricing_table = table;
        }
        Err(e) => {
            if config.verbose {
                eprintln!(
                    "{}",
                    format!("[Warning: failed to fetch model pricing: {e}]").dimmed()
                );
            }
        }
    }
}

/// Run the interactive REPL with the iocraft-based UI.
pub async fn run_repl(mut config: Config) -> Result<()> {
    let client = Arc::new(crate::api::ApiClient::new(
        config.api_key.clone(),
        Some(config.base_url.clone()),
        Some(config.model.clone()),
    ));

    // Fetch dynamic pricing from the API
    fetch_pricing(&client, &mut config).await;

    let tool_registry = tools::build_default_registry();
    let tool_context = build_tool_context(&config, client.clone());
    let mut messages: Vec<Message> = Vec::new();
    let mut cost_tracker = CostTracker::with_pricing(config.pricing_table.clone());
    let mut session = Session::new(config.cwd.clone(), config.model.clone());

    // Print welcome banner before the UI thread takes over
    print_welcome(&config);

    // Create the bridge between main thread and UI thread
    let escape_flag = Arc::new(AtomicBool::new(false));
    let (bridge, ui_handle) = UiBridge::new(escape_flag.clone());

    // Send initial status info
    let routing = if config.auto_model_routing {
        "auto".to_string()
    } else {
        config.model.clone()
    };
    bridge.model_info(&routing);
    bridge.turn_info(0, config.max_turns);

    // Spawn the iocraft UI thread
    let ui_thread = renderer::spawn_ui_thread(ui_handle);

    // Give the UI thread a moment to initialize
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Main REPL loop: send prompt events, wait for input, run queries
    loop {
        bridge.show_prompt();

        // Wait for user input from the UI thread.
        // Use block_in_place so tokio can still run background tasks while we wait.
        let input = tokio::task::block_in_place(|| {
            match bridge.recv_command() {
                Some(UiCommand::UserInput(line)) => Some(line),
                Some(UiCommand::EscapePressed) => Some(String::new()), // will be skipped below
                Some(UiCommand::PermissionResponse(_)) => Some(String::new()),
                None => None, // channel closed
            }
        });

        let input = match input {
            Some(line) => line,
            None => break, // UI thread exited
        };

        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        // Handle slash commands
        if input.starts_with('/') {
            match commands::handle_command(&input, &mut messages, &cost_tracker, &mut config, &client).await {
                commands::CommandResult::Output(text) => {
                    bridge.status(&text, StatusLevel::Info);
                    continue;
                }
                commands::CommandResult::Exit => break,
                commands::CommandResult::Unknown(text) => {
                    bridge.status(&text, StatusLevel::Warning);
                    continue;
                }
            }
        }

        // Run the query
        messages.push(Message::user_text(&input));
        escape_flag.store(false, Ordering::Relaxed);

        if let Err(e) = engine::query_loop(
            &client,
            &mut messages,
            &tool_registry,
            &config,
            &mut cost_tracker,
            &tool_context,
            &escape_flag,
            None,
            Some(&bridge),
        )
        .await
        {
            bridge.status(&format!("Error: {e}"), StatusLevel::Error);
        }

        bridge.stream_end();

        // Update session
        update_session(&mut session, &messages, &cost_tracker, &config);

        // Send cost update to UI
        let cost = cost_tracker.estimate_cost_usd(&config.model);
        bridge.cost_update(
            cost_tracker.total_input_tokens,
            cost_tracker.total_output_tokens,
            cost,
        );
    }

    // Tell the UI to exit
    bridge.request_exit();

    // Wait for UI thread to finish (with timeout)
    let _ = ui_thread.join();

    Ok(())
}

fn update_session(
    session: &mut Session,
    messages: &[Message],
    cost_tracker: &CostTracker,
    config: &Config,
) {
    session.messages = messages.to_vec();
    session.updated_at = chrono::Utc::now();
    session.total_cost_usd = cost_tracker.estimate_cost_usd(&config.model);
    session.total_input_tokens = cost_tracker.total_input_tokens;
    session.total_output_tokens = cost_tracker.total_output_tokens;

    if let Err(e) = session.save() {
        if config.verbose {
            eprintln!("{}", format!("Failed to save session: {e}").dimmed());
        }
    }
}

/// Spawn a background thread that listens for ESC key presses and sets the flag.
/// Used for oneshot mode where iocraft is not active.
fn spawn_escape_listener(flag: &Arc<AtomicBool>) -> EscapeListenerGuard {
    let flag = flag.clone();
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    std::thread::spawn(move || {
        use std::io::Read;
        let stdin_fd = 0;

        while running_clone.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(150));

            if !running_clone.load(Ordering::Relaxed) {
                break;
            }

            if crate::permissions::is_prompt_active() {
                continue;
            }

            if crossterm::terminal::enable_raw_mode().is_err() {
                continue;
            }

            let mut pfd = libc::pollfd {
                fd: stdin_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut pfd, 1, 0) };

            let esc_detected = if ready > 0 && (pfd.revents & libc::POLLIN) != 0 {
                if crate::permissions::is_prompt_active() {
                    let _ = crossterm::terminal::disable_raw_mode();
                    continue;
                }
                let mut buf = [0u8; 8];
                let n = std::io::stdin().lock().read(&mut buf).unwrap_or(0);
                n > 0 && buf[0] == 0x1B
            } else {
                false
            };

            let _ = crossterm::terminal::disable_raw_mode();

            if esc_detected {
                flag.store(true, Ordering::Relaxed);
                break;
            }
        }
    });

    EscapeListenerGuard { running }
}

struct EscapeListenerGuard {
    running: Arc<AtomicBool>,
}

impl Drop for EscapeListenerGuard {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Run a one-shot query (non-interactive, no iocraft UI).
pub async fn run_oneshot(mut config: Config, prompt: &str) -> Result<()> {
    let client = Arc::new(crate::api::ApiClient::new(
        config.api_key.clone(),
        Some(config.base_url.clone()),
        Some(config.model.clone()),
    ));

    fetch_pricing(&client, &mut config).await;

    let tool_registry = tools::build_default_registry();
    let tool_context = build_tool_context(&config, client.clone());
    let mut messages = vec![Message::user_text(prompt)];
    let mut cost_tracker = CostTracker::with_pricing(config.pricing_table.clone());

    let escape_flag = Arc::new(AtomicBool::new(false));
    let _guard = spawn_escape_listener(&escape_flag);
    engine::query_loop(
        &client,
        &mut messages,
        &tool_registry,
        &config,
        &mut cost_tracker,
        &tool_context,
        &escape_flag,
        None,
        None, // no bridge for oneshot mode
    )
    .await?;

    if config.verbose {
        eprintln!("{}", cost_tracker.summary(&config.model).dimmed());
    }

    Ok(())
}

fn print_welcome(config: &Config) {
    println!(
        "{}",
        r#"
    ╔═══════════════════════════════════╗
    ║         Arcee Code v2.1.3         ║
    ║   AI coding assistant (Rust)      ║
    ║   Powered by Arcee AI             ║
    ╚═══════════════════════════════════╝
"#
        .cyan()
    );
    let routing = if config.auto_model_routing {
        "auto (mini ↔ large-thinking)".to_string()
    } else {
        config.model.clone()
    };
    println!(
        "  Model: {}  |  Intensity: {}  |  Permissions: {:?} ({:?})",
        routing.green(),
        format!("{:?}", config.intensity).green(),
        config.permission_mode,
        config.permission_strictness,
    );
    println!(
        "  CWD: {}",
        config.cwd.display().to_string().dimmed()
    );
    println!(
        "  {}",
        "Type /help for commands, Ctrl+D to exit.".dimmed()
    );
    println!();
}
