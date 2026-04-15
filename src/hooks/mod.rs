//! Async trait-based hook / middleware system.
//!
//! Replaces the previous command-shelling hook executor with a Rust trait
//! so hooks can be Rust-native (fast path) while still supporting the legacy
//! JSON-stdin contract via [`ShellHook`]. The engine dispatches events
//! through a [`HookChain`] which fans each event out to every registered
//! hook in order; a hook can return `Block(reason)` to abort a tool call or
//! `AppendContext(text)` to inject additional context into the model's view
//! of the result.
//!
//! `HooksConfig` mirrors the `~/.arcee/hooks.json` schema unchanged so
//! existing user configs keep working — a [`HookChain::from_config`]
//! constructor builds a chain of `ShellHook` instances from it.

use crate::tools::ToolOutput;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

// ─── Hook events ─────────────────────────────────────────────────────────────

/// Events the engine dispatches through the [`HookChain`].
#[derive(Debug, Clone)]
pub enum HookEvent {
    SessionStart,
    SessionEnd,
    UserPromptSubmit { text: String },
    PreToolUse { tool: String, input: serde_json::Value },
    PostToolUse { tool: String, output: ToolOutput },
    PreCompact { tokens: u64, message_count: usize },
    PostCompact { tokens_before: u64, tokens_after: u64, message_count_before: usize, message_count_after: usize },
    SubagentStart { description: String },
    SubagentStop { summary: String },
    Stop { reason: String, turn: u32 },
}

impl HookEvent {
    /// Canonical event name for the JSON-compat shell hook contract.
    pub fn name(&self) -> &'static str {
        match self {
            HookEvent::SessionStart => "SessionStart",
            HookEvent::SessionEnd => "SessionEnd",
            HookEvent::UserPromptSubmit { .. } => "UserPromptSubmit",
            HookEvent::PreToolUse { .. } => "PreToolUse",
            HookEvent::PostToolUse { .. } => "PostToolUse",
            HookEvent::PreCompact { .. } => "PreCompact",
            HookEvent::PostCompact { .. } => "PostCompact",
            HookEvent::SubagentStart { .. } => "SubagentStart",
            HookEvent::SubagentStop { .. } => "SubagentStop",
            HookEvent::Stop { .. } => "Stop",
        }
    }

    /// Tool name for tool-scoped events, else empty string.
    pub fn tool_name(&self) -> &str {
        match self {
            HookEvent::PreToolUse { tool, .. } => tool,
            HookEvent::PostToolUse { tool, .. } => tool,
            _ => "",
        }
    }
}

/// Decision returned by a hook.
#[derive(Debug, Clone)]
pub enum HookDecision {
    /// No-op: let execution continue.
    Continue,
    /// Abort the in-flight tool call. The reason is returned to the model.
    Block(String),
    /// Append this text to the tool result or the user message.
    AppendContext(String),
}

/// Shared per-session state passed to hooks.
#[derive(Debug, Clone)]
pub struct HookContext {
    pub cwd: PathBuf,
}

impl HookContext {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

// ─── Hook trait ──────────────────────────────────────────────────────────────

/// Async trait every hook implements.
#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;

    async fn on_event(&self, event: &mut HookEvent, ctx: &HookContext) -> HookDecision;
}

/// A fan-out dispatcher for hooks.
#[derive(Default, Clone)]
pub struct HookChain {
    hooks: Arc<Vec<Arc<dyn Hook>>>,
}

