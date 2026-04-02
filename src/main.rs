use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use arcee_code::config::{CliOverrides, Config};
use arcee_code::permissions::{PermissionMode, PermissionStrictness};
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

    /// Resume a session: --resume (latest) or --resume <session-id>
    #[arg(long, short = 'c', alias = "continue", num_args = 0..=1, default_missing_value = "")]
    resume: Option<String>,

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

    /// Permission strictness: high, medium, low
    #[arg(long)]
    permission_strictness: Option<String>,

    /// Print mode: non-interactive, output only the result (for piping/SDK use)
    #[arg(long)]
    print: bool,

    /// Output format: text (default), json, stream-json
    #[arg(long, default_value = "text")]
    output_format: String,

    /// Replace the entire system prompt with this text
    #[arg(long)]
    system_prompt: Option<String>,

    /// Replace the entire system prompt with contents of this file
    #[arg(long)]
    system_prompt_file: Option<String>,

    /// Append this text to the default system prompt
    #[arg(long)]
    append_system_prompt: Option<String>,

    /// Append contents of this file to the default system prompt
    #[arg(long)]
    append_system_prompt_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise tracing subscriber driven by RUST_LOG (e.g. RUST_LOG=arcee_code=debug).
    // Logs go to ~/.arcee-code.log to avoid corrupting the TUI (raw terminal mode).
    if std::env::var("RUST_LOG").is_ok() {
        let log_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".arcee-code.log");
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .with_writer(file)
                .with_ansi(false)
                .init();
            tracing::info!("logging to {}", log_path.display());
        }
    }

    let cli = Cli::parse();

    let perm_mode = cli.permission_mode.as_deref().map(|s| match s {
        "auto" => PermissionMode::Auto,
        "plan" => PermissionMode::Plan,
        "bypass" => PermissionMode::Bypass,
        _ => PermissionMode::Default,
    });

    let perm_strictness = cli.permission_strictness.as_deref().map(|s| match s {
        "high" => PermissionStrictness::High,
        "low" => PermissionStrictness::Low,
        _ => PermissionStrictness::Medium,
    });

    // Resolve system prompt: --system-prompt-file takes priority over --system-prompt
    let system_prompt = if let Some(ref path) = cli.system_prompt_file {
        match std::fs::read_to_string(path) {
            Ok(content) => Some(content),
            Err(e) => {
                eprintln!("{}", format!("Error reading system prompt file '{path}': {e}").red());
                std::process::exit(1);
            }
        }
    } else {
        cli.system_prompt
    };

    // Resolve append system prompt: --append-system-prompt-file takes priority
    let append_system_prompt = if let Some(ref path) = cli.append_system_prompt_file {
        match std::fs::read_to_string(path) {
            Ok(content) => Some(content),
            Err(e) => {
                eprintln!("{}", format!("Error reading append system prompt file '{path}': {e}").red());
                std::process::exit(1);
            }
        }
    } else {
        cli.append_system_prompt
    };

    let (resume, resume_session_id) = match cli.resume {
        Some(id) if id.is_empty() => (true, None),         // --resume (latest)
        Some(id) => (true, Some(id)),                       // --resume <id>
        None => (false, None),
    };

    let overrides = CliOverrides {
        model: cli.model,
        permission_mode: perm_mode,
        max_turns: cli.max_turns,
        budget: cli.budget,
        verbose: cli.verbose,
        resume,
        resume_session_id,
        no_auto_route: cli.no_auto_route,
        permission_strictness: perm_strictness,
        system_prompt,
        append_system_prompt,
        print_mode: cli.print,
        output_format: Some(cli.output_format),
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

    // Handle resume — load the session to pass to run_repl
    let resume_session = if overrides.resume {
        let session = if let Some(ref id) = overrides.resume_session_id {
            Some(Session::load(id)?)
        } else {
            Session::load_latest()?
        };
        match &session {
            Some(s) => {
                println!(
                    "{}",
                    format!(
                        "Resuming session {} ({} messages, ${:.4})",
                        &s.id[..8],
                        s.messages.len(),
                        s.total_cost_usd,
                    )
                    .green()
                );
            }
            None => {
                println!("{}", "No previous session found.".yellow());
            }
        }
        session
    } else {
        None
    };

    // One-shot mode (prompt provided as arguments)
    if let Some(ref prompt) = overrides.prompt {
        return arcee_code::ui::run_oneshot(config, prompt, &overrides).await;
    }

    // Check if stdin is piped
    if !atty_is_terminal() {
        let mut input = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
        if !input.trim().is_empty() {
            return arcee_code::ui::run_oneshot(config, &input, &overrides).await;
        }
    }

    // Interactive REPL mode
    arcee_code::ui::run_repl(config, &overrides, resume_session).await
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
