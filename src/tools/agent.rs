use crate::api::types::ContentBlock;
use crate::engine;
use crate::engine::cost::CostTracker;
use crate::messages::types::*;
use crate::tools::{Tool, ToolContext, ToolRegistry, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AgentTool;

const MAX_SUBAGENT_TURNS: usize = 15;

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "Agent"
    }

    fn description(&self) -> String {
        "Launch a sub-agent to handle complex, multi-step tasks autonomously. \
         The agent runs with its own message history and returns a summary result. \
         Use 'explore' type for read-only research, 'general' for full capabilities."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The task for the sub-agent to perform"
                },
                "subagent_type": {
                    "type": "string",
                    "description": "Agent type: 'explore' (read-only, default), 'plan' (read-only), or 'general' (all tools except Agent)"
                }
            },
            "required": ["prompt"]
        })
    }

    fn is_read_only(&self, input: &serde_json::Value) -> bool {
        // Explore and plan agents are read-only; general is not
        let agent_type = input["subagent_type"].as_str().unwrap_or("explore");
        matches!(agent_type, "explore" | "plan")
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let prompt = input["prompt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'prompt' parameter"))?;
        let agent_type = input["subagent_type"].as_str().unwrap_or("explore");

        if prompt.trim().is_empty() {
            return Ok(ToolResult::error("Prompt cannot be empty"));
        }

        let registry = build_subagent_registry(agent_type);
        let mut messages = vec![Message::user_text(prompt)];
        let mut cost_tracker = CostTracker::with_pricing(context.config.pricing_table.clone());

        // Create a sub-config with limited turns
        let mut sub_config = context.config.clone();
        sub_config.max_turns = MAX_SUBAGENT_TURNS;

        // Create an isolated child context: own permission_mode and plan_file_path
        // to prevent sub-agent from mutating parent state.
        // Task store and LSP manager are shared (intentional).
        let current_mode = *context.permission_mode.lock().await;
        let child_context = ToolContext {
            cwd: context.cwd.clone(),
            permission_mode: Arc::new(Mutex::new(current_mode)),
            task_store: context.task_store.clone(),
            api_client: context.api_client.clone(),
            config: sub_config.clone(),
            lsp_manager: context.lsp_manager.clone(),
            plan_file_path: Arc::new(Mutex::new(None)),
        };

        println!("\n  [Agent ({agent_type}): starting sub-agent]");

        // Sub-agents get their own escape flag (not interruptible by parent ESC)
        let sub_escape = Arc::new(AtomicBool::new(false));
        let result = engine::query_loop(
            &context.api_client,
            &mut messages,
            &registry,
            &sub_config,
            &mut cost_tracker,
            &child_context,
            &sub_escape,
        )
        .await;

        if let Err(ref e) = result {
            return Ok(ToolResult::error(format!("Sub-agent error: {e}")));
        }

        // Extract the final assistant text response
        let final_text = messages
            .iter()
            .rev()
            .find_map(|msg| {
                if let Message::Assistant(assistant) = msg {
                    let texts: Vec<String> = assistant
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect();
                    if texts.is_empty() {
                        None
                    } else {
                        Some(texts.join("\n"))
                    }
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "(no response from sub-agent)".to_string());

        let cost = cost_tracker.estimate_cost_usd(&context.config.model);
        let summary = format!(
            "{final_text}\n\n[Sub-agent used {} input + {} output tokens, ${cost:.4}]",
            cost_tracker.total_input_tokens, cost_tracker.total_output_tokens,
        );

        Ok(ToolResult::success(summary))
    }
}

/// Build a tool registry for a sub-agent. Never includes Agent to prevent recursion.
fn build_subagent_registry(agent_type: &str) -> ToolRegistry {
    use crate::tools::*;

    let mut registry = ToolRegistry::new();

    // Read-only tools (always available)
    registry.register(Box::new(read::ReadTool));
    registry.register(Box::new(glob::GlobTool));
    registry.register(Box::new(grep::GrepTool));
    registry.register(Box::new(web_fetch::WebFetchTool));
    registry.register(Box::new(ask_user::AskUserTool));
    registry.register(Box::new(task_list::TaskListTool));
    registry.register(Box::new(task_get::TaskGetTool));
    registry.register(Box::new(web_search::WebSearchTool));

    match agent_type {
        "general" => {
            // General gets write tools too (but not Agent)
            registry.register(Box::new(bash::BashTool));
            registry.register(Box::new(write::WriteTool));
            registry.register(Box::new(edit::EditTool));
            registry.register(Box::new(task_create::TaskCreateTool));
            registry.register(Box::new(task_update::TaskUpdateTool));
            registry.register(Box::new(notebook_edit::NotebookEditTool));
        }
        // "explore" | "plan" | _ — read-only tools only
        _ => {}
    }

    registry
}