impl HookChain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, hook: Arc<dyn Hook>) {
        // Clone-out-of-Arc to mutate
        let mut v: Vec<Arc<dyn Hook>> = (*self.hooks).clone();
        v.push(hook);
        self.hooks = Arc::new(v);
    }

    /// Build a chain from a user config by wrapping each config entry in a
    /// [`ShellHook`]. Preserves backwards compatibility with existing
    /// `~/.arcee/hooks.json` files.
    pub fn from_config(config: &HooksConfig) -> Self {
        let mut chain = Self::new();
        let mut add_groups = |groups: &[HookGroup], event: &str| {
            for group in groups {
                for action in &group.hooks {
                    if action.action_type == "command" {
                        chain.push(Arc::new(ShellHook {
                            event_name: event.to_string(),
                            matcher: group.matcher.clone(),
                            command: action.command.clone(),
                            timeout_secs: action.timeout.unwrap_or(120),
                        }));
                    }
                }
            }
        };

        add_groups(&config.pre_tool_use, "PreToolUse");
        add_groups(&config.post_tool_use, "PostToolUse");
        add_groups(&config.notification, "Notification");
        add_groups(&config.user_prompt_submit, "UserPromptSubmit");
        add_groups(&config.session_start, "SessionStart");
        add_groups(&config.session_end, "SessionEnd");
        add_groups(&config.stop, "Stop");
        add_groups(&config.subagent_start, "SubagentStart");
        add_groups(&config.subagent_stop, "SubagentStop");
        add_groups(&config.pre_compact, "PreCompact");
        add_groups(&config.post_compact, "PostCompact");

        chain
    }

    /// Dispatch an event through every hook. Returns the strongest decision:
    /// any `Block` short-circuits; otherwise any `AppendContext` texts are
    /// concatenated; otherwise `Continue`.
    pub async fn dispatch(&self, event: &mut HookEvent, ctx: &HookContext) -> HookDecision {
        let mut appended: Vec<String> = Vec::new();
        for hook in self.hooks.iter() {
            match hook.on_event(event, ctx).await {
                HookDecision::Continue => {}
                HookDecision::Block(reason) => return HookDecision::Block(reason),
                HookDecision::AppendContext(text) => {
                    if !text.is_empty() {
                        appended.push(text);
                    }
                }
            }
        }
        if appended.is_empty() {
            HookDecision::Continue
        } else {
            HookDecision::AppendContext(appended.join("\n"))
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

// ─── Shell hook (legacy command-based compat shim) ──────────────────────────

/// Runs an external shell command, passing the event payload as JSON on
/// stdin and parsing a `HookOutput` JSON envelope (or raw text) on stdout.
///
/// Exit code 2 signals a blocking error, matching the pre-rewrite contract.
pub struct ShellHook {
    pub event_name: String,
    pub matcher: Option<String>,
    pub command: String,
    pub timeout_secs: u64,
}

#[async_trait]
impl Hook for ShellHook {
    fn name(&self) -> &str {
        &self.event_name
    }

    async fn on_event(&self, event: &mut HookEvent, ctx: &HookContext) -> HookDecision {
        if self.event_name != event.name() {
            return HookDecision::Continue;
        }
        // Tool matcher check. Clone tool name into an owned string so the
        // borrow of `event` ends before we destructure its fields below.
        let tool = event.tool_name().to_string();
        if !matcher_allows(&self.matcher, &tool) {
            return HookDecision::Continue;
        }

        let (tool_input, tool_response) = match event {
            HookEvent::PreToolUse { input, .. } => (input.clone(), None),
            HookEvent::PostToolUse { output, .. } => {
                (serde_json::Value::Null, Some(output.render()))
            }
            HookEvent::UserPromptSubmit { text } => (
                serde_json::json!({ "text": text }),
                None,
            ),
            HookEvent::PreCompact { tokens, message_count } => (
                serde_json::json!({ "tokens": tokens, "message_count": message_count }),
                None,
            ),
            HookEvent::PostCompact {
                tokens_before,
                tokens_after,
                message_count_before,
                message_count_after,
            } => (
                serde_json::json!({
                    "tokens_before": tokens_before,
                    "tokens_after": tokens_after,
                    "message_count_before": message_count_before,
                    "message_count_after": message_count_after,
                }),
                None,
            ),
            HookEvent::SubagentStart { description } => {
                (serde_json::json!({ "description": description }), None)
            }
            HookEvent::SubagentStop { summary } => {
                (serde_json::json!({ "summary": summary }), None)
            }
            HookEvent::Stop { reason, turn } => {
                (serde_json::json!({ "reason": reason, "turn": turn }), None)
            }
            HookEvent::SessionStart | HookEvent::SessionEnd => {
                (serde_json::Value::Null, None)
            }
        };

        let input = HookInput {
            hook_event_name: event.name().to_string(),
            tool_name: tool.clone(),
            tool_input,
            tool_response,
            cwd: ctx.cwd.display().to_string(),
        };
        let input_json = match serde_json::to_string(&input) {
            Ok(s) => s,
            Err(_) => return HookDecision::Continue,
        };

        let result = run_command(&self.command, &input_json, self.timeout_secs, &ctx.cwd).await;

        if result.timed_out {
            return HookDecision::Block(format!("Hook timed out: {}", self.command));
        }

        // Exit code 2 = blocking
        if result.exit_code == 2 {
            let reason = if !result.stderr.is_empty() {
                result.stderr.trim().to_string()
            } else if !result.stdout.is_empty() {
                result.stdout.trim().to_string()
            } else {
                "Hook returned blocking exit code 2".to_string()
            };
            return HookDecision::Block(reason);
        }

        // Parse stdout for structured response.
        let stdout = result.stdout.trim();
        if !stdout.is_empty() {
            if let Ok(output) = serde_json::from_str::<HookOutput>(stdout) {
                if !output.should_continue {
                    let reason = output
                        .additional_context
                        .unwrap_or_else(|| "Hook blocked execution".to_string());
                    return HookDecision::Block(reason);
                }
                if let Some(ctx_text) = output.additional_context {
                    if !ctx_text.is_empty() {
                        return HookDecision::AppendContext(ctx_text);
                    }
                }
            } else {
                return HookDecision::AppendContext(stdout.to_string());
            }
        }

        HookDecision::Continue
    }
}

fn matcher_allows(matcher: &Option<String>, tool_name: &str) -> bool {
    match matcher {
        None => true,
        Some(m) if m.is_empty() => true,
        Some(m) => m.split('|').any(|part| part.trim() == tool_name),
    }
}

struct CmdResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

async fn run_command(command: &str, input_json: &str, timeout_secs: u64, cwd: &Path) -> CmdResult {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    let child = Command::new(&shell)
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return CmdResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Failed to spawn hook command: {e}"),
                timed_out: false,
            };
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input_json.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    let timeout = tokio::time::Duration::from_secs(timeout_secs);
    let wait_fut = async {
        let output = child.wait_with_output().await?;
        Ok::<_, std::io::Error>(output)
    };

    match tokio::time::timeout(timeout, wait_fut).await {
        Ok(Ok(output)) => CmdResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            timed_out: false,
        },
        Ok(Err(e)) => CmdResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Hook command error: {e}"),
            timed_out: false,
        },
        Err(_) => CmdResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Hook command timed out after {timeout_secs}s"),
            timed_out: true,
        },
    }
}

