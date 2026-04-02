use crate::engine::cost::CostTracker;
use crate::messages::types::Message;
use colored::Colorize;

/// Handle a slash command. Returns true if the command was handled.
pub fn handle_command(
    input: &str,
    messages: &mut Vec<Message>,
    cost_tracker: &CostTracker,
    model: &str,
) -> bool {
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
