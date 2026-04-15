use crate::api::types::ContentBlock;
use crate::engine;
use crate::engine::context;
use crate::engine::cost::CostTracker;
use crate::messages::types::*;
use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput, ToolRegistry};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;

pub struct AgentTool;

const MAX_SUBAGENT_TURNS: usize = 25;

const DESCRIPTION: &str = "Launch a new agent to handle complex, multi-step tasks autonomously.\n\n\
The Agent tool launches specialized agents (subprocesses) that autonomously handle complex tasks. \
Each agent type has specific capabilities and tools available to it.\n\n\
Available agent types and the tools they have access to:\n\
- general-purpose: General-purpose agent for researching complex questions, searching for code, and executing multi-step tasks. \
When you are searching for a keyword or file and are not confident that you will find the right match in the first few tries \
use this agent to perform the search for you. (Tools: *)\n\
- Explore: Fast agent specialized for exploring codebases. Use this when you need to quickly find files by patterns \
(eg. \"src/components/**/*.tsx\"), search code for keywords (eg. \"API endpoints\"), or answer questions about the codebase \
(eg. \"how do API endpoints work?\"). When calling this agent, specify the desired thoroughness level: \"quick\" for basic searches, \
\"medium\" for moderate exploration, or \"very thorough\" for comprehensive analysis across multiple locations and naming conventions. \
(Tools: All tools except Agent, ExitPlanMode, Edit, Write, NotebookEdit)\n\
- Plan: Software architect agent for designing implementation plans. Use this when you need to plan the implementation strategy \
for a task. Returns step-by-step plans, identifies critical files, and considers architectural trade-offs. \
(Tools: All tools except Agent, ExitPlanMode, Edit, Write, NotebookEdit)\n\n\
When using the Agent tool, you must specify a subagent_type parameter to select which agent type to use.\n\n\
When NOT to use the Agent tool:\n\
- If you want to read a specific file path, use the Read or Glob tool instead of the Agent tool, to find the match more quickly\n\
- If you are searching for a specific class definition like \"class Foo\", use the Glob tool instead, to find the match more quickly\n\
- If you are searching for code within a specific file or set of 2-3 files, use the Read tool instead of the Agent tool\n\
- Other tasks that are not related to the agent descriptions above\n\n\
Usage notes:\n\
- Always include a short description (3-5 words) summarizing what the agent will do\n\
- Launch multiple agents concurrently whenever possible, to maximize performance; to do that, use a single message with multiple tool uses\n\
- When the agent is done, it will return a single message back to you. The result returned by the agent is not visible to the user. \
To show the user the result, you should send a text message back to the user with a concise summary of the result.\n\
- You can optionally run agents in the background using the run_in_background parameter. When an agent runs in the background, \
you will be automatically notified when it completes — do NOT sleep, poll, or proactively check on its progress.\n\
- Foreground vs background: Use foreground (default) when you need the agent's results before you can proceed. \
Use background when you have genuinely independent work to do in parallel.\n\
- Agents can be resumed using the `resume` parameter by passing the agent ID from a previous invocation.\n\
- Provide clear, detailed prompts so the agent can work autonomously and return exactly the information you need.\n\
- Clearly tell the agent whether you expect it to write code or just to do research.\n\
- If the agent description mentions that it should be used proactively, then you should try your best to use it without the user having to ask.\n\
- You can optionally set `isolation: \"worktree\"` to run the agent in a temporary git worktree, giving it an isolated copy of the repository.";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "description": {
                    "type": "string",
                    "description": "A short (3-5 word) description of the task"
                },
                "prompt": {
                    "type": "string",
                    "description": "The task for the agent to perform"
                },
                "subagent_type": {
                    "type": "string",
                    "description": "The type of specialized agent to use for this task"
                },
                "model": {
                    "type": "string",
                    "description": "Optional model to use for this agent. If not specified, inherits from parent. Prefer haiku for quick, straightforward tasks to minimize cost and latency.",
                    "enum": ["sonnet", "opus", "haiku"]
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Set to true to run this agent in the background. You will be notified when it completes."
                },
                "resume": {
                    "type": "string",
                    "description": "Optional agent ID to resume from. If provided, the agent will continue from the previous execution transcript."
                },
                "max_turns": {
                    "type": "integer",
                    "description": "Maximum number of agentic turns (API round-trips) before stopping.",
                    "exclusiveMinimum": 0
                },
                "isolation": {
                    "type": "string",
                    "description": "Isolation mode. \"worktree\" creates a temporary git worktree so the agent works on an isolated copy of the repo.",
                    "enum": ["worktree"]
                }
            },
            "required": ["description", "prompt", "subagent_type"]
        })
    })
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &'static str {
        "Agent"
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        schema()
    }

    fn permission(&self, input: &serde_json::Value) -> PermissionClass {
        let agent_type = input["subagent_type"].as_str().unwrap_or("explore");
        if matches!(agent_type, "explore" | "Explore" | "plan" | "Plan") {
            PermissionClass::ReadOnly
        } else {
            PermissionClass::Ask
        }
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let prompt = input["prompt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'prompt' parameter"))?;
        // Map claude-code agent type names to internal types
        let raw_type = input["subagent_type"].as_str().unwrap_or("explore");
        let agent_type = match raw_type {
            "general-purpose" | "general" => "general",
            "Explore" | "explore" => "explore",
            "Plan" | "plan" => "plan",
            _ => raw_type,
        };
        let run_in_background = input["run_in_background"].as_bool().unwrap_or(false);
        let description = input["description"]
            .as_str()
            .unwrap_or("sub-agent task")
            .to_string();

        if prompt.trim().is_empty() {
            return Ok(ToolOutput::error("Prompt cannot be empty"));
        }

        if run_in_background {
            return self
                .launch_background(prompt, agent_type, &description, context)
                .await;
        }

        self.run_foreground(prompt, agent_type, context).await
    }
}

