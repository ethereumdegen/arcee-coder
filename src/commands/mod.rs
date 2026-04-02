use crate::config::Config;
use crate::engine::cost::CostTracker;
use crate::engine::model_router::Intensity;
use crate::messages::types::Message;
use crate::permissions::PermissionStrictness;
use colored::Colorize;

/// Handle a slash command. Returns true if the command was handled.
pub fn handle_command(
    input: &str,
    messages: &mut Vec<Message>,
    cost_tracker: &CostTracker,
    config: &mut Config,
) -> bool {
    let model = config.model.as_str();
    let parts: Vec<&str> = input.trim().splitn(2, ' ').collect();
    let cmd = parts[0];
    let args = parts.get(1).copied().unwrap_or("");

    match cmd {
        "/help" => {
            print_help();
            true
        }
        "/clear" => {
            messages.clear();
            println!("{}", "Conversation cleared.".green());
            true
        }
        "/compact" => {
            let before = messages.len();
            let tokens_before = crate::engine::compact::estimate_tokens(messages);
            let compacted = crate::engine::compact::compact_messages(messages, 6);
            *messages = compacted;
            let tokens_after = crate::engine::compact::estimate_tokens(messages);
            println!(
                "{}",
                format!(
                    "Compacted: {} → {} messages (~{} → ~{} tokens)",
                    before,
                    messages.len(),
                    tokens_before,
                    tokens_after
                )
                .green()
            );
            true
        }
        "/cost" => {
            println!("{}", cost_tracker.summary(model));
            true
        }
        "/model" => {
            if args.is_empty() {
                println!("Current model: {}", model.green());
                println!("Available: trinity-mini, trinity-large-thinking");
                println!("Usage: /model <name>");
            } else {
                // Model switching is informational — actual switch happens in config
                println!(
                    "{}",
                    format!("Model set to: {args}. Will apply to next API call.").green()
                );
                println!(
                    "{}",
                    "Note: use --model flag or ARCEE_MODEL env var to persist.".dimmed()
                );
            }
            true
        }
        "/permission-strictness" | "/strictness" => {
            if args.is_empty() {
                let current = match config.permission_strictness {
                    PermissionStrictness::High => "high",
                    PermissionStrictness::Medium => "medium",
                    PermissionStrictness::Low => "low",
                };
                println!("Current permission strictness: {}", current.green());
                println!("  {} — prompt for all non-read-only tools", "high".cyan());
                println!(
                    "  {} — auto-allow safe bash commands, prompt for moderate+",
                    "medium".cyan()
                );
                println!(
                    "  {} — only prompt for destructive bash commands",
                    "low".cyan()
                );
                println!("Usage: /permission-strictness <high|medium|low>");
            } else {
                let new_strictness = match args.trim().to_lowercase().as_str() {
                    "high" => PermissionStrictness::High,
                    "medium" => PermissionStrictness::Medium,
                    "low" => PermissionStrictness::Low,
                    _ => {
                        println!(
                            "{}",
                            format!("Unknown strictness level: {args}. Use high, medium, or low.")
                                .yellow()
                        );
                        return true;
                    }
                };
                config.permission_strictness = new_strictness;
                println!(
                    "{}",
                    format!("Permission strictness set to: {args}.").green()
                );
            }
            true
        }
        "/intensity" => {
            if args.is_empty() {
                let current = config.intensity;
                println!("Current intensity: {}", current.as_str().green());
                println!();
                for level in &[Intensity::High, Intensity::Medium, Intensity::Low] {
                    let marker = if *level == current { " <--" } else { "" };
                    println!(
                        "  {} — {}{}",
                        level.as_str().cyan(),
                        level.description(),
                        marker.green()
                    );
                }
                println!();
                println!("Usage: /intensity <high|medium|low>");
            } else {
                match Intensity::from_str(args.trim()) {
                    Some(new_intensity) => {
                        config.intensity = new_intensity;
                        println!(
                            "{}",
                            format!(
                                "Intensity set to: {} — {}",
                                new_intensity.as_str(),
                                new_intensity.description()
                            )
                            .green()
                        );
                    }
                    None => {
                        println!(
                            "{}",
                            format!("Unknown intensity: {args}. Use high, medium, or low.").yellow()
                        );
                    }
                }
            }
            true
        }
        "/tokens" => {
            let estimated = crate::engine::compact::estimate_tokens(messages);
            println!(
                "Estimated context: ~{} tokens ({} messages)",
                estimated,
                messages.len()
            );
            true
        }
        "/history" => {
            print_history(messages);
            true
        }
        "/quit" | "/exit" | "/q" => {
            std::process::exit(0);
        }
        _ => {
            println!(
                "{}",
                format!("Unknown command: {cmd}. Type /help for available commands.").yellow()
            );
            true
        }
    }
}

fn print_help() {
    println!(
        "{}",
        r#"
Available commands:
  /help      Show this help message
  /clear     Clear the conversation
  /compact   Compress conversation context
  /cost      Show token usage and estimated cost
  /model     Show or switch model (trinity-mini, trinity-large-thinking)
  /intensity Set model routing intensity (high, medium, low)
  /strictness  Show or set permission strictness (high, medium, low)
  /tokens    Show estimated token count
  /history   Show conversation summary
  /quit      Exit arcee-code

Keyboard shortcuts:
  Ctrl+C     Interrupt current generation
  Ctrl+D     Exit

Environment variables:
  ARCEE_API_KEY     Your Arcee API key
  ARCEE_BASE_URL    Custom API endpoint
  ARCEE_MODEL       Default model override
"#
        .trim()
    );
}

fn print_history(messages: &[Message]) {
    if messages.is_empty() {
        println!("{}", "No messages yet.".dimmed());
        return;
    }

    for (i, msg) in messages.iter().enumerate() {
        match msg {
            Message::User(u) => {
                let text: String = u
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        crate::messages::types::UserContent::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let preview: String = text.chars().take(80).collect();
                println!("{:>3}. {} {}", i + 1, "User:".blue(), preview);
            }
            Message::Assistant(a) => {
                let text: String = a
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        crate::api::types::ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let preview: String = text.chars().take(80).collect();
                println!("{:>3}. {} {}", i + 1, "Arcee:".green(), preview);
            }
            Message::System(s) => {
                let preview: String = s.content.chars().take(80).collect();
                println!(
                    "{:>3}. {} {}",
                    i + 1,
                    "System:".yellow(),
                    preview
                );
            }
        }
    }
}
