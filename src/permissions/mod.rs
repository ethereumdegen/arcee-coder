use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
    #[default]
    Default,
    Auto,
    Plan,
    Bypass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResult {
    Allow,
    Deny(String),
    Ask,
}

/// Check whether a tool invocation should be allowed, denied, or needs user confirmation.
pub fn check_tool_permission(
    tool_name: &str,
    input: &serde_json::Value,
    is_read_only: bool,
    mode: PermissionMode,
    allow_rules: &[crate::config::PermissionRule],
    deny_rules: &[crate::config::PermissionRule],
) -> PermissionResult {
    // Check deny rules first
    for rule in deny_rules {
        if matches_rule(&rule.tool_name, rule.pattern.as_deref(), tool_name, input) {
            return PermissionResult::Deny(format!("Denied by rule: {}", rule.tool_name));
        }
    }

    // Check allow rules
    for rule in allow_rules {
        if matches_rule(&rule.tool_name, rule.pattern.as_deref(), tool_name, input) {
            return PermissionResult::Allow;
        }
    }

    // Apply permission mode
    match mode {
        PermissionMode::Bypass => PermissionResult::Allow,
        PermissionMode::Default => {
            if is_read_only {
                PermissionResult::Allow
            } else {
                PermissionResult::Ask
            }
        }
        PermissionMode::Auto => {
            // In auto mode, allow read-only and common safe operations
            if is_read_only {
                PermissionResult::Allow
            } else {
                PermissionResult::Ask
            }
        }
        PermissionMode::Plan => {
            // Plan mode: ask during planning, allow during execution
            // For now, treat as default
            if is_read_only {
                PermissionResult::Allow
            } else {
                PermissionResult::Ask
            }
        }
    }
}

fn matches_rule(
    rule_tool: &str,
    rule_pattern: Option<&str>,
    tool_name: &str,
    _input: &serde_json::Value,
) -> bool {
    if rule_tool != tool_name && rule_tool != "*" {
        return false;
    }
    // If there's a pattern, we'd do more sophisticated matching
    // For now, tool name match is sufficient
    rule_pattern.is_none() || rule_pattern == Some("*")
}

/// Prompt the user for permission to execute a tool.
pub fn prompt_user_permission(
    tool_name: &str,
    input: &serde_json::Value,
    description: Option<&str>,
) -> io::Result<bool> {
    let desc = description.unwrap_or("");
    let input_summary = summarize_input(tool_name, input);

    println!(
        "\n{} {} {}",
        "Permission required:".yellow().bold(),
        tool_name.cyan().bold(),
        if desc.is_empty() {
            String::new()
        } else {
            format!("— {desc}")
        }
    );

    if !input_summary.is_empty() {
        println!("  {}", input_summary.dimmed());
    }

    print!("{} ", "[y/N]".yellow());
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    Ok(response.trim().eq_ignore_ascii_case("y"))
}

fn summarize_input(tool_name: &str, input: &serde_json::Value) -> String {
    match tool_name {
        "Bash" => input["command"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        "Read" => input["file_path"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        "Write" => format!(
            "{} ({} bytes)",
            input["file_path"].as_str().unwrap_or(""),
            input["content"].as_str().map_or(0, |s| s.len())
        ),
        "Edit" => input["file_path"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        "Glob" => input["pattern"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        "Grep" => format!(
            "{} in {}",
            input["pattern"].as_str().unwrap_or(""),
            input["path"].as_str().unwrap_or(".")
        ),
        _ => serde_json::to_string(input)
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect(),
    }
}
