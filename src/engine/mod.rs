pub mod compact;
pub mod context;
pub mod cost;
pub mod model_router;

use crate::api::client::ApiClient;
use crate::api::types::*;
use crate::config::Config;
use crate::messages::normalize::normalize_for_api;
use crate::messages::types::*;
use crate::output::{OutputFormat, StreamEvent};
use crate::permissions;
use crate::tools::{ToolContext, ToolRegistry};
use crate::ui::bridge::UiBridge;
use crate::ui::events::StatusLevel;
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

/// Helper for outputting messages either through the bridge or direct println.
struct Output<'a> {
    bridge: Option<&'a UiBridge>,
}

impl<'a> Output<'a> {
    fn new(bridge: Option<&'a UiBridge>) -> Self {
        Self { bridge }
    }

    fn status_dim(&self, msg: &str) {
        if let Some(b) = self.bridge {
            b.status(msg, StatusLevel::Dim);
        } else {
            eprintln!("{}", msg.dimmed());
        }
    }

    fn status_warn(&self, msg: &str) {
        if let Some(b) = self.bridge {
            b.status(msg, StatusLevel::Warning);
        } else {
            eprintln!("{}", msg.yellow());
        }
    }

    fn status_error(&self, msg: &str) {
        if let Some(b) = self.bridge {
            b.status(msg, StatusLevel::Error);
        } else {
            eprintln!("{}", msg.red());
        }
    }

    fn tool_result(&self, preview: &str, is_error: bool) {
        if let Some(b) = self.bridge {
            b.status(
                &if is_error {
                    format!("  Error: {preview}")
                } else {
                    format!("  {preview}")
                },
                if is_error {
                    StatusLevel::Error
                } else {
                    StatusLevel::Dim
                },
            );
        } else if is_error {
            println!("{}", format!("  Error: {preview}").red());
        } else {
            println!("{}", format!("  {preview}").dimmed());
        }
    }

    fn newline(&self) {
        if self.bridge.is_none() {
            println!();
        }
    }
}

