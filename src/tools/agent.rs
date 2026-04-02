use crate::api::types::ContentBlock;
use crate::engine;
use crate::engine::context;
use crate::engine::cost::CostTracker;
use crate::messages::types::*;
use crate::tools::{Tool, ToolContext, ToolRegistry, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use colored::Colorize;
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AgentTool;

const MAX_SUBAGENT_TURNS: usize = 25;

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "Agent"
    }

    fn description(&self) -> String {
        "Launch a sub-agent to handle complex, multi-step research or coding tasks autonomously. \
         The agent runs with its own message history and returns a comprehensive result.\n\n\
         Agent types:\n\
         - 'explore' (default): Fast, read-only research agent for codebase exploration. \
         Use when you need to find files, search for patterns, understand architecture, \
         or answer questions about the codebase. Specify thoroughness: 'quick' for basic searches, \
         'medium' for moderate exploration, 'very thorough' for comprehensive analysis.\n\
         - 'plan': Read-only architect agent for designing implementation approaches. \
         Explores thoroughly before proposing plans.\n\
         - 'general': Full-capability agent for tasks requiring code changes.\n\n\
         When to use Agent vs direct tools:\n\
         - Use Glob/Grep/Read directly for simple, known-target searches (1-2 queries)\n\
         - Use Agent(explore) for broader exploration needing 3+ searches, multiple strategies, \
         or when you don't know exactly where to look\n\
         - Launch multiple agents in parallel for independent research tasks\n\n\
         Background execution:\n\
         - Set run_in_background=true to run the agent in the background\n\
         - You will be automatically notified when it completes — do NOT poll or check on it\n\
         - Use foreground (default) when you need results before proceeding\n\
         - Use background when you have independent work to do in parallel\n\
         - Use TaskOutput tool to retrieve output from a completed background task"
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Detailed task description for the sub-agent. Be specific about what to find or do. Include context about why you need this information."
                },
                "subagent_type": {
                    "type": "string",
                    "description": "Agent type: 'explore' (read-only research, default), 'plan' (read-only architecture), or 'general' (all tools except Agent)"
                },
                "description": {
                    "type": "string",
                    "description": "A short (3-5 word) description of what the agent will do, shown in status display"
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Set to true to run this agent in the background. You will be notified when it completes. Continue with other work instead of waiting."
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
        let run_in_background = input["run_in_background"].as_bool().unwrap_or(false);
        let description = input["description"]
            .as_str()
            .unwrap_or("sub-agent task")
            .to_string();

        if prompt.trim().is_empty() {
            return Ok(ToolResult::error("Prompt cannot be empty"));
        }

        if run_in_background {
            return self
                .launch_background(prompt, agent_type, &description, context)
                .await;
        }

        // Foreground execution (original behavior)
        self.run_foreground(prompt, agent_type, context).await
    }
}

impl AgentTool {
    /// Run a sub-agent in the foreground, blocking until complete.
    async fn run_foreground(
        &self,
        prompt: &str,
        agent_type: &str,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        let (registry, sub_config, child_context, system_prompt) =
            build_agent_setup(agent_type, context).await;

        let mut messages = vec![Message::user_text(prompt)];
        let mut cost_tracker = CostTracker::with_pricing(context.config.pricing_table.clone());

        println!("\n  [Agent ({agent_type}): starting sub-agent]");

        let sub_escape = Arc::new(AtomicBool::new(false));
        let result = engine::query_loop(
            &context.api_client,
            &mut messages,
            &registry,
            &sub_config,
            &mut cost_tracker,
            &child_context,
            &sub_escape,
            Some(&system_prompt),
            None, // sub-agents use direct print, no bridge
        )
        .await;

        if let Err(ref e) = result {
            return Ok(ToolResult::error(format!("Sub-agent error: {e}")));
        }

        let final_text = extract_final_text(&messages);
        let cost = cost_tracker.estimate_cost_usd(&context.config.model);
        let summary = format!(
            "{final_text}\n\n[Sub-agent used {} input + {} output tokens, ${cost:.4}]",
            cost_tracker.total_input_tokens, cost_tracker.total_output_tokens,
        );

        Ok(ToolResult::success(summary))
    }

