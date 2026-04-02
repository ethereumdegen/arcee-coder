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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Maximum characters in a single tool result before truncation.
const MAX_TOOL_RESULT_CHARS: usize = 50_000;
/// Token threshold to trigger auto-compaction (rough estimate).
const AUTO_COMPACT_TOKEN_THRESHOLD: u64 = 80_000;
/// Messages to keep when auto-compacting.
const AUTO_COMPACT_KEEP_RECENT: usize = 10;
/// Max consecutive max_tokens recovery attempts.
const MAX_OUTPUT_RECOVERY_ATTEMPTS: u32 = 3;
/// Max retries for transient API errors (rate-limit, server errors, network).
const MAX_API_RETRIES: u32 = 3;
/// Max consecutive empty end_turn responses before giving up.
const MAX_EMPTY_END_TURN: u32 = 3;
/// Max times the same tool call can repeat before we inject a stuck-loop warning.
const MAX_REPEATED_TOOL_CALLS: usize = 3;

/// Run the main query loop: send messages to the API, execute tool calls, repeat.
pub async fn query_loop(
    client: &ApiClient,
    messages: &mut Vec<Message>,
    tools: &ToolRegistry,
    config: &Config,
    cost_tracker: &mut CostTracker,
    tool_context: &ToolContext,
    escape_flag: &Arc<AtomicBool>,
) -> Result<()> {
    let system_prompt = context::build_system_prompt(&config.cwd, &config.model);
    let tool_defs = tools.definitions();

    let mut turns = 0;
    let mut max_tokens_recoveries: u32 = 0;
    let mut last_tool_names: Vec<String> = Vec::new();
    // Track repeated tool calls to detect stuck loops
    let mut last_tool_signature: Option<String> = None;
    let mut repeated_tool_count: usize = 0;
    // Track consecutive empty end_turn responses
    let mut empty_end_turn_count: u32 = 0;
    // Model override when recovering from empty responses (fallback to heavy model)
    let mut empty_response_fallback_model: Option<String> = None;

    loop {
        // Check escape flag
        if escape_flag.load(Ordering::Relaxed) {
            escape_flag.store(false, Ordering::Relaxed);
            println!();
            eprintln!(
                "{}",
                format!("[interrupted by user at turn {turns}]").yellow()
            );
            break;
        }

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

        // Adaptive model selection — override with fallback model if recovering from empty responses
        let model = match empty_response_fallback_model.take() {
            Some(fallback) => fallback,
            None => model_router::pick_model(
                &config.model,
                messages,
                &last_tool_names,
                turns,
                config.auto_model_routing,
                config.intensity,
            ),
        };

        if config.verbose {
            eprintln!("{}", format!("[model: {model}]").dimmed());
        }

        // Retry loop for transient API errors
        let mut api_retries = 0u32;
        let (content_blocks, stop_reason, usage) = loop {
            // Build API messages (rebuilt each retry in case compaction changed them)
            let api_messages = normalize_for_api(messages);

            // Stream the response with thinking indicator
            let thinking = std::sync::Mutex::new(Some(ThinkingIndicator::start()));

            let mut on_text = |text: &str| {
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
                    Some(escape_flag),
                )
                .await;

            // Ensure thinking indicator is stopped after streaming completes
            if let Ok(mut guard) = thinking.lock() {
                if let Some(indicator) = guard.take() {
                    indicator.stop();
                }
            }

            match result {
                Ok(r) => break r,
                Err(crate::api::errors::ApiError::ContextTooLong(_)) => {
                    eprintln!(
                        "{}",
                        "[context too long, compacting...]".yellow()
                    );
                    *messages = compact::compact_messages(messages, 6);
                    continue; // retry immediately after compaction
                }
                Err(e) if e.is_retryable() && api_retries < MAX_API_RETRIES => {
                    api_retries += 1;
                    let delay = match &e {
                        crate::api::errors::ApiError::RateLimit {
                            retry_after_secs: Some(s),
                        } => *s,
                        _ => 2u64.pow(api_retries), // exponential backoff
                    };
                    eprintln!(
                        "{}",
                        format!(
                            "[transient API error: {e} — retrying in {delay}s ({api_retries}/{MAX_API_RETRIES})]"
                        )
                        .yellow()
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    continue;
                }
                Err(e) => {
                    eprintln!(
                        "{}",
                        format!("[query_loop exiting: API error after {api_retries} retries: {e}]")
                            .red()
                    );
                    return Err(anyhow::anyhow!("API error: {e}"));
                }
            }
        };

        cost_tracker.add_usage(&usage);

        // Check if streaming was interrupted by ESC — break before processing tool calls
        if escape_flag.load(Ordering::Relaxed) {
            escape_flag.store(false, Ordering::Relaxed);
            println!();
            eprintln!(
                "{}",
                format!("[interrupted by user during streaming at turn {turns}]").yellow()
            );
            break;
        }

        // Store assistant message
        let assistant_msg = AssistantMessage {
            content: content_blocks.clone(),
            stop_reason: stop_reason.clone(),
            usage,
        };
        messages.push(Message::Assistant(assistant_msg));

        // Check if the response has any actual content
        let has_text = content_blocks.iter().any(|b| matches!(b, ContentBlock::Text { text } if !text.trim().is_empty()));
        let has_tool_calls = content_blocks.iter().any(|b| matches!(b, ContentBlock::ToolUse { .. }));

        // Check if we're done
        if stop_reason == StopReason::EndTurn {
            if !has_text && !has_tool_calls && turns > 0 {
                empty_end_turn_count += 1;

                if empty_end_turn_count == 1 {
                    // Strategy 1: Inject a nudge message to prompt the model to continue
                    eprintln!(
                        "{}",
                        format!(
                            "[warning: empty end_turn (attempt {}/{}), injecting nudge]",
                            empty_end_turn_count, MAX_EMPTY_END_TURN
                        )
                        .yellow()
                    );
                    messages.push(Message::user_text(
                        "You returned an empty response. Please continue your analysis based on the tool results above. \
                         Summarize what you found and proceed with the next step.",
                    ));
                    turns += 1;
                    continue;
                } else if empty_end_turn_count == 2 {
                    // Strategy 2: Fall back to the heavy model (mini often causes empty responses)
                    let current_model = model_router::pick_model(
                        &config.model,
                        messages,
                        &last_tool_names,
                        turns,
                        config.auto_model_routing,
                        config.intensity,
                    );
                    let fallback = if current_model == model_router::MODEL_HEAVY {
                        None // already on heavy, just retry with nudge
                    } else {
                        Some(model_router::MODEL_HEAVY.to_string())
                    };
                    if let Some(ref fb) = fallback {
                        eprintln!(
                            "{}",
                            format!(
                                "[warning: empty end_turn (attempt {}/{}), switching to '{}']",
                                empty_end_turn_count, MAX_EMPTY_END_TURN, fb
                            )
                            .yellow()
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format!(
                                "[warning: empty end_turn (attempt {}/{}), retrying with nudge]",
                                empty_end_turn_count, MAX_EMPTY_END_TURN
                            )
                            .yellow()
                        );
                    }
                    empty_response_fallback_model = fallback;
                    messages.push(Message::user_text(
                        "You returned an empty response again. Please respond to the user's request. \
                         Analyze the conversation and tool results and provide your answer.",
                    ));
                    turns += 1;
                    continue;
                } else {
                    // Exhausted all recovery strategies — give up
                    eprintln!(
                        "{}",
                        format!(
                            "[loop exit: empty end_turn {} times at turn {turns}, recovery exhausted]",
                            empty_end_turn_count
                        )
                        .dimmed()
                    );
                }
            } else {
                empty_end_turn_count = 0;
                empty_response_fallback_model = None;
                println!();
                eprintln!(
                    "{}",
                    format!("[loop exit: model returned end_turn at turn {turns}]").dimmed()
                );
            }
            break;
        }
        empty_end_turn_count = 0;
        empty_response_fallback_model = None;

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
                eprintln!(
                    "{}",
                    format!(
                        "[loop exit: max_tokens hit {MAX_OUTPUT_RECOVERY_ATTEMPTS} times, unrecoverable at turn {turns}]"
                    )
                    .red()
                );
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

            // Stuck-loop detection: hash current tool calls and compare to previous turn
            let current_signature = {
                let mut sig_parts: Vec<String> = tool_uses
                    .iter()
                    .map(|(_, name, input)| format!("{}:{}", name, input))
                    .collect();
                sig_parts.sort();
                sig_parts.join("|")
            };

            if Some(&current_signature) == last_tool_signature.as_ref() {
                repeated_tool_count += 1;
            } else {
                repeated_tool_count = 1;
                last_tool_signature = Some(current_signature);
            }

            if repeated_tool_count >= MAX_REPEATED_TOOL_CALLS {
                eprintln!(
                    "{}",
                    format!(
                        "[stuck-loop detected: same tool call repeated {} times, injecting warning]",
                        repeated_tool_count
                    )
                    .yellow()
                );
                // Don't execute the tools again — inject a warning instead
                let tool_names: Vec<_> = tool_uses.iter().map(|(_, n, _)| n.as_str()).collect();
                let warning = format!(
                    "STOP: You have called the exact same tool(s) ({}) with the exact same arguments {} times in a row, getting the same result each time. \
                     This is a stuck loop. You MUST try a completely different approach. \
                     Do NOT retry the same command. Either:\n\
                     1. Fix the underlying issue (e.g. wrong parameters, missing config)\n\
                     2. Ask the user for help\n\
                     3. Explain what went wrong and stop",
                    tool_names.join(", "),
                    repeated_tool_count
                );
                // Add dummy tool results so the conversation stays valid
                let results: Vec<_> = tool_uses
                    .iter()
                    .map(|(id, _, _)| (id.clone(), warning.clone(), true))
                    .collect();
                messages.push(Message::tool_results(results));
                turns += 1;
                continue;
            }

            let mut results = Vec::new();

            for (id, name, input) in &tool_uses {
                // Check escape between tool calls
                if escape_flag.load(Ordering::Relaxed) {
                    escape_flag.store(false, Ordering::Relaxed);
                    println!();
                    eprintln!(
                        "{}",
                        format!("[interrupted by user during tool execution at turn {turns}]").yellow()
                    );
                    // Return partial results so conversation stays valid
                    results.push((id.clone(), "Interrupted by user".to_string(), true));
                    break;
                }

                let tool = match tools.get(name) {
                    Some(t) => t,
                    None => {
                        results.push((id.clone(), format!("Unknown tool: {name}"), true));
                        continue;
                    }
                };

                // Validate required parameters before executing
                if let Some(validation_error) = validate_tool_input(tool, input) {
                    eprintln!(
                        "{}",
                        format!("[tool '{name}' input validation failed: {validation_error}]")
                            .yellow()
                    );
                    results.push((id.clone(), validation_error, true));
                    continue;
                }

                let is_read_only = tool.is_read_only(input);

                // Read permission mode from shared state
                let current_mode = *tool_context.permission_mode.lock().await;

                // Check permissions
                let perm = permissions::check_tool_permission(
                    name,
                    input,
                    is_read_only,
                    current_mode,
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

                    match tool.call(input.clone(), tool_context).await {
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
            eprintln!(
                "{}",
                format!("[loop exit: max turns limit ({}) reached]", config.max_turns).red()
            );
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
                eprintln!(
                    "{}",
                    format!("[loop exit: budget ${budget:.2} exhausted (spent ${current_cost:.2})]")
                        .red()
                );
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

/// Validate tool input against the tool's JSON schema.
/// Returns Some(error_message) if validation fails, None if OK.
fn validate_tool_input(tool: &dyn crate::tools::Tool, input: &serde_json::Value) -> Option<String> {
    let schema = tool.input_schema();

    // Check required fields
    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        let mut missing: Vec<&str> = Vec::new();
        for field in required {
            if let Some(field_name) = field.as_str() {
                let is_missing = match input.get(field_name) {
                    None => true,
                    Some(serde_json::Value::Null) => true,
                    _ => false,
                };
                if is_missing {
                    missing.push(field_name);
                }
            }
        }
        if !missing.is_empty() {
            let schema_hint = schema
                .get("properties")
                .and_then(|p| p.as_object())
                .map(|props| {
                    missing
                        .iter()
                        .filter_map(|name| {
                            props.get(*name).and_then(|p| {
                                let typ = p.get("type").and_then(|t| t.as_str()).unwrap_or("any");
                                let desc = p.get("description").and_then(|d| d.as_str()).unwrap_or("");
                                Some(format!("  - {name} ({typ}): {desc}"))
                            })
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();

            return Some(format!(
                "Missing required parameter(s): {}. You MUST provide these parameters.\n{}",
                missing.join(", "),
                schema_hint
            ));
        }
    }

    // Check type constraints for provided fields
    if let (Some(properties), Some(input_obj)) = (
        schema.get("properties").and_then(|p| p.as_object()),
        input.as_object(),
    ) {
        for (key, value) in input_obj {
            if let Some(prop_schema) = properties.get(key) {
                if let Some(expected_type) = prop_schema.get("type").and_then(|t| t.as_str()) {
                    let type_ok = match expected_type {
                        "string" => value.is_string(),
                        "number" | "integer" => value.is_number(),
                        "boolean" => value.is_boolean(),
                        "array" => value.is_array(),
                        "object" => value.is_object(),
                        _ => true,
                    };
                    if !type_ok {
                        return Some(format!(
                            "Parameter '{key}' must be type '{expected_type}', got: {}",
                            value
                        ));
                    }
                }
            }
        }
    }

    None
}
