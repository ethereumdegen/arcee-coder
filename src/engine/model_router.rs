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

    // If last turn used complex tools, stay heavy
    let heavy_tools = [
        "Write", "Edit", "Bash", "Agent",
    ];
    if last_tool_names
        .iter()
        .any(|t| heavy_tools.contains(&t.as_str()))
    {
        return TaskWeight::Heavy;
    }

    // If last turn was only read-only tools, might be light
    let light_tools = ["Read", "Glob", "Grep", "WebFetch", "AskUserQuestion"];
    let all_light = !last_tool_names.is_empty()
        && last_tool_names
            .iter()
            .all(|t| light_tools.contains(&t.as_str()));

    if all_light {
        // Check if the latest user message is short/simple
        if let Some(last_user_len) = last_user_message_len(messages) {
            if last_user_len < 200 {
                return TaskWeight::Light;
            }
        }
    }

    // If the conversation is getting long, use the thinking model for coherence
    if turn_count > 10 {
        return TaskWeight::Heavy;
    }

    // After a tool-only turn (continuing agentic loop), check what's happening
    if last_tool_names.is_empty() {
        // No tools last turn = assistant gave a text response, user replied
        // Probably needs reasoning
        return TaskWeight::Heavy;
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
