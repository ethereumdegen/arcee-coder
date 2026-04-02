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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionStrictness {
    High,
    #[default]
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResult {
    Allow,
    Deny(String),
    Ask,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum DangerLevel {
    Safe,
    Moderate,
    Destructive,
}

/// Classify the danger level of a bash command.
fn classify_bash_danger(command: &str) -> DangerLevel {
    // Split on shell operators and classify each sub-command, return the highest
    let sub_commands = split_shell_commands(command);

    sub_commands
        .iter()
        .map(|cmd| classify_single_command(cmd))
        .max()
        .unwrap_or(DangerLevel::Safe)
}

fn split_shell_commands(command: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(c);
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(c);
            }
            '&' if !in_single_quote && !in_double_quote => {
                if chars.peek() == Some(&'&') {
                    chars.next();
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        results.push(trimmed);
                    }
                    current.clear();
                } else {
                    current.push(c);
                }
            }
            '|' if !in_single_quote && !in_double_quote => {
                if chars.peek() == Some(&'|') {
                    chars.next();
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        results.push(trimmed);
                    }
                    current.clear();
                } else {
                    // Pipe — still part of the same logical command group,
                    // but we check the overall command as one unit
                    current.push(c);
                }
            }
            ';' if !in_single_quote && !in_double_quote => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    results.push(trimmed);
                }
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        results.push(trimmed);
    }

    results
}

fn classify_single_command(cmd: &str) -> DangerLevel {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return DangerLevel::Safe;
    }

    // Check destructive patterns first
    let destructive_patterns: &[&str] = &[
        "rm -rf",
        "rm -fr",
        "git push --force",
        "git push -f",
        "git reset --hard",
        "git clean -f",
        "drop table",
        "drop database",
        "dd if=",
        "mkfs",
        "truncate",
        "format ",
        "fdisk",
        ":(){ :|:& };:",
    ];

    let cmd_lower = cmd.to_lowercase();
    for pattern in destructive_patterns {
        if cmd_lower.contains(pattern) {
            return DangerLevel::Destructive;
        }
    }

    // Check safe commands
    let safe_commands: &[&str] = &[
        "ls", "cat", "head", "tail", "echo", "pwd", "whoami", "date", "wc", "sort", "uniq",
        "diff", "find", "grep", "rg", "which", "type", "file", "stat", "du", "df", "env",
        "printenv", "uname", "hostname", "tree", "less", "more",
    ];

    let first_word = cmd.split_whitespace().next().unwrap_or("");

    for safe in safe_commands {
        if first_word == *safe {
            return DangerLevel::Safe;
        }
    }

    // Check safe prefixes
    let safe_prefixes: &[&str] = &[
        "git log",
        "git status",
        "git diff",
        "git show",
        "git branch",
        "git remote",
        "cargo build",
        "cargo test",
        "cargo check",
        "cargo clippy",
        "cargo fmt",
        "cargo run",
        "npm test",
        "npm run",
        "python -c",
        "rustc --version",
        "node --version",
        "cd ",
    ];

    for prefix in safe_prefixes {
        if cmd.starts_with(prefix) || cmd == prefix.trim() {
            return DangerLevel::Safe;
        }
    }

    // Check moderate patterns
    let moderate_indicators: &[&str] = &[
        "rm", "mv", "chmod", "chown", "kill", "pkill", "sudo", "git push", "git reset",
        "git checkout --", "git restore .", "curl | sh", "wget", "npm install", "pip install",
        "apt", "brew install",
    ];

    for indicator in moderate_indicators {
        if cmd_lower.starts_with(indicator) || cmd_lower.contains(indicator) {
            return DangerLevel::Moderate;
        }
    }

    // Check for redirects
    if cmd.contains('>') || cmd.contains(">>") {
        return DangerLevel::Moderate;
    }

    // Unknown commands default to Moderate
    DangerLevel::Moderate
}

/// Determine whether a tool invocation should prompt the user for permission.
fn should_ask_permission(
    tool_name: &str,
    input: &serde_json::Value,
    is_read_only: bool,
    strictness: PermissionStrictness,
) -> bool {
    // Read-only tools never need permission
    if is_read_only {
        return false;
    }

    match strictness {
        PermissionStrictness::High => {
            // Always ask for non-read-only tools
            true
        }
        PermissionStrictness::Medium => {
            if tool_name == "Bash" {
                let command = input["command"].as_str().unwrap_or("");
                let danger = classify_bash_danger(command);
                // Only auto-allow Safe commands
                danger >= DangerLevel::Moderate
            } else {
                // Write / Edit always ask on Medium
                true
            }
        }
        PermissionStrictness::Low => {
            if tool_name == "Bash" {
                let command = input["command"].as_str().unwrap_or("");
                let danger = classify_bash_danger(command);
                // Only ask for Destructive
                danger >= DangerLevel::Destructive
            } else {
                // Write / Edit are allowed on Low
                false
            }
        }
    }
}

