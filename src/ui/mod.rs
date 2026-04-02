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
use crate::ui::input_queue::drain_pending_stdin;
use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

/// Run the interactive REPL.
pub async fn run_repl(mut config: Config) -> Result<()> {
    let client = crate::api::ApiClient::new(
        config.api_key.clone(),
        Some(config.base_url.clone()),
        Some(config.model.clone()),
    );

    let tool_registry = tools::build_default_registry();
    let mut messages: Vec<Message> = Vec::new();
    let mut cost_tracker = CostTracker::new();
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
                )
                .await;
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "Interrupted.".yellow());
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Goodbye!".dimmed());
                break;
            }
            Err(e) => {
                eprintln!("{}", format!("Input error: {e}").red());
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
    client: &crate::api::ApiClient,
    messages: &mut Vec<Message>,
    tools: &crate::tools::ToolRegistry,
    config: &mut Config,
    cost_tracker: &mut CostTracker,
    session: &mut Session,
) {
    messages.push(Message::user_text(input));

    println!();
    if let Err(e) = engine::query_loop(client, messages, tools, config, cost_tracker).await {
        eprintln!("{}", format!("Error: {e}").red());
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
        if let Err(e) = engine::query_loop(client, messages, tools, config, cost_tracker).await {
            eprintln!("{}", format!("Error: {e}").red());
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
pub async fn run_oneshot(config: Config, prompt: &str) -> Result<()> {
    let client = crate::api::ApiClient::new(
        config.api_key.clone(),
        Some(config.base_url.clone()),
        Some(config.model.clone()),
    );

    let tool_registry = tools::build_default_registry();
    let mut messages = vec![Message::user_text(prompt)];
    let mut cost_tracker = CostTracker::new();

    engine::query_loop(
        &client,
        &mut messages,
        &tool_registry,
        &config,
        &mut cost_tracker,
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
        "  Model: {}  |  Mode: {:?}",
        routing.green(),
        config.permission_mode
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