// ─── Legacy JSON config types (unchanged for backwards compat) ──────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksConfig {
    #[serde(rename = "PreToolUse", default)]
    pub pre_tool_use: Vec<HookGroup>,
    #[serde(rename = "PostToolUse", default)]
    pub post_tool_use: Vec<HookGroup>,
    #[serde(rename = "Notification", default)]
    pub notification: Vec<HookGroup>,
    #[serde(rename = "UserPromptSubmit", default)]
    pub user_prompt_submit: Vec<HookGroup>,
    #[serde(rename = "SessionStart", default)]
    pub session_start: Vec<HookGroup>,
    #[serde(rename = "SessionEnd", default)]
    pub session_end: Vec<HookGroup>,
    #[serde(rename = "Stop", default)]
    pub stop: Vec<HookGroup>,
    #[serde(rename = "SubagentStart", default)]
    pub subagent_start: Vec<HookGroup>,
    #[serde(rename = "SubagentStop", default)]
    pub subagent_stop: Vec<HookGroup>,
    #[serde(rename = "PreCompact", default)]
    pub pre_compact: Vec<HookGroup>,
    #[serde(rename = "PostCompact", default)]
    pub post_compact: Vec<HookGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookGroup {
    pub matcher: Option<String>,
    pub hooks: Vec<HookAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub command: String,
    pub timeout: Option<u64>,
}

/// JSON envelope sent to a shell hook on stdin.
#[derive(Debug, Serialize)]
pub struct HookInput {
    pub hook_event_name: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_response: Option<String>,
    pub cwd: String,
}

