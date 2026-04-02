pub mod paths;

use crate::engine::cost::PricingTable;
use crate::engine::model_router::Intensity;
use crate::permissions::{PermissionMode, PermissionStrictness};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_model")]
    pub model: String,

    #[serde(skip)]
    pub api_key: String,

    #[serde(default = "default_base_url")]
    pub base_url: String,

    #[serde(default)]
    pub permission_mode: PermissionMode,

    #[serde(default)]
    pub permission_strictness: PermissionStrictness,

    #[serde(default = "default_max_turns")]
    pub max_turns: usize,

    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    #[serde(default)]
    pub budget_usd: Option<f64>,

    #[serde(default)]
    pub allow_rules: Vec<PermissionRule>,

    #[serde(default)]
    pub deny_rules: Vec<PermissionRule>,

    #[serde(skip)]
    pub config_dir: PathBuf,

    #[serde(skip)]
    pub cwd: PathBuf,

    #[serde(default)]
    pub verbose: bool,

    /// Automatically switch between trinity-mini and trinity-large-thinking.
    #[serde(default = "default_auto_routing")]
    pub auto_model_routing: bool,

    /// Routing intensity: high (always big model), medium (balanced), low (prefer cheap).
    #[serde(default)]
    pub intensity: Intensity,

    /// Dynamic pricing table fetched from the API on boot.
    #[serde(skip)]
    pub pricing_table: PricingTable,

    /// Hooks configuration for PreToolUse / PostToolUse events.
    #[serde(default)]
    pub hooks: crate::hooks::HooksConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub tool_name: String,
    pub pattern: Option<String>,
}

fn default_model() -> String {
    "trinity-large-thinking".to_string()
}

fn default_base_url() -> String {
    "https://api.arcee.ai".to_string()
}

fn default_max_turns() -> usize {
    200
}

fn default_max_tokens() -> u32 {
    16384
}

fn default_auto_routing() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: default_model(),
            api_key: String::new(),
            base_url: default_base_url(),
            permission_mode: PermissionMode::Default,
            permission_strictness: PermissionStrictness::default(),
            max_turns: default_max_turns(),
            max_tokens: default_max_tokens(),
            budget_usd: None,
            allow_rules: Vec::new(),
            deny_rules: Vec::new(),
            config_dir: paths::config_dir(),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            verbose: false,
            auto_model_routing: default_auto_routing(),
            intensity: Intensity::default(),
            pricing_table: PricingTable::new(),
            hooks: Default::default(),
        }
    }
}

impl Config {
    /// Load configuration from files and environment, with CLI overrides.
    pub fn load(cli: &CliOverrides) -> Result<Self> {
        let mut config = Self::default();

        // 1. Load user-level config
        let user_config_path = paths::config_dir().join("config.json");
        if user_config_path.exists() {
            let content = std::fs::read_to_string(&user_config_path)?;
            match serde_json::from_str::<Config>(&content) {
                Ok(file_config) => {
                    config.model = file_config.model;
                    config.permission_mode = file_config.permission_mode;
                    config.permission_strictness = file_config.permission_strictness;
                    config.max_turns = file_config.max_turns;
                    config.max_tokens = file_config.max_tokens;
                    config.allow_rules = file_config.allow_rules;
                    config.deny_rules = file_config.deny_rules;
                    config.auto_model_routing = file_config.auto_model_routing;
                    config.intensity = file_config.intensity;
                    config.hooks = file_config.hooks;
                }
                Err(e) => {
                    eprintln!(
                        "Warning: failed to parse {}: {}. Using defaults.",
                        user_config_path.display(),
                        e
                    );
                }
            }

            // Also check for saved api_key in config file
            if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(key) = raw["api_key"].as_str() {
                    if !key.is_empty() {
                        config.api_key = key.to_string();
                    }
                }
            }
        }

        // 2. Load project-level config
        let project_config_path = config.cwd.join(".arcee").join("settings.json");
        if project_config_path.exists() {
            let content = std::fs::read_to_string(&project_config_path)?;
            if let Ok(proj_config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(mode) = proj_config["permission_mode"].as_str() {
                    config.permission_mode =
                        serde_json::from_value(serde_json::Value::String(mode.to_string()))
                            .unwrap_or(config.permission_mode);
                }
                if let Some(s) = proj_config["permission_strictness"].as_str() {
                    config.permission_strictness =
                        serde_json::from_value(serde_json::Value::String(s.to_string()))
                            .unwrap_or(config.permission_strictness);
                }
                // Merge project-level hooks (additive on top of user-level)
                if let Ok(proj_hooks) =
                    serde_json::from_value::<crate::hooks::HooksConfig>(
                        proj_config["hooks"].clone(),
                    )
                {
                    crate::hooks::merge(&mut config.hooks, proj_hooks);
                }
            }
        }

        // 3. Environment variables (override config file)
        if let Ok(key) = std::env::var("ARCEE_API_KEY") {
            config.api_key = key;
        }
        if let Ok(url) = std::env::var("ARCEE_BASE_URL") {
            config.base_url = url;
        }
        if let Ok(model) = std::env::var("ARCEE_MODEL") {
            config.model = model;
        }
        if let Ok(s) = std::env::var("ARCEE_PERMISSION_STRICTNESS") {
            config.permission_strictness = match s.to_lowercase().as_str() {
                "high" => PermissionStrictness::High,
                "low" => PermissionStrictness::Low,
                _ => PermissionStrictness::Medium,
            };
        }

        // 4. CLI overrides (highest priority)
        if let Some(ref model) = cli.model {
            config.model = model.clone();
        }
        if let Some(mode) = cli.permission_mode {
            config.permission_mode = mode;
        }
        if let Some(s) = cli.permission_strictness {
            config.permission_strictness = s;
        }
        if let Some(turns) = cli.max_turns {
            config.max_turns = turns;
        }
        if let Some(budget) = cli.budget {
            config.budget_usd = Some(budget);
        }
        if cli.no_auto_route {
            config.auto_model_routing = false;
        }
        config.verbose = cli.verbose;

        // Validate
        if config.max_turns == 0 {
            config.max_turns = 1;
        }
        if config.max_tokens == 0 {
            config.max_tokens = default_max_tokens();
        }

        Ok(config)
    }
}

/// Overrides from CLI arguments.
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub model: Option<String>,
    pub permission_mode: Option<PermissionMode>,
    pub max_turns: Option<usize>,
    pub budget: Option<f64>,
    pub verbose: bool,
    pub resume: bool,
    pub resume_session_id: Option<String>,
    pub prompt: Option<String>,
    pub no_auto_route: bool,
    pub permission_strictness: Option<PermissionStrictness>,
    pub system_prompt: Option<String>,
    pub append_system_prompt: Option<String>,
    pub print_mode: bool,
    pub output_format: Option<String>,
}
