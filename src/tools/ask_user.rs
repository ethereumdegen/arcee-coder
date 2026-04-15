use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::io::{self, Write};
use std::sync::OnceLock;

pub struct AskUserTool;

const DESCRIPTION: &str = "Use this tool when you need to ask the user questions during execution. This allows you to:\n\
1. Gather user preferences or requirements\n\
2. Clarify ambiguous instructions\n\
3. Get decisions on implementation choices as you work\n\
4. Offer choices to the user about what direction to take.\n\n\
Usage notes:\n\
- Users will always be able to select \"Other\" to provide custom text input\n\
- Use multiSelect: true to allow multiple answers to be selected for a question\n\
- If you recommend a specific option, make that the first option in the list and add \"(Recommended)\" at the end of the label\n\n\
Plan mode note: In plan mode, use this tool to clarify requirements or choose between approaches BEFORE finalizing your plan. \
Do NOT use this tool to ask \"Is my plan ready?\" or \"Should I proceed?\" - use ExitPlanMode for plan approval. \
IMPORTANT: Do not reference \"the plan\" in your questions (e.g., \"Do you have feedback about the plan?\", \
\"Does the plan look good?\") because the user cannot see the plan in the UI until you call ExitPlanMode. \
If you need plan approval, use ExitPlanMode instead.\n\n\
Preview feature:\n\
Use the optional `markdown` field on options when presenting concrete artifacts that users need to visually compare:\n\
- ASCII mockups of UI layouts or components\n\
- Code snippets showing different implementations\n\
- Diagram variations\n\
- Configuration examples\n\n\
When any option has a markdown, the UI switches to a side-by-side layout with a vertical option list on the left and \
preview on the right. Do not use previews for simple preference questions where labels and descriptions suffice. \
Note: previews are only supported for single-select questions (not multiSelect).";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "questions": {
                    "description": "Questions to ask the user (1-4 questions)",
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 4,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "The complete question to ask the user. Should be clear, specific, and end with a question mark."
                            },
                            "header": {
                                "type": "string",
                                "description": "Very short label displayed as a chip/tag (max 12 chars). Examples: \"Auth method\", \"Library\", \"Approach\"."
                            },
                            "options": {
                                "description": "The available choices for this question. Must have 2-4 options.",
                                "type": "array",
                                "minItems": 2,
                                "maxItems": 4,
                                "items": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "The display text for this option (1-5 words)."
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": "Explanation of what this option means or what will happen if chosen."
                                        },
                                        "markdown": {
                                            "type": "string",
                                            "description": "Optional preview content shown when this option is focused. Use for ASCII mockups, code snippets, or diagrams."
                                        }
                                    },
                                    "required": ["label", "description"]
                                }
                            },
                            "multiSelect": {
                                "type": "boolean",
                                "default": false,
                                "description": "Set to true to allow the user to select multiple options instead of just one."
                            }
                        },
                        "required": ["question", "header", "options", "multiSelect"]
                    }
                },
                "annotations": {
                    "type": "object",
                    "description": "Optional per-question annotations from the user (e.g., notes on preview selections). Keyed by question text.",
                    "additionalProperties": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "markdown": {
                                "type": "string",
                                "description": "The markdown preview content of the selected option."
                            },
                            "notes": {
                                "type": "string",
                                "description": "Free-text notes the user added to their selection."
                            }
                        }
                    }
                },
                "answers": {
                    "type": "object",
                    "description": "User answers collected by the permission component",
                    "additionalProperties": { "type": "string" }
                },
                "metadata": {
                    "type": "object",
                    "additionalProperties": false,
                    "description": "Optional metadata for tracking and analytics purposes. Not displayed to user.",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Optional identifier for the source of this question."
                        }
                    }
                }
            },
            "required": ["questions"]
        })
    })
}

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &'static str {
        "AskUserQuestion"
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

    async fn call(&self, input: serde_json::Value, _context: &ToolContext) -> Result<ToolOutput> {
        // Support both new multi-question format and legacy single-question format
        let question = if let Some(questions) = input["questions"].as_array() {
            // New format: extract questions and format them
            let mut parts = Vec::new();
            for q in questions {
                let q_text = q["question"].as_str().unwrap_or("(no question)");
                let mut part = q_text.to_string();
                if let Some(options) = q["options"].as_array() {
                    for (i, opt) in options.iter().enumerate() {
                        let label = opt["label"].as_str().unwrap_or("?");
                        let desc = opt["description"].as_str().unwrap_or("");
                        part.push_str(&format!("\n  {}. {} — {}", i + 1, label, desc));
                    }
                }
                parts.push(part);
            }
            parts.join("\n\n")
        } else if let Some(q) = input["question"].as_str() {
            q.to_string()
        } else {
            return Err(anyhow::anyhow!("Missing 'questions' parameter"));
        };
        let question = &question;

        // In interactive (iocraft) mode, stdin is in raw mode and managed by the UI thread.
        // We cannot safely read from it here. Instead, surface the question as the tool result
        // so the model includes it in its output; the user will see it and respond naturally.
        if crossterm::terminal::is_raw_mode_enabled().unwrap_or(false) {
            return Ok(ToolOutput::success()
                .with_summary("Question surfaced to user")
                .with_text(format!(
                    "QUESTION FOR USER: {question}\n\n\
                     (The question has been displayed. Stop here and wait for the user to respond \
                     in their next message.)"
                ))
                .with_next_step("Wait for the user's next message before taking further actions"));
        }

        // Non-interactive (oneshot) mode: read from stdin directly.
        println!("\n{}", question);
        print!("> ");
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let response = response.trim().to_string();

        if response.is_empty() {
            Ok(ToolOutput::empty("(no response from user)"))
        } else {
            Ok(ToolOutput::success()
                .with_summary(format!("User replied with {} chars", response.len()))
                .with_text(response))
        }
    }
}