impl AgentTool {
    async fn run_foreground(
        &self,
        prompt: &str,
        agent_type: &str,
        context: &ToolContext,
    ) -> Result<ToolOutput> {
        let (registry, sub_config, child_context, system_prompt) =
            build_agent_setup(agent_type, context).await;

        // Fire SubagentStart hook (legacy compat)
        crate::hooks::run_event_hooks(
            &context.config.hooks,
            "SubagentStart",
            json!({ "agent_type": agent_type, "prompt": prompt }),
            &context.cwd,
        )
        .await;

        let mut messages = vec![Message::user_text(prompt)];
        let mut cost_tracker = CostTracker::with_pricing(context.config.pricing_table.clone());

        let sub_escape = Arc::new(AtomicBool::new(false));
        let result = engine::query_loop(
            context.provider.as_ref(),
            &mut messages,
            &registry,
            &sub_config,
            &mut cost_tracker,
            &child_context,
            &sub_escape,
            Some(&system_prompt),
            None, // sub-agents use direct print, no bridge
            crate::output::OutputFormat::Text,
        )
        .await;

        // Fire SubagentStop hook
        crate::hooks::run_event_hooks(
            &context.config.hooks,
            "SubagentStop",
            json!({ "agent_type": agent_type, "success": result.is_ok() }),
            &context.cwd,
        )
        .await;

        if let Err(ref e) = result {
            return Ok(ToolOutput::error(format!("Sub-agent error: {e}")));
        }

        let final_text = extract_final_text(&messages);
        let cost = cost_tracker.estimate_cost_usd(&context.config.model);
        let summary = format!(
            "sub-agent {} — {} in + {} out tokens, ${cost:.4}",
            agent_type,
            cost_tracker.total_input_tokens,
            cost_tracker.total_output_tokens
        );

        Ok(ToolOutput::success()
            .with_summary(summary)
            .with_text(final_text))
    }

    async fn launch_background(
        &self,
        prompt: &str,
        agent_type: &str,
        description: &str,
        context: &ToolContext,
    ) -> Result<ToolOutput> {
        let task_id = {
            let mut bg = context.background_tasks.lock().await;
            bg.register(description.to_string(), agent_type.to_string())
        };

        let (registry, sub_config, child_context, system_prompt) =
            build_agent_setup(agent_type, context).await;

        let prompt_owned = prompt.to_string();
        let provider = context.provider.clone();
        let model = context.config.model.clone();
        let pricing = context.config.pricing_table.clone();
        let bg_store = context.background_tasks.clone();
        let task_id_clone = task_id.clone();

        tokio::spawn(async move {
            let mut messages = vec![Message::user_text(&prompt_owned)];
            let mut cost_tracker = CostTracker::with_pricing(pricing);
            let sub_escape = Arc::new(AtomicBool::new(false));

            let result = engine::query_loop(
                provider.as_ref(),
                &mut messages,
                &registry,
                &sub_config,
                &mut cost_tracker,
                &child_context,
                &sub_escape,
                Some(&system_prompt),
                None,
                crate::output::OutputFormat::Text,
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
        });

        Ok(ToolOutput::success()
            .with_summary(format!("Launched sub-agent #{task_id} in background"))
            .with_text(format!(
                "Agent launched in background as task #{task_id}. \
                 You will be automatically notified when it completes. \
                 Continue with other work — do NOT poll or check on it."
            ))
            .with_next_step(format!(
                "Call TaskOutput with task_id=\"{task_id}\" after you are notified"
            )))
    }
}

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
        provider: parent_context.provider.clone(),
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
    registry.register(Box::new(lsp::LspTool));

    if agent_type == "general" {
        registry.register(Box::new(bash::BashTool));
        registry.register(Box::new(write::WriteTool));
        registry.register(Box::new(edit::EditTool));
        registry.register(Box::new(task_create::TaskCreateTool));
        registry.register(Box::new(task_update::TaskUpdateTool));
        registry.register(Box::new(notebook_edit::NotebookEditTool));
    }

    registry
}