/// JSON envelope expected from a shell hook on stdout.
#[derive(Debug, Deserialize)]
pub struct HookOutput {
    #[serde(rename = "continue", default = "default_true")]
    pub should_continue: bool,
    #[serde(rename = "additionalContext")]
    pub additional_context: Option<String>,
    pub decision: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for HookOutput {
    fn default() -> Self {
        Self {
            should_continue: true,
            additional_context: None,
            decision: None,
        }
    }
}

/// Merge project-level hooks into a base config (additive).
pub fn merge(base: &mut HooksConfig, overlay: HooksConfig) {
    base.pre_tool_use.extend(overlay.pre_tool_use);
    base.post_tool_use.extend(overlay.post_tool_use);
    base.notification.extend(overlay.notification);
    base.user_prompt_submit.extend(overlay.user_prompt_submit);
    base.session_start.extend(overlay.session_start);
    base.session_end.extend(overlay.session_end);
    base.stop.extend(overlay.stop);
    base.subagent_start.extend(overlay.subagent_start);
    base.subagent_stop.extend(overlay.subagent_stop);
    base.pre_compact.extend(overlay.pre_compact);
    base.post_compact.extend(overlay.post_compact);
}

// ─── Thin backwards-compat wrappers used by legacy call sites ───────────────

/// Run PreToolUse hooks. Returns Some(block_reason) if a hook blocks.
pub async fn run_pre_tool_hooks(
    config: &HooksConfig,
    tool_name: &str,
    tool_input: &serde_json::Value,
    cwd: &Path,
) -> Option<String> {
    let chain = HookChain::from_config(config);
    if chain.is_empty() {
        return None;
    }
    let mut event = HookEvent::PreToolUse {
        tool: tool_name.to_string(),
        input: tool_input.clone(),
    };
    let ctx = HookContext::new(cwd.to_path_buf());
    match chain.dispatch(&mut event, &ctx).await {
        HookDecision::Block(reason) => Some(reason),
        _ => None,
    }
}

/// Run PostToolUse hooks. Returns any additional context to append.
pub async fn run_post_tool_hooks(
    config: &HooksConfig,
    tool_name: &str,
    _tool_input: &serde_json::Value,
    tool_response: &str,
    cwd: &Path,
) -> String {
    let chain = HookChain::from_config(config);
    if chain.is_empty() {
        return String::new();
    }
    // Build a pseudo-output from raw text for compat.
    let output = ToolOutput::success().with_text(tool_response);
    let mut event = HookEvent::PostToolUse {
        tool: tool_name.to_string(),
        output,
    };
    let ctx = HookContext::new(cwd.to_path_buf());
    match chain.dispatch(&mut event, &ctx).await {
        HookDecision::AppendContext(text) => text,
        _ => String::new(),
    }
}

/// Run hooks for a generic lifecycle event (SessionStart, Stop, etc.).
pub async fn run_event_hooks(
    config: &HooksConfig,
    event_name: &str,
    data: serde_json::Value,
    cwd: &Path,
) -> String {
    let chain = HookChain::from_config(config);
    if chain.is_empty() {
        return String::new();
    }

    // Map the event name → canonical HookEvent so ShellHook can serialize it.
    let mut event = match event_name {
        "SessionStart" => HookEvent::SessionStart,
        "SessionEnd" => HookEvent::SessionEnd,
        "UserPromptSubmit" => HookEvent::UserPromptSubmit {
            text: data
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        "PreCompact" => HookEvent::PreCompact {
            tokens: data.get("estimated_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            message_count: data
                .get("message_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
        },
        "PostCompact" => HookEvent::PostCompact {
            tokens_before: 0,
            tokens_after: data.get("estimated_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            message_count_before: data
                .get("message_count_before")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            message_count_after: data
                .get("message_count_after")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
        },
        "Stop" => HookEvent::Stop {
            reason: data
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            turn: data.get("turn").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        },
        "SubagentStart" => HookEvent::SubagentStart {
            description: data
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        "SubagentStop" => HookEvent::SubagentStop {
            summary: data
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        _ => return String::new(),
    };

    let ctx = HookContext::new(cwd.to_path_buf());
    match chain.dispatch(&mut event, &ctx).await {
        HookDecision::AppendContext(text) => text,
        _ => String::new(),
    }
}

// Bring anyhow into scope for any future Result-returning hook impls.
#[allow(dead_code)]
fn _type_hint() -> Result<()> {
    Ok(())
}
