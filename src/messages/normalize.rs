use crate::api::types::{ChatMessage, FunctionCall, ToolCall};
use crate::messages::types::*;

/// Convert internal messages to Arcee/OpenAI chat format.
pub fn normalize_for_api(messages: &[Message]) -> Vec<ChatMessage> {
    let mut chat_messages = Vec::new();

    for msg in messages {
        match msg {
            Message::User(user_msg) => {
                // Collect text parts
                let text_parts: Vec<String> = user_msg
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        UserContent::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect();

                // Collect tool results
                let tool_results: Vec<_> = user_msg
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        UserContent::ToolResult {
                            tool_use_id,
                            content,
                            ..
                        } => Some((tool_use_id.clone(), content.clone())),
                        _ => None,
                    })
                    .collect();

                // If there are tool results, emit them as individual "tool" messages
                for (tool_call_id, content) in &tool_results {
                    chat_messages.push(ChatMessage::tool_result(tool_call_id, content));
                }

                // If there's text content (and no tool results, or mixed), emit user message
                if !text_parts.is_empty() && tool_results.is_empty() {
                    chat_messages.push(ChatMessage::user(text_parts.join("\n")));
                } else if !text_parts.is_empty() {
                    // Mixed: tool results already emitted, also emit user text
                    chat_messages.push(ChatMessage::user(text_parts.join("\n")));
                }
            }
            Message::Assistant(assistant_msg) => {
                let mut text_parts = Vec::new();
                let mut tool_calls = Vec::new();

                for block in &assistant_msg.content {
                    use crate::api::types::ContentBlock;
                    match block {
                        ContentBlock::Text { text } => {
                            text_parts.push(text.clone());
                        }
                        ContentBlock::ToolUse { id, name, input } => {
                            tool_calls.push(ToolCall {
                                id: id.clone(),
                                call_type: "function".to_string(),
                                function: FunctionCall {
                                    name: name.clone(),
                                    arguments: serde_json::to_string(input)
                                        .unwrap_or_else(|_| "{}".to_string()),
                                },
                            });
                        }
                        ContentBlock::Thinking { thinking } => {
                            // Preserve thinking blocks for providers that support
                            // extended thinking / chain-of-thought. Included as a
                            // prefixed text segment so the model can build on its
                            // prior reasoning.
                            if !thinking.is_empty() {
                                text_parts.push(format!("<thinking>\n{thinking}\n</thinking>"));
                            }
                        }
                    }
                }

                if !tool_calls.is_empty() {
                    // Assistant message with tool calls
                    let mut msg = ChatMessage::assistant_tool_calls(tool_calls);
                    if !text_parts.is_empty() {
                        msg.content =
                            Some(serde_json::Value::String(text_parts.join("\n")));
                    }
                    chat_messages.push(msg);
                } else if !text_parts.is_empty() {
                    chat_messages.push(ChatMessage::assistant_text(text_parts.join("\n")));
                }
            }
            Message::System(_) => {
                // System messages handled separately (prepended by client)
            }
        }
    }

    chat_messages
}
