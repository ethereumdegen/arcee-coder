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

        // Print the question
        println!("\n{}", question);
        print!("> ");
        io::stdout().flush()?;

        // Read user input
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
