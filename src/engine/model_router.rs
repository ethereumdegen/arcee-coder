/// Adaptive model routing: picks the right Arcee model based on task complexity.
///
/// Trinity-Mini: fast, cheap — for simple reads, globs, greps, short answers
/// Trinity-Large-Thinking: frontier reasoning — for complex code gen, multi-step, debugging

use serde::{Deserialize, Serialize};

/// The intensity level of a conversation turn, used to decide which model to route to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TurnIntensity {
    Low,
    High,
}

/// User-selectable routing intensity. Controls how aggressively we use the cheap model.
/// Switchable at runtime via `/intensity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Intensity {
    /// Always use the heavy/expensive model
    High,
    /// Balanced: use mini for reads/follow-ups, heavy for writes/reasoning (default)
    Medium,
    /// Prefer mini whenever possible, only use heavy for first turn and complex writes
    Low,
}

impl Default for Intensity {
    fn default() -> Self {
        Self::Medium
    }
}

impl Intensity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "high" => Some(Self::High),
            "medium" => Some(Self::Medium),
            "low" => Some(Self::Low),
            _ => None,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::High => "always use large-thinking model (expensive, best quality)",
            Self::Medium => "smart routing: mini for reads, large for writes (balanced)",
            Self::Low => "prefer mini whenever possible (cheapest, faster)",
        }
    }
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
    intensity: Intensity,
) -> String {
    // If auto-routing is off, always use the configured model
    if !auto_route {
        return configured_model.to_string();
    }

    // If user explicitly set a non-standard model, respect it
    if configured_model != MODEL_HEAVY
        && configured_model != MODEL_LIGHT
        && !configured_model.is_empty()
    {
        return configured_model.to_string();
    }

    // High intensity: always use the big model
    if intensity == Intensity::High {
        return MODEL_HEAVY.to_string();
    }

    let turn_intensity = classify_turn(messages, last_tool_names, turn_count, intensity);

    match turn_intensity {
        TurnIntensity::Low => MODEL_LIGHT.to_string(),
        TurnIntensity::High => MODEL_HEAVY.to_string(),
    }
}

/// Tools that are simple read-only operations — mini handles these fine.
const LIGHT_TOOLS: &[&str] = &[
    "Read", "Glob", "Grep", "WebFetch", "TaskList", "TaskGet", "LSP",
];

/// Tools that need reasoning — always use heavy.
const HEAVY_TOOLS: &[&str] = &[
    "Edit", "Write", "Bash", "Agent", "NotebookEdit",
];

/// Classify the upcoming turn based on heuristics + intensity preference.
fn classify_turn(
    messages: &[crate::messages::types::Message],
    last_tool_names: &[String],
    turn_count: usize,
    intensity: Intensity,
) -> TurnIntensity {
    // First turn is always high (need to understand the full request)
    if turn_count == 0 {
        return TurnIntensity::High;
    }

    match intensity {
        Intensity::High => TurnIntensity::High,

        Intensity::Medium => {
            // After heavy tools (write/edit/bash), stay heavy for reasoning
            if last_tool_names.iter().any(|n| HEAVY_TOOLS.contains(&n.as_str())) {
                return TurnIntensity::High;
            }

            // After only light tools (read/glob/grep), mini can continue
            if !last_tool_names.is_empty()
                && last_tool_names.iter().all(|n| LIGHT_TOOLS.contains(&n.as_str()))
            {
                return TurnIntensity::Low;
            }

            // Short user follow-ups: mini
            if let Some(len) = last_user_message_len(messages) {
                if len < 120 {
                    return TurnIntensity::Low;
                }
            }

            TurnIntensity::High
        }

        Intensity::Low => {
            // Low intensity: only use heavy after Edit/Write/Agent (the really critical ones)
            if last_tool_names.iter().any(|n| matches!(n.as_str(), "Edit" | "Write" | "Agent")) {
                return TurnIntensity::High;
            }

            // Everything else: mini
            TurnIntensity::Low
        }
    }
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
