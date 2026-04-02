use crate::tools::path_safety::resolve_and_validate;
use crate::tools::{Tool, ToolContext, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct NotebookEditTool;

#[async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &str {
        "NotebookEdit"
    }

    fn description(&self) -> String {
        "Edit a Jupyter notebook (.ipynb file). Can replace, insert, or delete cells.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "notebook_path": {
                    "type": "string",
                    "description": "Absolute path to the .ipynb file"
                },
                "new_source": {
                    "type": "string",
                    "description": "New source content for the cell"
                },
                "cell_number": {
                    "type": "number",
                    "description": "0-indexed cell number to operate on"
                },
                "cell_type": {
                    "type": "string",
                    "description": "Cell type: 'code' or 'markdown'. Required for insert."
                },
                "edit_mode": {
                    "type": "string",
                    "description": "Edit mode: 'replace' (default), 'insert', or 'delete'"
                }
            },
            "required": ["notebook_path", "new_source"]
        })
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let notebook_path_str = input["notebook_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'notebook_path' parameter"))?;
        let new_source = input["new_source"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_source' parameter"))?;

        let cell_number = input["cell_number"].as_u64().map(|n| n as usize);
        let cell_type = input["cell_type"].as_str().unwrap_or("code");
        let edit_mode = input["edit_mode"].as_str().unwrap_or("replace");

        let notebook_path = match resolve_and_validate(notebook_path_str, &context.cwd) {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::error(e)),
        };

        if !notebook_path.exists() && edit_mode != "insert" {
            return Ok(ToolResult::error(format!(
                "Notebook not found: {}",
                notebook_path.display()
            )));
        }

        // Read the notebook
        let content = if notebook_path.exists() {
            tokio::fs::read_to_string(&notebook_path).await?
        } else {
            // Create a new notebook structure
            serde_json::to_string_pretty(&json!({
                "cells": [],
                "metadata": {
                    "kernelspec": {
                        "display_name": "Python 3",
                        "language": "python",
                        "name": "python3"
                    },
                    "language_info": {
                        "name": "python",
                        "version": "3.10.0"
                    }
                },
                "nbformat": 4,
                "nbformat_minor": 5
            }))?
        };

        let mut notebook: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse notebook {}: {e}",
                notebook_path.display()
            )
        })?;

        let cells = notebook["cells"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("Invalid notebook: missing 'cells' array"))?;

        // Split source into lines for notebook format
        let source_lines: Vec<serde_json::Value> = if new_source.is_empty() {
            vec![]
        } else {
            let total_lines = new_source.lines().count();
            new_source
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    if i < total_lines - 1 {
                        json!(format!("{line}\n"))
                    } else {
                        json!(line.to_string())
                    }
                })
                .collect()
        };

        match edit_mode {
            "replace" => {
                let idx = cell_number.unwrap_or(0);
                if idx >= cells.len() {
                    return Ok(ToolResult::error(format!(
                        "Cell index {idx} out of range (notebook has {} cells)",
                        cells.len()
                    )));
                }
                cells[idx]["source"] = json!(source_lines);
                if cell_type == "markdown" || cell_type == "code" {
                    cells[idx]["cell_type"] = json!(cell_type);
                }
            }
            "insert" => {
                let idx = cell_number.unwrap_or(cells.len());
                let new_cell = if cell_type == "markdown" {
                    json!({
                        "cell_type": "markdown",
                        "metadata": {},
                        "source": source_lines
                    })
                } else {
                    json!({
                        "cell_type": "code",
                        "execution_count": null,
                        "metadata": {},
                        "outputs": [],
                        "source": source_lines
                    })
                };
                let idx = idx.min(cells.len());
                cells.insert(idx, new_cell);
            }
            "delete" => {
                let idx = cell_number.unwrap_or(0);
                if idx >= cells.len() {
                    return Ok(ToolResult::error(format!(
                        "Cell index {idx} out of range (notebook has {} cells)",
                        cells.len()
                    )));
                }
                cells.remove(idx);
            }
            _ => {
                return Ok(ToolResult::error(format!(
                    "Invalid edit_mode '{edit_mode}'. Must be 'replace', 'insert', or 'delete'."
                )));
            }
        }

        // Write back
        let output = serde_json::to_string_pretty(&notebook)?;
        tokio::fs::write(&notebook_path, output).await?;

        Ok(ToolResult::success(format!(
            "Notebook {}: {} cell at index {} ({} total cells)",
            notebook_path.display(),
            match edit_mode {
                "replace" => "replaced",
                "insert" => "inserted",
                "delete" => "deleted",
                _ => edit_mode,
            },
            cell_number.unwrap_or(0),
            notebook["cells"].as_array().map_or(0, |c| c.len())
        )))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_source_line_splitting() {
        let source = "import pandas as pd\ndf = pd.read_csv('data.csv')\nprint(df.head())";
        let total = source.lines().count();
        let lines: Vec<String> = source
            .lines()
            .enumerate()
            .map(|(i, line)| {
                if i < total - 1 {
                    format!("{line}\n")
                } else {
                    line.to_string()
                }
            })
            .collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "import pandas as pd\n");
        assert_eq!(lines[2], "print(df.head())");
    }
}
