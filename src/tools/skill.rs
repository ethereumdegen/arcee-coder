use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct SkillTool;

// Bundled skills embedded at compile time
const SKILL_COMMIT: &str = include_str!("../skills/commit.md");
const SKILL_REVIEW_PR: &str = include_str!("../skills/review-pr.md");
const SKILL_SIMPLIFY: &str = include_str!("../skills/simplify.md");

const DESCRIPTION: &str = "Execute a skill within the main conversation\n\n\
When users ask you to perform tasks, check if any of the available skills match. Skills provide specialized capabilities and domain knowledge.\n\n\
When users reference a \"slash command\" or \"/<something>\" (e.g., \"/commit\", \"/review-pr\"), they are referring to a skill. Use this tool to invoke it.\n\n\
How to invoke:\n\
- Use this tool with the skill name and optional arguments\n\
- Examples:\n\
  - `skill: \"pdf\"` - invoke the pdf skill\n\
  - `skill: \"commit\", args: \"-m 'Fix bug'\"` - invoke with arguments\n\
  - `skill: \"review-pr\", args: \"123\"` - invoke with arguments\n\n\
Important:\n\
- Available skills are listed in system-reminder messages in the conversation\n\
- When a skill matches the user's request, this is a BLOCKING REQUIREMENT: invoke the relevant Skill tool BEFORE generating any other response about the task\n\
- NEVER mention a skill without actually calling this tool\n\
- Do not invoke a skill that is already running\n\
- Do not use this tool for built-in CLI commands (like /help, /clear, etc.)\n\
- If you see a <command-name> tag in the current conversation turn, the skill has ALREADY been loaded - follow the instructions directly instead of calling this tool again";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "The skill name (e.g., 'commit', 'review-pr', 'simplify')"
                },
                "args": {
                    "type": "string",
                    "description": "Optional arguments for the skill"
                }
            },
            "required": ["skill"]
        })
    })
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &'static str {
        "Skill"
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        schema()
    }

    fn permission(&self, _input: &serde_json::Value) -> PermissionClass {
        PermissionClass::ReadOnly
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let skill_name = input["skill"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'skill' parameter"))?;
        let args = input["args"].as_str().unwrap_or("");

        // Sanitize skill name: reject path separators and traversal
        if skill_name.contains('/')
            || skill_name.contains('\\')
            || skill_name.contains("..")
            || skill_name.is_empty()
        {
            return Ok(ToolOutput::error(format!(
                "Invalid skill name '{skill_name}'. Skill names must be simple identifiers (no path separators)."
            )));
        }

        // Try project-local skills first
        let project_skill_path = context.cwd.join(".arcee").join("skills").join(format!("{skill_name}.md"));
        let skill_content = if project_skill_path.exists() {
            tokio::fs::read_to_string(&project_skill_path).await.ok()
        } else {
            None
        };

        // Fall back to bundled skills
        let skill_content = skill_content.unwrap_or_else(|| {
            get_bundled_skill(skill_name)
                .map(String::from)
                .unwrap_or_default()
        });

        if skill_content.is_empty() {
            let available = list_available_skills(context).await;
            return Ok(ToolOutput::error(format!(
                "Unknown skill '{skill_name}'. Available skills: {}",
                available.join(", ")
            )));
        }

        // Parse YAML frontmatter if present
        let (metadata, prompt_body) = parse_skill_file(&skill_content);

        // Build the expanded prompt
        let mut expanded = String::new();

        if let Some(desc) = metadata.get("description") {
            expanded.push_str(&format!("# Skill: {skill_name}\n{desc}\n\n"));
        } else {
            expanded.push_str(&format!("# Skill: {skill_name}\n\n"));
        }

        expanded.push_str(prompt_body);

        if !args.is_empty() {
            expanded.push_str(&format!("\n\nArguments: {args}"));
        }

        Ok(ToolOutput::success()
            .with_summary(format!("loaded skill '{skill_name}'"))
            .with_text(expanded))
    }
}

fn get_bundled_skill(name: &str) -> Option<&'static str> {
    match name {
        "commit" => Some(SKILL_COMMIT),
        "review-pr" => Some(SKILL_REVIEW_PR),
        "simplify" => Some(SKILL_SIMPLIFY),
        _ => None,
    }
}

async fn list_available_skills(context: &ToolContext) -> Vec<String> {
    let mut skills = vec![
        "commit".to_string(),
        "review-pr".to_string(),
        "simplify".to_string(),
    ];

    // Add project-local skills
    let skills_dir = context.cwd.join(".arcee").join("skills");
    if skills_dir.is_dir() {
        if let Ok(mut entries) = tokio::fs::read_dir(&skills_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if !skills.contains(&stem.to_string()) {
                            skills.push(stem.to_string());
                        }
                    }
                }
            }
        }
    }

    skills
}

/// Parse a skill file with optional YAML frontmatter.
/// Returns (metadata, body).
fn parse_skill_file(content: &str) -> (HashMap<String, String>, &str) {
    let mut metadata = HashMap::new();

    if content.starts_with("---\n") {
        if let Some(end) = content[4..].find("\n---") {
            let frontmatter = &content[4..4 + end];
            let body = &content[4 + end + 4..];

            // Simple YAML key: value parsing
            for line in frontmatter.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim().to_string();
                    let value = value.trim().trim_matches('"').to_string();
                    metadata.insert(key, value);
                }
            }

            return (metadata, body.trim_start());
        }
    }

    (metadata, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_file_with_frontmatter() {
        let content = "---\ndescription: Test skill\nauthor: test\n---\nDo the thing.";
        let (meta, body) = parse_skill_file(content);
        assert_eq!(meta.get("description").unwrap(), "Test skill");
        assert_eq!(meta.get("author").unwrap(), "test");
        assert_eq!(body, "Do the thing.");
    }

    #[test]
    fn test_parse_skill_file_without_frontmatter() {
        let content = "Just a prompt body.";
        let (meta, body) = parse_skill_file(content);
        assert!(meta.is_empty());
        assert_eq!(body, "Just a prompt body.");
    }
}
