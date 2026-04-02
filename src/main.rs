use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use arcee_code::config::{CliOverrides, Config};
use arcee_code::permissions::PermissionMode;
use arcee_code::session::Session;

#[derive(Parser)]
#[command(
    name = "arcee",
    version = "0.1.0",
    about = "Arcee Code — AI coding assistant, powered by Arcee AI"
)]
struct Cli {
    /// Initial prompt (non-interactive mode if provided via pipe)
    prompt: Vec<String>,

    /// Model to use (default: trinity-large-thinking)
    #[arg(short, long)]
    model: Option<String>,

    /// Permission mode: default, auto, plan, bypass
    #[arg(short, long)]
    permission_mode: Option<String>,

    /// Resume the most recent session
    #[arg(long)]
    resume: bool,

    /// Maximum agentic turns
    #[arg(long)]
    max_turns: Option<usize>,

    /// Maximum spend budget in USD
    #[arg(long)]
    budget: Option<f64>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Disable adaptive model routing (always use the configured model)
    #[arg(long)]
    no_auto_route: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let perm_mode = cli.permission_mode.as_deref().map(|s| match s {
        "auto" => PermissionMode::Auto,
        "plan" => PermissionMode::Plan,
        "bypass" => PermissionMode::Bypass,
        _ => PermissionMode::Default,
    });

    let overrides = CliOverrides {
        model: cli.model,
        permission_mode: perm_mode,
        max_turns: cli.max_turns,
        budget: cli.budget,
        verbose: cli.verbose,
        resume: cli.resume,
        no_auto_route: cli.no_auto_route,
        prompt: if cli.prompt.is_empty() {
            None
        } else {
            Some(cli.prompt.join(" "))
        },
    };

    let mut config = Config::load(&overrides)?;

    // If no API key, prompt the user interactively
    if config.api_key.is_empty() {
        eprintln!(
            "{}",
            "No Arcee API key found.".yellow().bold()
        );
        eprintln!(
            "{}",
            "You can set it via: export ARCEE_API_KEY=your-api-key".dimmed()
        );
        eprintln!(
            "{}",
            "Get your key at: https://app.arcee.ai/".dimmed()
        );
        eprintln!();

        if atty_is_terminal() {
            eprint!("{}", "Enter your Arcee API key: ".cyan());
            use std::io::Write;
            std::io::stderr().flush()?;

            let mut key = String::new();
            std::io::stdin().read_line(&mut key)?;
            let key = key.trim().to_string();

            if key.is_empty() {
                eprintln!(
                    "{}",
                    "Error: API key is required. Set ARCEE_API_KEY or enter it when prompted."
                        .red()
                );
                std::process::exit(1);
            }

            config.api_key = key.clone();

            // Offer to save it
            eprint!(
                "{}",
                "Save key to ~/.arcee/config.json for future sessions? [Y/n] ".cyan()
            );
            std::io::stderr().flush()?;

            let mut save_response = String::new();
            std::io::stdin().read_line(&mut save_response)?;
            let save = save_response.trim().is_empty()
                || save_response.trim().eq_ignore_ascii_case("y");

            if save {
                save_api_key(&key)?;
                eprintln!("{}", "API key saved.".green());
            }
        } else {
            eprintln!(
                "{}",
                "Error: ARCEE_API_KEY environment variable is required.".red()
            );
            std::process::exit(1);
        }
    }

    // Ensure config directories exist
    arcee_code::config::paths::ensure_dirs()?;

    // Handle resume
    if overrides.resume {
        match Session::load_latest()? {
            Some(session) => {
                println!(
                    "{}",
                    format!(
                        "Resuming session {} ({} messages)",
                        &session.id[..8],
                        session.messages.len()
                    )
                    .green()
                );
            }
            None => {
                println!("{}", "No previous session found.".yellow());
            }
        }
    }

    // One-shot mode (prompt provided as arguments)
    if let Some(ref prompt) = overrides.prompt {
        return arcee_code::ui::run_oneshot(config, prompt).await;
    }

    // Check if stdin is piped
    if !atty_is_terminal() {
        let mut input = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
        if !input.trim().is_empty() {
            return arcee_code::ui::run_oneshot(config, &input).await;
        }
    }

    // Interactive REPL mode
    arcee_code::ui::run_repl(config).await
}

fn atty_is_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

fn save_api_key(key: &str) -> Result<()> {
    let config_dir = arcee_code::config::paths::config_dir();
    std::fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("config.json");

    // Load existing config or create new
    let mut config_value: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    config_value["api_key"] = serde_json::Value::String(key.to_string());

    let content = serde_json::to_string_pretty(&config_value)?;
    std::fs::write(&config_path, &content)?;

    // Set file permissions to 0600 (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}
