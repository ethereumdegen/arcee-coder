/// Adaptive model routing: picks the right Arcee model based on task complexity.
///
/// Trinity-Mini: fast, cheap — for simple reads, globs, greps, short answers
/// Trinity-Large-Thinking: frontier reasoning — for complex code gen, multi-step, debugging

/// The "weight" of a conversation turn, used to decide which model to route to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskWeight {
    Light,
    Heavy,
}

/// Models available for routing.
pub const MODEL_LIGHT: &str = "trinity-mini";
pub const MODEL_HEAVY: &str = "trinity-large-thinking";

/// Decide the model to use for the next API call based on conversation state.
pub fn pick_model(
    configured_model: &str,
    messages: &[crate::messages::types::Message],
    last_tool_names: &[String],
    turn_count: usize,
    auto_route: bool,
) -> String {
    // If auto-routing is off, always use the configured model
    if !auto_route {
        return configured_model.to_string();
    }

    // If user explicitly set a specific model, respect it
    if configured_model != MODEL_HEAVY
        && configured_model != MODEL_LIGHT
        && !configured_model.is_empty()
    {
        return configured_model.to_string();
    }

    let weight = classify_task(messages, last_tool_names, turn_count);

    match weight {
        TaskWeight::Light => MODEL_LIGHT.to_string(),
        TaskWeight::Heavy => MODEL_HEAVY.to_string(),
    }
}

/// Classify the upcoming task based on heuristics.
fn classify_task(
    messages: &[crate::messages::types::Message],
    last_tool_names: &[String],
    turn_count: usize,
) -> TaskWeight {
    // First turn is always heavy (need to understand the full request)
    if turn_count == 0 {
        return TaskWeight::Heavy;
    }

    // During an active agentic loop (model made tool calls last turn and we're
    // continuing), ALWAYS use the heavy model. Mini is too weak for multi-step
    // tool use — it generates malformed arguments and asks unnecessary questions.
    if !last_tool_names.is_empty() {
        return TaskWeight::Heavy;
    }

    // After a text-only turn (no tools), user replied with something new.
    // Use mini only for trivially short follow-ups.
    if let Some(last_user_len) = last_user_message_len(messages) {
        if last_user_len < 80 && turn_count <= 3 {
            return TaskWeight::Light;
        }
    }

    // Default: heavy (better to be safe)
    TaskWeight::Heavy
}

fn last_user_message_len(messages: &[crate::messages::types::Message]) -> Option<usize> {
    for msg in messages.iter().rev() {
        if let crate::messages::types::Message::User(u) = msg {
            let total: usize = u
                .content
                .iter()
                .map(|c| match c {
                    crate::messages::types::UserContent::Text(t) => t.len(),
                    _ => 0,
                })
                .sum();
            return Some(total);
        }
    }
    None
}
