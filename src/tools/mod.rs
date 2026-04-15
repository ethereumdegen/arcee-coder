pub mod agent;
pub mod ask_user;
pub mod background_tasks;
pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod lsp;
pub mod notebook_edit;
pub mod output;
pub mod path_safety;
pub mod plan_mode;
pub mod read;
pub mod skill;
pub mod task_create;
pub mod task_get;
pub mod task_list;
pub mod task_output;
pub mod task_store;
pub mod task_update;
pub mod web_fetch;
pub mod web_search;
pub mod worktree;
pub mod write;

pub use output::{ToolBody, ToolOutput, Truncation};

use crate::api::types::ToolDefinition;
use crate::config::Config;
use crate::permissions::PermissionMode;
use crate::provider::Provider;
use crate::tools::background_tasks::BackgroundTaskStore;
use crate::tools::lsp::LspManager;
use crate::tools::task_store::TaskStore;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Permission class reported by a tool for the engine to enforce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionClass {
    /// Always allowed (pure read).
    ReadOnly,
    /// Requires an "ask" prompt unless the permission mode bypasses it.
    Ask,
    /// Always denied.
    Forbidden,
}

impl PermissionClass {
    pub fn is_read_only(self) -> bool {
        matches!(self, PermissionClass::ReadOnly)
    }
}

/// Context provided to tools during execution.
#[derive(Clone)]
pub struct ToolContext {
    pub cwd: PathBuf,
    pub permission_mode: Arc<Mutex<PermissionMode>>,
    pub task_store: Arc<Mutex<TaskStore>>,
    pub background_tasks: Arc<Mutex<BackgroundTaskStore>>,
    pub provider: Arc<dyn Provider>,
    pub config: Config,
    pub lsp_manager: Arc<Mutex<LspManager>>,
    pub plan_file_path: Arc<Mutex<Option<PathBuf>>>,
}

/// Core `Tool` trait. All tools implement this.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique name of this tool.
    fn name(&self) -> &'static str;

    /// Human-readable description for the model.
    fn description(&self) -> &'static str;

    /// JSON Schema for the tool's input parameters. Returns a `&'static`
    /// reference backed by `OnceLock` so definitions can be cached at
    /// registration time without per-turn allocation.
    fn input_schema(&self) -> &'static serde_json::Value;

    /// Classify the tool's permission requirement based on the concrete
    /// input. Default: `Ask`. Read-only tools should override to return
    /// `PermissionClass::ReadOnly`.
    fn permission(&self, _input: &serde_json::Value) -> PermissionClass {
        PermissionClass::Ask
    }

    /// Execute the tool with the given input.
    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput>;
}

/// Registry of all available tools.
///
/// At registration time the `definitions` vector is rebuilt once and reused
/// for every turn — fixing the O(N) per-turn rebuild that the previous
/// implementation incurred.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    definitions_cache: Vec<ToolDefinition>,
    names_cache: Vec<String>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            definitions_cache: Vec::new(),
            names_cache: Vec::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
        self.rebuild_caches();
    }

    fn rebuild_caches(&mut self) {
        let mut defs: Vec<_> = self
            .tools
            .values()
            .map(|tool| {
                ToolDefinition::new(
                    tool.name().to_string(),
                    tool.description().to_string(),
                    tool.input_schema().clone(),
                )
            })
            .collect();
        defs.sort_by(|a, b| a.function.name.cmp(&b.function.name));
        self.definitions_cache = defs;

        let mut names: Vec<_> = self.tools.keys().cloned().collect();
        names.sort();
        self.names_cache = names;
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Return the cached tool definitions. O(1) — does not rebuild per turn.
    pub fn definitions(&self) -> &[ToolDefinition] {
        &self.definitions_cache
    }

    /// Clone the definitions when the API layer needs an owned Vec.
    pub fn definitions_cloned(&self) -> Vec<ToolDefinition> {
        self.definitions_cache.clone()
    }

    pub fn tool_names(&self) -> &[String] {
        &self.names_cache
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the default tool registry with all built-in tools.
pub fn build_default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Core tools
    registry.register(Box::new(bash::BashTool));
    registry.register(Box::new(read::ReadTool));
    registry.register(Box::new(write::WriteTool));
    registry.register(Box::new(edit::EditTool));
    registry.register(Box::new(glob::GlobTool));
    registry.register(Box::new(grep::GrepTool));
    registry.register(Box::new(web_fetch::WebFetchTool));
    registry.register(Box::new(ask_user::AskUserTool));

    // Task management
    registry.register(Box::new(task_create::TaskCreateTool));
    registry.register(Box::new(task_update::TaskUpdateTool));
    registry.register(Box::new(task_list::TaskListTool));
    registry.register(Box::new(task_get::TaskGetTool));
    registry.register(Box::new(task_output::TaskOutputTool));

    // Agent (sub-agents)
    registry.register(Box::new(agent::AgentTool));

    // Plan mode
    registry.register(Box::new(plan_mode::EnterPlanModeTool));
    registry.register(Box::new(plan_mode::ExitPlanModeTool));

    // Web search
    registry.register(Box::new(web_search::WebSearchTool));

    // LSP code intelligence
    registry.register(Box::new(lsp::LspTool));

    // Jupyter notebook editing
    registry.register(Box::new(notebook_edit::NotebookEditTool));

    // Git worktree
    registry.register(Box::new(worktree::EnterWorktreeTool));
    registry.register(Box::new(worktree::ExitWorktreeTool));

    // Skills
    registry.register(Box::new(skill::SkillTool));

    registry
}
