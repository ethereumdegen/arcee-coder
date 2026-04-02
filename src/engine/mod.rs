pub mod compact;
pub mod context;
pub mod cost;
pub mod model_router;

use crate::api::client::ApiClient;
use crate::api::types::*;
use crate::config::Config;
use crate::messages::normalize::normalize_for_api;
use crate::messages::types::*;
use crate::permissions;
use crate::tools::{ToolContext, ToolRegistry};
use crate::ui::thinking::ThinkingIndicator;
use anyhow::Result;
use colored::Colorize;
use cost::CostTracker;

/// Maximum characters in a single tool result before truncation.
const MAX_TOOL_RESULT_CHARS: usize = 50_000;
/// Token threshold to trigger auto-compaction (rough estimate).
const AUTO_COMPACT_TOKEN_THRESHOLD: u64 = 80_000;
/// Messages to keep when auto-compacting.
const AUTO_COMPACT_KEEP_RECENT: usize = 10;
/// Max consecutive max_tokens recovery attempts.
const MAX_OUTPUT_RECOVERY_ATTEMPTS: u32 = 3;

/// Run the main query loop: send messages to the API, execute tool calls, repeat.
pub async fn query_loop(
    client: &ApiClient,
    messages: &mut Vec<Message>,
    tools: &ToolRegistry,
    config: &Config,
    cost_tracker: &mut CostTracker,
) -> Result<()> {
    let system_prompt = context::build_system_prompt(&config.cwd, &config.model);
    let tool_defs = tools.definitions();
    let tool_context = ToolContext {
        cwd: config.cwd.clone(),
        permission_mode: config.permission_mode,
    };

    let mut turns = 0;
    let mut max_tokens_recoveries: u32 = 0;
    let mut last_tool_names: Vec<String> = Vec::new();

    loop {
        // Auto-compaction check
        let estimated_tokens = compact::estimate_tokens(messages);
        if estimated_tokens > AUTO_COMPACT_TOKEN_THRESHOLD && messages.len() > AUTO_COMPACT_KEEP_RECENT + 2 {
            let before = messages.len();
            *messages = compact::compact_messages(messages, AUTO_COMPACT_KEEP_RECENT);
            if config.verbose {
                eprintln!(
                    "{}",
                    format!(
                        "[auto-compact: {} → {} messages, ~{} tokens]",
                        before,
                        messages.len(),
                        compact::estimate_tokens(messages)
                    )
                    .dimmed()
                );
            }
        }

        // Adaptive model selection
        let model = model_router::pick_model(
            &config.model,
            messages,
            &last_tool_names,
            turns,
            config.auto_model_routing,
        );

        if config.verbose {
            eprintln!("{}", format!("[model: {model}]").dimmed());
        }

        // Build API messages
        let api_messages = normalize_for_api(messages);

        // Stream the response with thinking indicator
        let thinking = std::sync::Mutex::new(Some(ThinkingIndicator::start()));

        let mut on_text = |text: &str| {
            // Stop the thinking indicator on first text output
            if let Ok(mut guard) = thinking.lock() {
                if let Some(indicator) = guard.take() {
                    indicator.stop();
                }
            }
            print!("{text}");
            use std::io::Write;
            let _ = std::io::stdout().flush();
        };

        let mut on_tool_start = |_id: &str, name: &str| {
            // Stop thinking indicator if a tool starts before any text
            if let Ok(mut guard) = thinking.lock() {
                if let Some(indicator) = guard.take() {
                    indicator.stop();
                }
            }
            println!("\n{} {}", "Tool:".cyan().bold(), name.cyan());
        };

        let result = client
            .send_message_with_model(
                &model,
                &system_prompt,
                api_messages,
                tool_defs.clone(),
                config.max_tokens,
                &mut on_text,
                &mut on_tool_start,
            )
            .await;

        // Ensure thinking indicator is stopped after streaming completes
        if let Ok(mut guard) = thinking.lock() {
            if let Some(indicator) = guard.take() {
                indicator.stop();
            }
        }

        let (content_blocks, stop_reason, usage) = match result {
            Ok(r) => r,
            Err(crate::api::errors::ApiError::ContextTooLong(_)) => {
                // Try compacting and retrying
                eprintln!(
                    "{}",
                    "[context too long, compacting...]".yellow()
                );
                *messages = compact::compact_messages(messages, 6);
                continue;
            }
            Err(e) => return Err(anyhow::anyhow!("API error: {e}")),
        };

        cost_tracker.add_usage(&usage);

        // Store assistant message
        let assistant_msg = AssistantMessage {
            content: content_blocks.clone(),
            stop_reason: stop_reason.clone(),
            usage,
        };
        messages.push(Message::Assistant(assistant_msg));

        // Check if we're done
        if stop_reason == StopReason::EndTurn {
            println!();
            break;
        }

        if stop_reason == StopReason::MaxTokens {
            max_tokens_recoveries += 1;
            if max_tokens_recoveries <= MAX_OUTPUT_RECOVERY_ATTEMPTS {
                // Inject a continuation prompt
                if config.verbose {
                    eprintln!(
                        "{}",
                        format!(
                            "[max_tokens hit, recovery attempt {}/{}]",
                            max_tokens_recoveries, MAX_OUTPUT_RECOVERY_ATTEMPTS
                        )
                        .dimmed()
                    );
                }
                messages.push(Message::user_text(
                    "Continue from where you left off. Do not repeat what you already said.",
                ));
                turns += 1;
                continue;
            } else {
                println!("\n{}", "(max tokens reached, could not recover)".yellow());
                break;
            }
        }

        max_tokens_recoveries = 0;

        // Execute tool calls
        if stop_reason == StopReason::ToolUse {
            let tool_uses: Vec<_> = content_blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolUse { id, name, input } => {
                        Some((id.clone(), name.clone(), input.clone()))
                    }
                    _ => None,
                })
                .collect();

            last_tool_names = tool_uses.iter().map(|(_, n, _)| n.clone()).collect();
            let mut results = Vec::new();

            for (id, name, input) in &tool_uses {
                let tool = match tools.get(name) {
                    Some(t) => t,
                    None => {
                        results.push((id.clone(), format!("Unknown tool: {name}"), true));
                        continue;
                    }
                };

                let is_read_only = tool.is_read_only(input);

                // Check permissions
                let perm = permissions::check_tool_permission(
                    name,
                    input,
                    is_read_only,
                    config.permission_mode,
                    &config.allow_rules,
                    &config.deny_rules,
                    config.permission_strictness,
                );

                let allowed = match perm {
                    permissions::PermissionResult::Allow => true,
                    permissions::PermissionResult::Deny(reason) => {
                        results.push((id.clone(), format!("Permission denied: {reason}"), true));
                        continue;
                    }
                    permissions::PermissionResult::Ask => {
                        let description = input["description"].as_str();
                        match permissions::prompt_user_permission(name, input, description) {
                            Ok(true) => true,
                            Ok(false) => {
                                results.push((
                                    id.clone(),
                                    "User denied permission".to_string(),
                                    true,
                                ));
                                continue;
                            }
                            Err(e) => {
                                results.push((
                                    id.clone(),
                                    format!("Permission prompt error: {e}"),
                                    true,
                                ));
                                continue;
                            }
                        }
                    }
                };

                if allowed {
                    if config.verbose {
                        let input_str = serde_json::to_string_pretty(input).unwrap_or_default();
                        println!("{}", input_str.dimmed());
                    }

                    match tool.call(input.clone(), &tool_context).await {
                        Ok(mut result) => {
                            // Truncate large tool results
                            if result.content.len() > MAX_TOOL_RESULT_CHARS {
                                let truncated =
                                    crate::tools::path_safety::safe_truncate(
                                        &result.content,
                                        MAX_TOOL_RESULT_CHARS,
                                    );
                                result.content = format!(
                                    "{}\n\n... (truncated, {} total chars)",
                                    truncated,
                                    result.content.len()
                                );
                            }

                            let preview: String =
                                result.content.chars().take(500).collect();
                            if result.is_error {
                                println!("{}", format!("  Error: {preview}").red());
                            } else {
                                println!("{}", format!("  {preview}").dimmed());
                            }
                            if result.content.len() > 500 {
                                println!(
                                    "{}",
                                    format!("  ... ({} total chars)", result.content.len())
                                        .dimmed()
                                );
                            }

                            results.push((id.clone(), result.content, result.is_error));
                        }
                        Err(e) => {
                            let error_msg = format!("Tool execution error: {e}");
                            println!("{}", format!("  {error_msg}").red());
                            results.push((id.clone(), error_msg, true));
                        }
                    }
                }
            }

            // Add tool results as user message
            if !results.is_empty() {
                messages.push(Message::tool_results(results));
            }
        } else {
            last_tool_names.clear();
        }

        turns += 1;
        if turns >= config.max_turns {
            println!(
                "\n{}",
                format!("Max turns ({}) reached.", config.max_turns).yellow()
            );
            break;
        }

        // Check budget
        if let Some(budget) = config.budget_usd {
            let current_cost = cost_tracker.estimate_cost_usd(&config.model);
            if current_cost >= budget {
                println!(
                    "\n{}",
                    format!("Budget limit (${budget:.2}) reached.").yellow()
                );
                break;
            }
        }
    }

    Ok(())
}
