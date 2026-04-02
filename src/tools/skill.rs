use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

pub struct SkillTool;

// Bundled skills embedded at compile time
const SKILL_COMMIT: &str = include_str!("../skills/commit.md");
const SKILL_REVIEW_PR: &str = include_str!("../skills/review-pr.md");
const SKILL_SIMPLIFY: &str = include_str!("../skills/simplify.md");

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "Skill"
    }

    fn description(&self) -> String {
        "Execute a named skill (slash command). Skills provide specialized prompts \
         for common workflows like committing, reviewing PRs, or simplifying code."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
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
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true // The skill itself is just a prompt expansion
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
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
            return Ok(ToolResult::error(format!(
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
            return Ok(ToolResult::error(format!(
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

        Ok(ToolResult::success(expanded))
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
