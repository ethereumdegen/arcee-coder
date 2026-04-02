use crate::api::types::ContentBlock;
use crate::messages::types::*;

/// Estimate token count from text (rough heuristic: ~3.8 chars per token).
pub fn estimate_tokens(messages: &[Message]) -> u64 {
    let total_chars: usize = messages
        .iter()
        .map(|msg| match msg {
            Message::User(u) => u
                .content
                .iter()
                .map(|c| match c {
                    UserContent::Text(t) => t.len(),
                    UserContent::ToolResult { content, .. } => content.len(),
                })
                .sum(),
            Message::Assistant(a) => a
                .content
                .iter()
                .map(|b| match b {
                    ContentBlock::Text { text } => text.len(),
                    ContentBlock::ToolUse { input, .. } => {
                        serde_json::to_string(input).unwrap_or_default().len()
                    }
                    ContentBlock::Thinking { thinking } => thinking.len(),
                })
                .sum(),
            Message::System(s) => s.content.len(),
        })
        .sum();

    (total_chars as f64 / 3.8) as u64
}

/// Compact messages by summarizing older conversation turns.
/// Keeps the most recent `keep_recent` messages intact.
pub fn compact_messages(messages: &[Message], keep_recent: usize) -> Vec<Message> {
    if messages.len() <= keep_recent {
        return messages.to_vec();
    }

    let split_at = messages.len() - keep_recent;
    let old_messages = &messages[..split_at];

    // Build a summary of the old messages
    let mut summary_parts = Vec::new();
    for msg in old_messages {
        match msg {
            Message::User(u) => {
                for c in &u.content {
                    if let UserContent::Text(t) = c {
                        let preview: String = t.chars().take(200).collect();
                        summary_parts.push(format!("User: {preview}"));
                    }
                }
            }
            Message::Assistant(a) => {
                for block in &a.content {
                    match block {
                        ContentBlock::Text { text } => {
                            let preview: String = text.chars().take(200).collect();
                            summary_parts.push(format!("Assistant: {preview}"));
                        }
                        ContentBlock::ToolUse { name, .. } => {
                            summary_parts.push(format!("Tool: {name}"));
                        }
                        _ => {}
                    }
                }
            }
            Message::System(_) => {}
        }
    }

    let summary = format!(
        "[Context compacted — {} earlier messages summarized]\n{}",
        old_messages.len(),
        summary_parts.join("\n")
    );

    let mut compacted = vec![Message::System(SystemMessage {
        content: summary,
        message_type: SystemMessageType::CompactBoundary,
    })];

    compacted.extend_from_slice(&messages[split_at..]);
    compacted
}