/// Check whether a tool invocation should be allowed, denied, or needs user confirmation.
pub fn check_tool_permission(
    tool_name: &str,
    input: &serde_json::Value,
    is_read_only: bool,
    mode: PermissionMode,
    allow_rules: &[crate::config::PermissionRule],
    deny_rules: &[crate::config::PermissionRule],
    strictness: PermissionStrictness,
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
        PermissionMode::Default | PermissionMode::Auto | PermissionMode::Plan => {
            if should_ask_permission(tool_name, input, is_read_only, strictness) {
                PermissionResult::Ask
            } else {
                PermissionResult::Allow
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_bash_danger_safe() {
        assert_eq!(classify_bash_danger("ls -la"), DangerLevel::Safe);
        assert_eq!(classify_bash_danger("git log --oneline"), DangerLevel::Safe);
        assert_eq!(classify_bash_danger("git status"), DangerLevel::Safe);
        assert_eq!(classify_bash_danger("git diff HEAD"), DangerLevel::Safe);
        assert_eq!(classify_bash_danger("cargo test"), DangerLevel::Safe);
        assert_eq!(classify_bash_danger("cargo build"), DangerLevel::Safe);
        assert_eq!(classify_bash_danger("echo hello"), DangerLevel::Safe);
        assert_eq!(classify_bash_danger("cat foo.txt"), DangerLevel::Safe);
        assert_eq!(classify_bash_danger("pwd"), DangerLevel::Safe);
        assert_eq!(classify_bash_danger("cd /tmp"), DangerLevel::Safe);
    }

    #[test]
    fn test_classify_bash_danger_moderate() {
        assert_eq!(classify_bash_danger("rm foo.txt"), DangerLevel::Moderate);
        assert_eq!(classify_bash_danger("git push origin main"), DangerLevel::Moderate);
        assert_eq!(classify_bash_danger("mv a.txt b.txt"), DangerLevel::Moderate);
        assert_eq!(classify_bash_danger("npm install express"), DangerLevel::Moderate);
        assert_eq!(classify_bash_danger("sudo apt update"), DangerLevel::Moderate);
    }

    #[test]
    fn test_classify_bash_danger_destructive() {
        assert_eq!(classify_bash_danger("rm -rf /"), DangerLevel::Destructive);
        assert_eq!(classify_bash_danger("rm -fr ."), DangerLevel::Destructive);
        assert_eq!(classify_bash_danger("git push --force"), DangerLevel::Destructive);
        assert_eq!(classify_bash_danger("git push -f origin main"), DangerLevel::Destructive);
        assert_eq!(classify_bash_danger("git reset --hard HEAD~1"), DangerLevel::Destructive);
        assert_eq!(classify_bash_danger("git clean -fd"), DangerLevel::Destructive);
    }

    #[test]
    fn test_classify_bash_danger_compound_commands() {
        // Compound: highest danger wins
        assert_eq!(
            classify_bash_danger("ls && rm -rf /tmp"),
            DangerLevel::Destructive
        );
        assert_eq!(
            classify_bash_danger("git status; git log"),
            DangerLevel::Safe
        );
        assert_eq!(
            classify_bash_danger("cargo test && git push"),
            DangerLevel::Moderate
        );
    }

    #[test]
    fn test_should_ask_permission_read_only() {
        let input = serde_json::json!({});
        // Read-only tools never ask, regardless of strictness
        assert!(!should_ask_permission("Read", &input, true, PermissionStrictness::High));
        assert!(!should_ask_permission("Read", &input, true, PermissionStrictness::Medium));
        assert!(!should_ask_permission("Read", &input, true, PermissionStrictness::Low));
    }

    #[test]
    fn test_should_ask_permission_bash_safe() {
        let input = serde_json::json!({"command": "git log"});
        assert!(should_ask_permission("Bash", &input, false, PermissionStrictness::High));
        assert!(!should_ask_permission("Bash", &input, false, PermissionStrictness::Medium));
        assert!(!should_ask_permission("Bash", &input, false, PermissionStrictness::Low));
    }

    #[test]
    fn test_should_ask_permission_bash_moderate() {
        let input = serde_json::json!({"command": "rm foo.txt"});
        assert!(should_ask_permission("Bash", &input, false, PermissionStrictness::High));
        assert!(should_ask_permission("Bash", &input, false, PermissionStrictness::Medium));
        assert!(!should_ask_permission("Bash", &input, false, PermissionStrictness::Low));
    }

    #[test]
    fn test_should_ask_permission_bash_destructive() {
        let input = serde_json::json!({"command": "rm -rf /"});
        assert!(should_ask_permission("Bash", &input, false, PermissionStrictness::High));
        assert!(should_ask_permission("Bash", &input, false, PermissionStrictness::Medium));
        assert!(should_ask_permission("Bash", &input, false, PermissionStrictness::Low));
    }

    #[test]
    fn test_should_ask_permission_write_edit() {
        let input = serde_json::json!({"file_path": "/tmp/foo"});
        // Write/Edit: ask on High and Medium, not Low
        assert!(should_ask_permission("Write", &input, false, PermissionStrictness::High));
        assert!(should_ask_permission("Write", &input, false, PermissionStrictness::Medium));
        assert!(!should_ask_permission("Write", &input, false, PermissionStrictness::Low));

        assert!(should_ask_permission("Edit", &input, false, PermissionStrictness::High));
        assert!(should_ask_permission("Edit", &input, false, PermissionStrictness::Medium));
        assert!(!should_ask_permission("Edit", &input, false, PermissionStrictness::Low));
    }

    #[test]
    fn test_check_tool_permission_integration() {
        // Medium + Bash("ls") → Allow
        let input = serde_json::json!({"command": "ls"});
        let result = check_tool_permission(
            "Bash", &input, false,
            PermissionMode::Default, &[], &[],
            PermissionStrictness::Medium,
        );
        assert_eq!(result, PermissionResult::Allow);

        // Medium + Bash("rm foo") → Ask
        let input = serde_json::json!({"command": "rm foo"});
        let result = check_tool_permission(
            "Bash", &input, false,
            PermissionMode::Default, &[], &[],
            PermissionStrictness::Medium,
        );
        assert_eq!(result, PermissionResult::Ask);

        // Bypass mode always allows
        let input = serde_json::json!({"command": "rm -rf /"});
        let result = check_tool_permission(
            "Bash", &input, false,
            PermissionMode::Bypass, &[], &[],
            PermissionStrictness::High,
        );
        assert_eq!(result, PermissionResult::Allow);
    }
}
