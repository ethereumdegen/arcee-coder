use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::io::{self, Write};

pub struct AskUserTool;

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "AskUserQuestion"
    }

    fn description(&self) -> String {
        "Ask the user a question and wait for their response. Use this when you need \
         clarification or input from the user to proceed."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to ask the user"
                }
            },
            "required": ["question"]
        })
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, _context: &ToolContext) -> Result<ToolResult> {
        let question = input["question"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'question' parameter"))?;

        // In interactive (iocraft) mode, stdin is in raw mode and managed by the UI thread.
        // We cannot safely read from it here. Instead, surface the question as the tool result
        // so the model includes it in its output; the user will see it and respond naturally.
        if crossterm::terminal::is_raw_mode_enabled().unwrap_or(false) {
            return Ok(ToolResult::success(format!(
                "QUESTION FOR USER: {question}\n\n\
                 (The question has been displayed. Stop here and wait for the user to respond \
                 in their next message.)"
            )));
        }

        // Non-interactive (oneshot) mode: read from stdin directly.
        println!("\n{}", question);
        print!("> ");
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let response = response.trim().to_string();

        if response.is_empty() {
            Ok(ToolResult::success("(no response from user)"))
        } else {
            Ok(ToolResult::success(response))
        }
    }
}
