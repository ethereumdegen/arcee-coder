pub mod render;

use crate::commands;
use crate::config::Config;
use crate::engine;
use crate::engine::cost::CostTracker;
use crate::messages::types::Message;
use crate::session::Session;
use crate::tools;
use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

/// Run the interactive REPL.
pub async fn run_repl(config: Config) -> Result<()> {
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
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                rl.add_history_entry(input)?;

                // Handle slash commands
                if input.starts_with('/') {
                    commands::handle_command(input, &mut messages, &cost_tracker, &config.model);
                    continue;
                }

                // Add user message
                messages.push(Message::user_text(input));

                // Run the query loop
                println!();
                match engine::query_loop(
                    &client,
                    &mut messages,
                    &tool_registry,
                    &config,
                    &mut cost_tracker,
                )
                .await
                {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("{}", format!("Error: {e}").red());
                    }
                }

                // Update session
                session.messages = messages.clone();
                session.updated_at = chrono::Utc::now();
                session.total_cost_usd = cost_tracker.estimate_cost_usd(&config.model);
                session.total_input_tokens = cost_tracker.total_input_tokens;
                session.total_output_tokens = cost_tracker.total_output_tokens;

                if let Err(e) = session.save() {
                    if config.verbose {
                        eprintln!("{}", format!("Failed to save session: {e}").dimmed());
                    }
                }

                // Show cost after each turn
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
