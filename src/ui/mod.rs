pub mod input_queue;
pub mod render;
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
use crate::ui::input_queue::drain_pending_stdin;
use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Spawn a background thread that listens for ESC key presses and sets the flag.
/// Returns a guard that stops the listener and restores terminal mode on drop.
///
/// Toggles raw mode briefly in a polling loop to read individual key bytes
/// from stdin, then immediately restores cooked mode so normal println! works.
fn spawn_escape_listener(flag: &Arc<AtomicBool>) -> EscapeListenerGuard {
    let flag = flag.clone();
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    std::thread::spawn(move || {
        use std::io::Read;
        let stdin_fd = 0; // STDIN_FILENO

        while running_clone.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(150));

            if !running_clone.load(Ordering::Relaxed) {
                break;
            }

            // If a permission prompt is active, don't touch stdin or terminal modes.
            if crate::permissions::is_prompt_active() {
                continue;
            }

            // Briefly enable raw mode to check for keypress
            if crossterm::terminal::enable_raw_mode().is_err() {
                continue;
            }

            // Use poll(2) to check if stdin has data, instead of toggling O_NONBLOCK
            // which races with permission prompts and causes EAGAIN denials.
            let mut pfd = libc::pollfd {
                fd: stdin_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut pfd, 1, 0) };

            let esc_detected = if ready > 0 && (pfd.revents & libc::POLLIN) != 0 {
                // Double-check prompt isn't active (could have started during poll)
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

            // Restore cooked mode
            let _ = crossterm::terminal::disable_raw_mode();

            if esc_detected {
                flag.store(true, Ordering::Relaxed);
                break;
            }
        }
    });

    EscapeListenerGuard { running }
}

/// RAII guard that stops the escape listener thread and ensures terminal is restored.
struct EscapeListenerGuard {
    running: Arc<AtomicBool>,
}

impl Drop for EscapeListenerGuard {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // Ensure terminal is in cooked mode
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Build a ToolContext from the current config and shared state.
fn build_tool_context(config: &Config, api_client: Arc<crate::api::ApiClient>) -> ToolContext {
    ToolContext {
        cwd: config.cwd.clone(),
        permission_mode: Arc::new(Mutex::new(config.permission_mode)),
        task_store: Arc::new(Mutex::new(TaskStore::new())),
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

/// Run the interactive REPL.
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

    print_welcome(&config);

    // Set up readline
    let history_path = config.config_dir.join("history.txt");
    let mut rl = DefaultEditor::new()?;
    let _ = rl.load_history(&history_path);

    loop {
        let prompt = format!("{} ", "arcee>".cyan().bold());
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim().to_string();
                if input.is_empty() {
                    continue;
                }

                rl.add_history_entry(&input)?;

                // Handle slash commands
                if input.starts_with('/') {
                    commands::handle_command(&input, &mut messages, &cost_tracker, &mut config);
                    continue;
                }

                // Run query and then drain any queued type-ahead input
                run_and_drain_queue(
                    &input,
                    &client,
                    &mut messages,
                    &tool_registry,
                    &mut config,
                    &mut cost_tracker,
                    &mut session,
                    &tool_context,
                )
                .await;
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "Interrupted.".yellow());
            }
            Err(ReadlineError::Eof) => {
                eprintln!(
                    "{}",
                    "[REPL exit: EOF on stdin (Ctrl+D or stdin closed by child process)]".yellow()
                );
                println!("{}", "Goodbye!".dimmed());
                break;
            }
            Err(e) => {
                eprintln!(
                    "{}",
                    format!("[REPL exit: readline error: {e}]").red()
                );
                break;
            }
        }
    }

    // Save history
    let _ = rl.save_history(&history_path);

    Ok(())
}

/// Execute a user input through query_loop, then drain and process any lines
/// the user typed ahead while the loop was running.
async fn run_and_drain_queue(
    input: &str,
    client: &Arc<crate::api::ApiClient>,
    messages: &mut Vec<Message>,
    tools: &crate::tools::ToolRegistry,
    config: &mut Config,
    cost_tracker: &mut CostTracker,
    session: &mut Session,
    tool_context: &ToolContext,
) {
    messages.push(Message::user_text(input));

    let escape_flag = Arc::new(AtomicBool::new(false));

    println!();
    {
        let _guard = spawn_escape_listener(&escape_flag);
        if let Err(e) = engine::query_loop(client, messages, tools, config, cost_tracker, tool_context, &escape_flag).await {
            eprintln!("{}", format!("Error: {e}").red());
        }
    }

    update_session(session, messages, cost_tracker, config);
    print_cost(cost_tracker, config);

    // Drain any lines typed while the loop was running
    let queued = drain_pending_stdin();
    for queued_input in queued {
        if queued_input.starts_with('/') {
            commands::handle_command(&queued_input, messages, cost_tracker, config);
            continue;
        }

        println!(
            "\n{} {}",
            "[queued]".cyan().dimmed(),
            queued_input.dimmed()
        );

        messages.push(Message::user_text(&queued_input));

        println!();
        escape_flag.store(false, Ordering::Relaxed);
        {
            let _guard = spawn_escape_listener(&escape_flag);
            if let Err(e) = engine::query_loop(client, messages, tools, config, cost_tracker, tool_context, &escape_flag).await {
                eprintln!("{}", format!("Error: {e}").red());
            }
        }

        update_session(session, messages, cost_tracker, config);
        print_cost(cost_tracker, config);
    }
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

fn print_cost(cost_tracker: &CostTracker, config: &Config) {
    let cost = cost_tracker.estimate_cost_usd(&config.model);
    println!(
        "{}",
        format!(
            "[{} in / {} out | ${:.4}]",
            cost_tracker.total_input_tokens,
            cost_tracker.total_output_tokens,
            cost
        )
        .dimmed()
    );
}

/// Run a one-shot query (non-interactive).
pub async fn run_oneshot(mut config: Config, prompt: &str) -> Result<()> {
    let client = Arc::new(crate::api::ApiClient::new(
        config.api_key.clone(),
        Some(config.base_url.clone()),
        Some(config.model.clone()),
    ));

    // Fetch dynamic pricing from the API
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
    )
    .await?;

    if config.verbose {
        eprintln!(
            "{}",
            cost_tracker.summary(&config.model).dimmed()
        );
    }

    Ok(())
}

fn print_welcome(config: &Config) {
    println!(
        "{}",
        r#"
    ╔═══════════════════════════════════╗
    ║         Arcee Code v0.1.0         ║
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
