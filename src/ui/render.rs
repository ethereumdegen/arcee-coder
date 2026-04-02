use crate::api::types::ContentBlock;
use crate::messages::types::*;
use colored::Colorize;

/// Render a message for display in the terminal.
pub fn render_message(msg: &Message) -> String {
    match msg {
        Message::User(u) => {
            let text: Vec<String> = u
                .content
                .iter()
                .map(|c| match c {
                    UserContent::Text(t) => t.clone(),
                    UserContent::ToolResult { content, is_error, .. } => {
                        if *is_error {
                            format!("[Tool Error] {content}")
                        } else {
                            format!("[Tool Result] {}", &content[..content.len().min(200)])
                        }
                    }
                })
                .collect();
            format!("{} {}", "You:".blue().bold(), text.join("\n"))
        }
        Message::Assistant(a) => {
            let mut parts = Vec::new();
            for block in &a.content {
                match block {
                    ContentBlock::Text { text } => {
                        parts.push(text.clone());
                    }
                    ContentBlock::ToolUse { name, .. } => {
                        parts.push(format!("[Using tool: {}]", name.cyan()));
                    }
                    ContentBlock::Thinking { thinking } => {
                        let preview: String = thinking.chars().take(100).collect();
                        parts.push(format!("{}", format!("(thinking: {preview}...)").dimmed()));
                    }
                }
            }
            format!("{} {}", "Arcee:".green().bold(), parts.join("\n"))
        }
        Message::System(s) => {
            format!("{} {}", "System:".yellow(), s.content)
        }
    }
}
