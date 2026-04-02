use crate::api::client::ApiClient;
use crate::api::types::ContentBlock;
use crate::messages::normalize::normalize_for_api;
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

/// Build a text transcript of messages for the summarization prompt.
fn build_transcript(messages: &[Message]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        match msg {
            Message::User(u) => {
                for c in &u.content {
                    match c {
                        UserContent::Text(t) => {
                            let preview: String = t.chars().take(1000).collect();
                            parts.push(format!("User: {preview}"));
                        }
                        UserContent::ToolResult {
                            tool_use_id: _,
                            content,
                            is_error,
                        } => {
                            let label = if *is_error {
                                "Tool Error"
                            } else {
                                "Tool Result"
                            };
                            let preview: String = content.chars().take(500).collect();
                            parts.push(format!("{label}: {preview}"));
                        }
                    }
                }
            }
            Message::Assistant(a) => {
                for block in &a.content {
                    match block {
                        ContentBlock::Text { text } => {
                            let preview: String = text.chars().take(1000).collect();
                            parts.push(format!("Assistant: {preview}"));
                        }
                        ContentBlock::ToolUse { name, input, .. } => {
                            let args: String = serde_json::to_string(input)
                                .unwrap_or_default()
                                .chars()
                                .take(200)
                                .collect();
                            parts.push(format!("Tool Call: {name}({args})"));
                        }
                        ContentBlock::Thinking { .. } => {}
                    }
                }
            }
            Message::System(_) => {}
        }
    }
    parts.join("\n")
}

/// Fallback: build a simple truncated summary (no API call).
fn build_fallback_summary(old_messages: &[Message]) -> String {
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

    format!(
        "[Context compacted — {} earlier messages summarized (fallback)]\n{}",
        old_messages.len(),
        summary_parts.join("\n")
    )
}

const COMPACT_SYSTEM_PROMPT: &str = "\
You are a conversation summarizer. Your job is to produce a concise but comprehensive \
summary of a conversation between a user and an AI coding assistant. \
The summary will replace the original messages, so it MUST preserve:\n\
- What the user asked for (their goals and requirements)\n\
- What files were read, created, or modified (with paths)\n\
- Key decisions made and why\n\
- Current state of the work (what's done, what's pending)\n\
- Any errors encountered and how they were resolved\n\
- Important code patterns, variable names, or architecture discussed\n\n\
Be specific — include file paths, function names, and concrete details. \
Do NOT be vague. Output ONLY the summary, no preamble.";

/// Compact messages using AI summarization, with fallback to truncation.
/// Returns the compacted message list.
pub async fn compact_messages_ai(
    client: &ApiClient,
    model: &str,
    messages: &[Message],
    keep_recent: usize,
) -> Vec<Message> {
    if messages.len() <= keep_recent {
        return messages.to_vec();
    }

    let split_at = messages.len() - keep_recent;
    let old_messages = &messages[..split_at];

    // Build transcript of old messages for summarization
    let transcript = build_transcript(old_messages);

    // Cap transcript to avoid blowing up the summarization call
    let transcript: String = transcript.chars().take(30_000).collect();

    let summary_request = format!(
        "Summarize this conversation ({} messages, ~{} tokens):\n\n{}",
        old_messages.len(),
        estimate_tokens(old_messages),
        transcript
    );

    // Try AI summarization
    let summary = match summarize_with_api(client, model, &summary_request).await {
        Ok(s) if !s.trim().is_empty() => {
            format!(
                "[Context compacted — {} earlier messages summarized by AI]\n\n{}",
                old_messages.len(),
                s.trim()
            )
        }
        Ok(_) => build_fallback_summary(old_messages),
        Err(e) => {
            eprintln!(
                "\x1b[33m[compact: AI summarization failed ({e}), using fallback]\x1b[0m"
            );
            build_fallback_summary(old_messages)
        }
    };

    let mut compacted = vec![Message::System(SystemMessage {
        content: summary,
        message_type: SystemMessageType::CompactBoundary,
    })];

    compacted.extend_from_slice(&messages[split_at..]);
    compacted
}

/// Call the API to generate a summary of the conversation.
async fn summarize_with_api(
    client: &ApiClient,
    model: &str,
    content: &str,
) -> Result<String, String> {
    let messages = vec![Message::User(UserMessage {
        content: vec![UserContent::Text(content.to_string())],
    })];

    let api_messages = normalize_for_api(&messages);

    let mut result_text = String::new();
    let mut noop_text = |text: &str| {
        result_text.push_str(text);
    };
    let mut noop_tool = |_id: &str, _name: &str| {};

    let (content_blocks, _stop, _usage) = client
        .send_message_with_model(
            model,
            COMPACT_SYSTEM_PROMPT,
            api_messages,
            vec![], // no tools
            4096,   // enough for a summary
            &mut noop_text,
            &mut noop_tool,
            None, // no escape flag
        )
        .await
        .map_err(|e| format!("{e}"))?;

    // Extract text from response
    let mut summary = String::new();
    for block in content_blocks {
        if let ContentBlock::Text { text } = block {
            summary.push_str(&text);
        }
    }

    // If streaming callback captured it but content_blocks didn't, use that
    if summary.is_empty() && !result_text.is_empty() {
        summary = result_text;
    }

    Ok(summary)
}

/// Synchronous fallback — compact messages without API call.
/// Used when no client is available (e.g., /compact command).
pub fn compact_messages(messages: &[Message], keep_recent: usize) -> Vec<Message> {
    if messages.len() <= keep_recent {
        return messages.to_vec();
    }

    let split_at = messages.len() - keep_recent;
    let old_messages = &messages[..split_at];
    let summary = build_fallback_summary(old_messages);

    let mut compacted = vec![Message::System(SystemMessage {
        content: summary,
        message_type: SystemMessageType::CompactBoundary,
    })];

    compacted.extend_from_slice(&messages[split_at..]);
    compacted
}
