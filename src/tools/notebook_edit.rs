use crate::tools::path_safety::resolve_and_validate;
use crate::tools::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::OnceLock;

pub struct NotebookEditTool;

const DESCRIPTION: &str = "Completely replaces the contents of a specific cell in a Jupyter notebook (.ipynb file) with new source. \
Jupyter notebooks are interactive documents that combine code, text, and visualizations, commonly used for data analysis and scientific computing. \
The notebook_path parameter must be an absolute path, not a relative path. The cell_number is 0-indexed. \
Use edit_mode=insert to add a new cell at the index specified by cell_number. Use edit_mode=delete to delete the cell at the index specified by cell_number.";

fn schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
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
                    "description": "Cell type: 'code' or 'markdown'. Required for insert.",
                    "enum": ["code", "markdown"]
                },
                "edit_mode": {
                    "type": "string",
                    "description": "The type of edit to make (replace, insert, delete). Defaults to replace.",
                    "enum": ["replace", "insert", "delete"]
                },
                "cell_id": {
                    "type": "string",
                    "description": "The ID of the cell to edit. When inserting a new cell, the new cell will be inserted after the cell with this ID."
                }
            },
            "required": ["notebook_path", "new_source"]
        })
    })
}

#[async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &'static str {
        "NotebookEdit"
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        schema()
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
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
            Err(e) => return Ok(ToolOutput::error(e)),
        };

        if !notebook_path.exists() && edit_mode != "insert" {
            return Ok(ToolOutput::error(format!(
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
                    return Ok(ToolOutput::error(format!(
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
                    return Ok(ToolOutput::error(format!(
                        "Cell index {idx} out of range (notebook has {} cells)",
                        cells.len()
                    )));
                }
                cells.remove(idx);
            }
            _ => {
                return Ok(ToolOutput::error(format!(
                    "Invalid edit_mode '{edit_mode}'. Must be 'replace', 'insert', or 'delete'."
                )));
            }
        }

        // Write back
        let output = serde_json::to_string_pretty(&notebook)?;
        tokio::fs::write(&notebook_path, output).await?;

        let verb = match edit_mode {
            "replace" => "replaced",
            "insert" => "inserted",
            "delete" => "deleted",
            _ => edit_mode,
        };
        let cell_count = notebook["cells"].as_array().map_or(0, |c| c.len());
        let idx = cell_number.unwrap_or(0);
        Ok(ToolOutput::success()
            .with_summary(format!(
                "{verb} cell {idx} ({cell_count} total cells)"
            ))
            .with_text(format!(
                "Notebook {}: {verb} cell at index {idx}",
                notebook_path.display()
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
