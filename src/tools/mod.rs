pub mod ask_user;
pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod path_safety;
pub mod read;
pub mod web_fetch;
pub mod write;

use crate::api::types::ToolDefinition;
use crate::permissions::PermissionMode;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

/// Simplified tool info for building API definitions.
pub struct ToolDefInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Context provided to tools during execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub cwd: PathBuf,
    pub permission_mode: PermissionMode,
}

/// Result returned by a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
        }
    }
}

/// The core Tool trait that all tools implement.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique name of this tool.
    fn name(&self) -> &str;

    /// Human-readable description for the model.
    fn description(&self) -> String;

    /// JSON Schema for the tool's input parameters.
    fn input_schema(&self) -> serde_json::Value;

    /// Whether this tool only reads (doesn't modify) state.
    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        false
    }

    /// Execute the tool with the given input.
    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult>;
}

/// Registry of all available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        let mut defs: Vec<_> = self
            .tools
            .values()
            .map(|tool| ToolDefinition::new(
                tool.name(),
                tool.description(),
                tool.input_schema(),
            ))
            .collect();
        defs.sort_by(|a, b| a.function.name.cmp(&b.function.name));
        defs
    }

    pub fn tool_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }
}

/// Build the default tool registry with all built-in tools.
pub fn build_default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(bash::BashTool));
    registry.register(Box::new(read::ReadTool));
    registry.register(Box::new(write::WriteTool));
    registry.register(Box::new(edit::EditTool));
    registry.register(Box::new(glob::GlobTool));
    registry.register(Box::new(grep::GrepTool));
    registry.register(Box::new(web_fetch::WebFetchTool));
    registry.register(Box::new(ask_user::AskUserTool));
    registry
}