    /// Launch a sub-agent in the background, returning immediately.
    async fn launch_background(
        &self,
        prompt: &str,
        agent_type: &str,
        description: &str,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        let task_id = {
            let mut bg = context.background_tasks.lock().await;
            bg.register(description.to_string(), agent_type.to_string())
        };

        let (registry, sub_config, child_context, system_prompt) =
            build_agent_setup(agent_type, context).await;

        let prompt_owned = prompt.to_string();
        let api_client = context.api_client.clone();
        let model = context.config.model.clone();
        let pricing = context.config.pricing_table.clone();
        let bg_store = context.background_tasks.clone();
        let task_id_clone = task_id.clone();
        let agent_type_owned = agent_type.to_string();

        println!(
            "\n  {}",
            format!(
                "[Agent ({agent_type_owned}): launched in background as task #{task_id}]"
            )
            .cyan()
        );

        // Spawn detached tokio task — runs independently of the main conversation
        tokio::spawn(async move {
            let mut messages = vec![Message::user_text(&prompt_owned)];
            let mut cost_tracker = CostTracker::with_pricing(pricing);
            let sub_escape = Arc::new(AtomicBool::new(false));

            let result = engine::query_loop(
                &api_client,
                &mut messages,
                &registry,
                &sub_config,
                &mut cost_tracker,
                &child_context,
                &sub_escape,
                Some(&system_prompt),
                None, // background agents use direct print, no bridge
            )
            .await;

            let mut bg = bg_store.lock().await;
            match result {
                Ok(()) => {
                    let final_text = extract_final_text(&messages);
                    let cost = cost_tracker.estimate_cost_usd(&model);
                    let summary = format!(
                        "{final_text}\n\n[Sub-agent used {} input + {} output tokens, ${cost:.4}]",
                        cost_tracker.total_input_tokens, cost_tracker.total_output_tokens,
                    );
                    bg.complete(&task_id_clone, summary);
                }
                Err(e) => {
                    bg.fail(&task_id_clone, format!("Sub-agent error: {e}"));
                }
            }

            eprintln!(
                "{}",
                format!(
                    "\n  [Background agent #{} ({}) completed]",
                    task_id_clone, agent_type_owned
                )
                .cyan()
            );
        });

        Ok(ToolResult::success(format!(
            "Agent launched in background as task #{task_id}. \
             You will be automatically notified when it completes. \
             Continue with other work — do NOT poll or check on it. \
             Use TaskOutput with task_id=\"{task_id}\" to retrieve results after notification."
        )))
    }
}

/// Extract the final assistant text response from a message history.
fn extract_final_text(messages: &[Message]) -> String {
    messages
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
        .unwrap_or_else(|| "(no response from sub-agent)".to_string())
}

/// Build the common setup for a sub-agent: registry, config, context, system prompt.
async fn build_agent_setup(
    agent_type: &str,
    parent_context: &ToolContext,
) -> (ToolRegistry, crate::config::Config, ToolContext, String) {
    let registry = build_subagent_registry(agent_type);

    let mut sub_config = parent_context.config.clone();
    sub_config.max_turns = MAX_SUBAGENT_TURNS;
    sub_config.auto_model_routing = false;
    sub_config.model = crate::engine::model_router::MODEL_HEAVY.to_string();

    let current_mode = *parent_context.permission_mode.lock().await;
    let child_context = ToolContext {
        cwd: parent_context.cwd.clone(),
        permission_mode: Arc::new(Mutex::new(current_mode)),
        task_store: parent_context.task_store.clone(),
        background_tasks: parent_context.background_tasks.clone(),
        api_client: parent_context.api_client.clone(),
        config: sub_config.clone(),
        lsp_manager: parent_context.lsp_manager.clone(),
        plan_file_path: Arc::new(Mutex::new(None)),
    };

    let system_prompt = context::build_subagent_system_prompt(
        &parent_context.config.cwd,
        &sub_config.model,
        agent_type,
    );

    (registry, sub_config, child_context, system_prompt)
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
    // Explore and plan agents also get LSP for code intelligence
    registry.register(Box::new(lsp::LspTool));

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
