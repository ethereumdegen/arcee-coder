use crate::config::Config;
use crate::engine::cost::CostTracker;
use crate::engine::model_router::Intensity;
use crate::messages::types::Message;
use crate::permissions::PermissionStrictness;
use crate::provider::Provider;
use std::fmt::Write;

/// Result of a slash command.
pub enum CommandResult {
    /// Command handled, output to display.
    Output(String),
    /// User wants to exit.
    Exit,
    /// Unknown command.
    Unknown(String),
}

/// Handle a slash command. Returns the output to display.
pub async fn handle_command(
    input: &str,
    messages: &mut Vec<Message>,
    cost_tracker: &CostTracker,
    config: &mut Config,
    provider: &dyn Provider,
) -> CommandResult {
    let model = config.model.as_str();
    let parts: Vec<&str> = input.trim().splitn(2, ' ').collect();
    let cmd = parts[0];
    let args = parts.get(1).copied().unwrap_or("");

    match cmd {
        "/help" => CommandResult::Output(help_text()),
        "/clear" => {
            messages.clear();
            CommandResult::Output("Conversation cleared.".to_string())
        }
        "/compact" => {
            let before = messages.len();
            let tokens_before = crate::engine::compact::estimate_tokens(messages);
            *messages = crate::engine::compact::compact_messages_ai(
                provider,
                crate::engine::model_router::MODEL_LIGHT,
                messages,
                6,
            )
            .await;
            let tokens_after = crate::engine::compact::estimate_tokens(messages);
            CommandResult::Output(format!(
                "Compacted: {} → {} messages (~{} → ~{} tokens)",
                before,
                messages.len(),
                tokens_before,
                tokens_after
            ))
        }
        "/cost" => CommandResult::Output(cost_tracker.summary(model)),
        "/model" => {
            let mut out = String::new();
            if args.is_empty() {
                let _ = writeln!(out, "Current model: {model}");
                let _ = writeln!(out, "Available: trinity-mini, trinity-large-thinking");
                let _ = write!(out, "Usage: /model <name>");
            } else {
                let _ = writeln!(out, "Model set to: {args}. Will apply to next API call.");
                let _ = write!(
                    out,
                    "Note: use --model flag or ARCEE_MODEL env var to persist."
                );
            }
            CommandResult::Output(out)
        }
        "/permission-strictness" | "/strictness" => {
            let mut out = String::new();
            if args.is_empty() {
                let current = match config.permission_strictness {
                    PermissionStrictness::High => "high",
                    PermissionStrictness::Medium => "medium",
                    PermissionStrictness::Low => "low",
                };
                let _ = writeln!(out, "Current permission strictness: {current}");
                let _ = writeln!(out, "  high — prompt for all non-read-only tools");
                let _ = writeln!(
                    out,
                    "  medium — auto-allow safe bash commands, prompt for moderate+"
                );
                let _ = writeln!(out, "  low — only prompt for destructive bash commands");
                let _ = write!(out, "Usage: /permission-strictness <high|medium|low>");
            } else {
                let new_strictness = match args.trim().to_lowercase().as_str() {
                    "high" => PermissionStrictness::High,
                    "medium" => PermissionStrictness::Medium,
                    "low" => PermissionStrictness::Low,
                    _ => {
                        return CommandResult::Output(format!(
                            "Unknown strictness level: {args}. Use high, medium, or low."
                        ));
                    }
                };
                config.permission_strictness = new_strictness;
                let _ = write!(out, "Permission strictness set to: {args}.");
            }
            CommandResult::Output(out)
        }
        "/intensity" => {
            let mut out = String::new();
            if args.is_empty() {
                let current = config.intensity;
                let _ = writeln!(out, "Current intensity: {}", current.as_str());
                let _ = writeln!(out);
                for level in &[Intensity::High, Intensity::Medium, Intensity::Low] {
                    let marker = if *level == current { " <--" } else { "" };
                    let _ = writeln!(
                        out,
                        "  {} — {}{}",
                        level.as_str(),
                        level.description(),
                        marker
                    );
                }
                let _ = writeln!(out);
                let _ = write!(out, "Usage: /intensity <high|medium|low>");
            } else {
                match Intensity::from_str(args.trim()) {
                    Some(new_intensity) => {
                        config.intensity = new_intensity;
                        let _ = write!(
                            out,
                            "Intensity set to: {} — {}",
                            new_intensity.as_str(),
                            new_intensity.description()
                        );
                    }
                    None => {
                        let _ = write!(
                            out,
                            "Unknown intensity: {args}. Use high, medium, or low."
                        );
                    }
                }
            }
            CommandResult::Output(out)
        }
        "/tokens" => {
            let estimated = crate::engine::compact::estimate_tokens(messages);
            CommandResult::Output(format!(
                "Estimated context: ~{} tokens ({} messages)",
                estimated,
                messages.len()
            ))
        }
        "/history" => CommandResult::Output(format_history(messages)),
        "/quit" | "/exit" | "/q" => CommandResult::Exit,
        _ => CommandResult::Unknown(format!(
            "Unknown command: {cmd}. Type /help for available commands."
        )),
    }
}

fn help_text() -> String {
    r#"Available commands:
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
  ESC        Interrupt current generation
  Ctrl+D     Exit

Environment variables:
  ARCEE_API_KEY     Your Arcee API key
  ARCEE_BASE_URL    Custom API endpoint
  ARCEE_MODEL       Default model override"#
        .to_string()
}

fn format_history(messages: &[Message]) -> String {
    if messages.is_empty() {
        return "No messages yet.".to_string();
    }

    let mut out = String::new();
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
                let _ = writeln!(out, "{:>3}. User: {}", i + 1, preview);
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
                let _ = writeln!(out, "{:>3}. Arcee: {}", i + 1, preview);
            }
            Message::System(s) => {
                let preview: String = s.content.chars().take(80).collect();
                let _ = writeln!(out, "{:>3}. System: {}", i + 1, preview);
            }
        }
    }
    out
}