/// Run the main query loop: send messages to the API, execute tool calls, repeat.
pub async fn query_loop(
    client: &ApiClient,
    messages: &mut Vec<Message>,
    tools: &ToolRegistry,
    config: &Config,
    cost_tracker: &mut CostTracker,
    tool_context: &ToolContext,
    escape_flag: &Arc<AtomicBool>,
    system_prompt_override: Option<&str>,
    bridge: Option<&UiBridge>,
    output_format: OutputFormat,
) -> Result<()> {
    let system_prompt = match system_prompt_override {
        Some(prompt) => prompt.to_string(),
        None => context::build_system_prompt(&config.cwd, &config.model),
    };
    let tool_defs = tools.definitions();
    let out = Output::new(bridge);

    let mut turns = 0;
    let mut max_tokens_recoveries: u32 = 0;
    let mut last_tool_names: Vec<String> = Vec::new();
    let mut last_tool_signature: Option<String> = None;
    let mut repeated_tool_count: usize = 0;
    let mut empty_end_turn_count: u32 = 0;
    let mut empty_response_fallback_model: Option<String> = None;
    let mut validation_failure_fallback_model: Option<String> = None;

    loop {
        // Check escape flag
        if escape_flag.load(Ordering::Relaxed) {
            escape_flag.store(false, Ordering::Relaxed);
            out.newline();
            out.status_warn(&format!("[interrupted by user at turn {turns}]"));
            break;
        }

        // Check for completed background tasks and inject notifications
        {
            let mut bg = tool_context.background_tasks.lock().await;
            let completed = bg.drain_completed();
            if !completed.is_empty() {
                let mut notification_parts = Vec::new();
                for task in &completed {
                    let elapsed = match task.completed_at {
                        Some(end) => end.duration_since(task.started_at).as_secs_f64(),
                        None => task.started_at.elapsed().as_secs_f64(),
                    };
                    let status = task.status.as_str();
                    let result = task.result.as_deref().unwrap_or("(no output)");
                    notification_parts.push(format!(
                        "<task_notification>\n\
                         <task_id>{}</task_id>\n\
                         <status>{status}</status>\n\
                         <description>{}</description>\n\
                         <duration>{elapsed:.1}s</duration>\n\
                         <result>\n{result}\n</result>\n\
                         </task_notification>",
                        task.id, task.description
                    ));

                    if let Some(b) = bridge {
                        b.send(crate::ui::events::UiEvent::BackgroundTaskCompleted {
                            id: task.id.clone(),
                            status: status.to_string(),
                            duration_secs: elapsed,
                        });
                    } else {
                        eprintln!(
                            "{}",
                            format!(
                                "  [Background task #{} ({}) {} in {:.1}s]",
                                task.id, task.description, status, elapsed
                            )
                            .cyan()
                        );
                    }
                }
                let notification_msg = notification_parts.join("\n\n");
                messages.push(Message::user_text(&notification_msg));
            }
        }

        // Auto-compaction check
        let estimated_tokens = compact::estimate_tokens(messages);
        if estimated_tokens > AUTO_COMPACT_TOKEN_THRESHOLD
            && messages.len() > AUTO_COMPACT_KEEP_RECENT + 2
        {
            let before = messages.len();
            out.status_dim("[auto-compacting conversation...]");
            crate::hooks::run_event_hooks(
                &config.hooks,
                "PreCompact",
                serde_json::json!({ "message_count": before, "estimated_tokens": estimated_tokens }),
                &tool_context.cwd,
            ).await;
            *messages = compact::compact_messages_ai(
                client,
                model_router::MODEL_LIGHT,
                messages,
                AUTO_COMPACT_KEEP_RECENT,
            )
            .await;
            let after_tokens = compact::estimate_tokens(messages);
            crate::hooks::run_event_hooks(
                &config.hooks,
                "PostCompact",
                serde_json::json!({ "message_count_before": before, "message_count_after": messages.len(), "estimated_tokens": after_tokens }),
                &tool_context.cwd,
            ).await;
            if config.verbose {
                out.status_dim(&format!(
                    "[auto-compact: {} → {} messages, ~{} tokens]",
                    before,
                    messages.len(),
                    after_tokens
                ));
            }
        }

        // Adaptive model selection — escalate to heavy on validation failures or empty responses
        let model = match validation_failure_fallback_model.take().or_else(|| empty_response_fallback_model.take()) {
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
            out.status_dim(&format!("[model: {model}]"));
        }

        // Send model/turn info to UI and signal inference start
        if let Some(b) = bridge {
            b.model_info(&model);
            b.turn_info(turns, config.max_turns);
            b.inference_start();
        }

        // Create markdown renderer for this turn (shared via Mutex for closure capture)
        let md_renderer = std::sync::Mutex::new(crate::ui::markdown::MarkdownRenderer::new());

        // Retry loop for transient API errors
        let mut api_retries = 0u32;
        let (content_blocks, stop_reason, usage) = loop {
            let api_messages = normalize_for_api(messages);

            // Create streaming callbacks — bridge-based or print-based
            let bridge_for_text = bridge.cloned();
            let bridge_for_tool = bridge.cloned();

            // ThinkingIndicator only used in non-bridge (oneshot) mode
            let thinking = if bridge.is_none() {
                std::sync::Mutex::new(Some(ThinkingIndicator::start()))
            } else {
                std::sync::Mutex::new(None)
            };

            let md = &md_renderer;
            let mut on_text = move |text: &str| {
                if let Some(ref b) = bridge_for_text {
                    // Bridge mode: pass raw text — iocraft handles its own rendering
                    b.stream_text(text);
                } else {
                    match output_format {
                        OutputFormat::StreamJson => {
                            StreamEvent::text(text).emit();
                        }
                        OutputFormat::Json => {
                            // Suppress streaming output; will emit final JSON at end
                        }
                        OutputFormat::Text => {
                            if let Ok(mut guard) = thinking.lock() {
                                if let Some(indicator) = guard.take() {
                                    indicator.stop();
                                }
                            }
                            // Render markdown for terminal output
                            if let Ok(mut renderer) = md.lock() {
                                let formatted = renderer.push_text(text);
                                print!("{formatted}");
                            } else {
                                print!("{text}");
                            }
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                        }
                    }
                }
            };

            let mut on_tool_start = move |id: &str, name: &str| {
                if let Some(ref b) = bridge_for_tool {
                    b.stream_tool_start(id, name);
                } else if output_format == OutputFormat::Text {
                    println!("\n{} {}", "Tool:".cyan().bold(), name.cyan());
                }
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

            match result {
                Ok(r) => break r,
                Err(crate::api::errors::ApiError::ContextTooLong(_)) => {
                    out.status_warn("[context too long, compacting...]");
                    *messages = compact::compact_messages_ai(
                        client,
                        model_router::MODEL_LIGHT,
                        messages,
                        6,
                    )
                    .await;
                    continue;
                }
                Err(e) if e.is_retryable() && api_retries < MAX_API_RETRIES => {
                    api_retries += 1;
                    let delay = match &e {
                        crate::api::errors::ApiError::RateLimit {
                            retry_after_secs: Some(s),
                        } => *s,
                        _ => 2u64.pow(api_retries),
                    };
                    out.status_warn(&format!(
                        "[transient API error: {e} — retrying in {delay}s ({api_retries}/{MAX_API_RETRIES})]"
                    ));
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    continue;
                }
                Err(e) => {
                    out.status_error(&format!(
                        "[query_loop exiting: API error after {api_retries} retries: {e}]"
                    ));
                    return Err(anyhow::anyhow!("API error: {e}"));
                }
            }
        };

        // Flush markdown renderer after streaming completes (only for direct terminal output)
        if bridge.is_none() && output_format == OutputFormat::Text {
            if let Ok(mut renderer) = md_renderer.lock() {
                let remaining = renderer.flush();
                if !remaining.is_empty() {
                    print!("{remaining}");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                }
            }
        }

        cost_tracker.add_usage(&usage);

        // Check if streaming was interrupted by ESC
        if escape_flag.load(Ordering::Relaxed) {
            escape_flag.store(false, Ordering::Relaxed);
            out.newline();
            out.status_warn(&format!(
                "[interrupted by user during streaming at turn {turns}]"
            ));
            break;
        }

        // Store assistant message
        let assistant_msg = AssistantMessage {
            content: content_blocks.clone(),
            stop_reason: stop_reason.clone(),
            usage,
        };
        messages.push(Message::Assistant(assistant_msg));

        let has_text = content_blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::Text { text } if !text.trim().is_empty()));
        let has_tool_calls = content_blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }));

        // Check if we're done
        if stop_reason == StopReason::EndTurn {
            if !has_text && !has_tool_calls && turns > 0 {
                empty_end_turn_count += 1;

                if empty_end_turn_count == 1 {
                    out.status_warn(&format!(
                        "[warning: empty end_turn (attempt {}/{}), injecting nudge]",
                        empty_end_turn_count, MAX_EMPTY_END_TURN
                    ));
                    messages.push(Message::user_text(
                        "You returned an empty response. Please continue your analysis based on the tool results above. \
                         Summarize what you found and proceed with the next step.",
                    ));
                    turns += 1;
                    continue;
                } else if empty_end_turn_count == 2 {
                    let current_model = model_router::pick_model(
                        &config.model,
                        messages,
                        &last_tool_names,
                        turns,
                        config.auto_model_routing,
                        config.intensity,
                    );
                    let fallback = if current_model == model_router::MODEL_HEAVY {
                        None
                    } else {
                        Some(model_router::MODEL_HEAVY.to_string())
                    };
                    if let Some(ref fb) = fallback {
                        out.status_warn(&format!(
                            "[warning: empty end_turn (attempt {}/{}), switching to '{}']",
                            empty_end_turn_count, MAX_EMPTY_END_TURN, fb
                        ));
                    } else {
                        out.status_warn(&format!(
                            "[warning: empty end_turn (attempt {}/{}), retrying with nudge]",
                            empty_end_turn_count, MAX_EMPTY_END_TURN
                        ));
                    }
                    empty_response_fallback_model = fallback;
                    messages.push(Message::user_text(
                        "You returned an empty response again. Please respond to the user's request. \
                         Analyze the conversation and tool results and provide your answer.",
                    ));
                    turns += 1;
                    continue;
                } else {
                    out.status_dim(&format!(
                        "[loop exit: empty end_turn {} times at turn {turns}, recovery exhausted]",
                        empty_end_turn_count
                    ));
                }
            } else {
                empty_end_turn_count = 0;
                empty_response_fallback_model = None;
                out.newline();
                out.status_dim(&format!(
                    "[loop exit: model returned end_turn at turn {turns}]"
                ));
                // Fire Stop hook
                crate::hooks::run_event_hooks(
                    &config.hooks,
                    "Stop",
                    serde_json::json!({ "reason": "end_turn", "turn": turns }),
                    &tool_context.cwd,
                ).await;
            }
            break;
        }
        empty_end_turn_count = 0;
        empty_response_fallback_model = None;

        if stop_reason == StopReason::MaxTokens {
            max_tokens_recoveries += 1;
            if max_tokens_recoveries <= MAX_OUTPUT_RECOVERY_ATTEMPTS {
                if config.verbose {
                    out.status_dim(&format!(
                        "[max_tokens hit, recovery attempt {}/{}]",
                        max_tokens_recoveries, MAX_OUTPUT_RECOVERY_ATTEMPTS
                    ));
                }
                messages.push(Message::user_text(
                    "Continue from where you left off. Do not repeat what you already said.",
                ));
                turns += 1;
                continue;
            } else {
                out.status_error(&format!(
                    "[loop exit: max_tokens hit {MAX_OUTPUT_RECOVERY_ATTEMPTS} times, unrecoverable at turn {turns}]"
                ));
                out.status_warn("(max tokens reached, could not recover)");
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

            // Stuck-loop detection
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
                let tool_names: Vec<_> =
                    tool_uses.iter().map(|(_, n, _)| n.as_str()).collect();

                if repeated_tool_count >= MAX_REPEATED_TOOL_CALLS + 2 {
                    out.status_error(&format!(
                        "[stuck-loop: model ignored warnings, force-stopping after {} repeats]",
                        repeated_tool_count
                    ));
                    let abort_msg = format!(
                        "The AI got stuck calling {} repeatedly with the same arguments and could not recover. \
                         Please try rephrasing your request.",
                        tool_names.join(", ")
                    );
                    let results: Vec<_> = tool_uses
                        .iter()
                        .map(|(id, _, _)| (id.clone(), abort_msg.clone(), true))
                        .collect();
                    messages.push(Message::tool_results(results));
                    out.status_warn(&abort_msg);
                    break;
                }

                out.status_warn(&format!(
                    "[stuck-loop detected: same tool call repeated {} times, injecting warning]",
                    repeated_tool_count
                ));
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
                let results: Vec<_> = tool_uses
                    .iter()
                    .map(|(id, _, _)| (id.clone(), warning.clone(), true))
                    .collect();
                messages.push(Message::tool_results(results));
                turns += 1;
                continue;
            }

            let mut results = Vec::new();
            let mut had_validation_failure = false;

            for (id, name, input) in &tool_uses {
                // Check escape between tool calls
                if escape_flag.load(Ordering::Relaxed) {
                    escape_flag.store(false, Ordering::Relaxed);
                    out.newline();
                    out.status_warn(&format!(
                        "[interrupted by user during tool execution at turn {turns}]"
                    ));
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
                    out.status_warn(&format!(
                        "[tool '{name}' input validation failed: {validation_error}]"
                    ));
                    results.push((id.clone(), validation_error, true));
                    had_validation_failure = true;
                    continue;
                }

                let is_read_only = tool.is_read_only(input);
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
                        results
                            .push((id.clone(), format!("Permission denied: {reason}"), true));
                        continue;
                    }
                    permissions::PermissionResult::Ask => {
                        // Route permission prompt through bridge if available
                        if let Some(b) = bridge {
                            let detail =
                                permissions::build_permission_detail(name, input);
                            // block_in_place so tokio background tasks keep running
                            let allowed_by_user = tokio::task::block_in_place(|| {
                                b.prompt_permission(detail)
                            });
                            if allowed_by_user {
                                true
                            } else {
                                results.push((
                                    id.clone(),
                                    "User denied permission".to_string(),
                                    true,
                                ));
                                continue;
                            }
                        } else {
                            // Direct stdin permission prompt (oneshot mode)
                            let description = input["description"].as_str();
                            match permissions::prompt_user_permission(
                                name,
                                input,
                                description,
                            ) {
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
                    }
                };

                if allowed {
                    if config.verbose {
                        let input_str =
                            serde_json::to_string_pretty(input).unwrap_or_default();
                        out.status_dim(&input_str);
                    }

                    // Run PreToolUse hooks
                    if let Some(block_reason) = crate::hooks::run_pre_tool_hooks(
                        &config.hooks, name, input, &tool_context.cwd,
                    ).await {
                        out.status_warn(&format!("[hook blocked {name}: {block_reason}]"));
                        results.push((id.clone(), format!("Blocked by hook: {block_reason}"), true));
                        continue;
                    }

                    // Notify UI of tool execution start
                    if let Some(b) = bridge {
                        b.tool_exec_start(name);
                    }

                    let tool_start = std::time::Instant::now();

                    match tool.call(input.clone(), tool_context).await {
                        Ok(mut result) => {
                            let duration_ms = tool_start.elapsed().as_millis() as u64;

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

                            // Show first line only as preview (multi-line results look broken in terminal)
                            let preview: String = result
                                .content
                                .lines()
                                .next()
                                .unwrap_or("")
                                .chars()
                                .take(500)
                                .collect();

                            if let Some(b) = bridge {
                                b.tool_result(
                                    name,
                                    &preview,
                                    result.is_error,
                                    duration_ms,
                                );
                            } else if output_format == OutputFormat::StreamJson {
                                StreamEvent::tool_use(name, input).emit();
                                StreamEvent::tool_result(name, &preview, result.is_error).emit();
                            } else if output_format == OutputFormat::Text {
                                out.tool_result(&preview, result.is_error);
                                if result.content.len() > 500 {
                                    out.status_dim(&format!(
                                        "  ... ({} total chars)",
                                        result.content.len()
                                    ));
                                }
                            }

                            // Run PostToolUse hooks
                            let hook_context = crate::hooks::run_post_tool_hooks(
                                &config.hooks, name, input, &result.content, &tool_context.cwd,
                            ).await;
                            if !hook_context.is_empty() {
                                result.content.push_str(&format!(
                                    "\n\n--- Hook Output ---\n{hook_context}"
                                ));
                            }

                            results
                                .push((id.clone(), result.content, result.is_error));
                        }
                        Err(e) => {
                            let duration_ms = tool_start.elapsed().as_millis() as u64;
                            let error_msg = format!("Tool execution error: {e}");

                            if let Some(b) = bridge {
                                b.tool_result(name, &error_msg, true, duration_ms);
                            } else {
                                out.tool_result(&error_msg, true);
                            }

                            results.push((id.clone(), error_msg, true));
                        }
                    }
                }
            }

            // Add tool results as user message
            if !results.is_empty() {
                messages.push(Message::tool_results(results));
            }

            // If tool validation failed and we used the light model, escalate to heavy
            if had_validation_failure && model == model_router::MODEL_LIGHT {
                out.status_dim("[validation failure on light model, escalating to heavy]");
                validation_failure_fallback_model =
                    Some(model_router::MODEL_HEAVY.to_string());
            }
        } else {
            last_tool_names.clear();
        }

        turns += 1;
        if turns >= config.max_turns {
            out.status_error(&format!(
                "[loop exit: max turns limit ({}) reached]",
                config.max_turns
            ));
            out.status_warn(&format!("Max turns ({}) reached.", config.max_turns));
            break;
        }

        // Check budget
        if let Some(budget) = config.budget_usd {
            let current_cost = cost_tracker.estimate_cost_usd(&config.model);
            if current_cost >= budget {
                out.status_error(&format!(
                    "[loop exit: budget ${budget:.2} exhausted (spent ${current_cost:.2})]"
                ));
                out.status_warn(&format!("Budget limit (${budget:.2}) reached."));
                break;
            }
        }
    }

    Ok(())
}

/// Validate tool input against the tool's JSON schema.
/// Returns Some(error_message) if validation fails, None if OK.
fn validate_tool_input(
    tool: &dyn crate::tools::Tool,
    input: &serde_json::Value,
) -> Option<String> {
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
                                let typ = p
                                    .get("type")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("any");
                                let desc = p
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("");
                                Some(format!("  - {name} ({typ}): {desc}"))
                            })
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();

            let is_completely_empty =
                input.as_object().map_or(true, |o| o.is_empty());
            let extra_hint = if is_completely_empty {
                "\n\nYour tool call had NO arguments at all. This usually means something went wrong. \
                 Do NOT retry with empty arguments — either provide the correct parameters or try a different approach."
            } else {
                ""
            };
            return Some(format!(
                "Missing required parameter(s): {}. You MUST provide these parameters.\n{}{}",
                missing.join(", "),
                schema_hint,
                extra_hint
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
                if let Some(expected_type) =
                    prop_schema.get("type").and_then(|t| t.as_str())
                {
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
