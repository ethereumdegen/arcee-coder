use crate::tools::{PermissionClass, Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// Manages LSP server processes for different languages.
pub struct LspManager {
    servers: HashMap<String, LspServer>,
}

struct LspServer {
    #[allow(dead_code)]
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
    initialized: bool,
    open_documents: HashSet<String>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    /// Get or start an LSP server for the given file extension.
    /// Returns None if no server is available for this extension.
    /// Removes dead servers and retries once.
    async fn get_server(
        &mut self,
        extension: &str,
        cwd: &Path,
    ) -> Result<Option<&mut LspServer>> {
        // Check if existing server is still alive
        let needs_restart = if let Some(server) = self.servers.get_mut(extension) {
            if server.is_alive() {
                false
            } else {
                // Server died — will restart silently below
                true
            }
        } else {
            true
        };

        if !needs_restart {
            return Ok(self.servers.get_mut(extension));
        }

        // Remove dead server if present
        self.servers.remove(extension);

        let (cmd, args) = match extension {
            "rs" => ("rust-analyzer", vec![]),
            "ts" | "tsx" | "js" | "jsx" => {
                ("typescript-language-server", vec!["--stdio".to_string()])
            }
            "py" => ("pylsp", vec![]),
            "go" => ("gopls", vec!["serve".to_string()]),
            _ => return Ok(None),
        };

        // Check if the command exists
        let which = tokio::process::Command::new("which")
            .arg(cmd)
            .output()
            .await;

        if which.is_err() || !which.unwrap().status.success() {
            return Ok(None);
        }

        let mut child = Command::new(cmd)
            .args(&args)
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        let mut server = LspServer {
            process: child,
            stdin,
            stdout,
            next_id: 1,
            initialized: false,
            open_documents: HashSet::new(),
        };

        // Send initialize request
        let root_uri = path_to_uri(cwd);
        let init_result = server
            .request(
                "initialize",
                json!({
                    "processId": std::process::id(),
                    "rootUri": root_uri,
                    "capabilities": {},
                }),
            )
            .await;

        if init_result.is_ok() {
            // Send initialized notification
            server.notify("initialized", json!({})).await?;
            server.initialized = true;
        }

        self.servers.insert(extension.to_string(), server);
        Ok(self.servers.get_mut(extension))
    }
}

impl LspServer {
    /// Check if the server process is still running.
    fn is_alive(&mut self) -> bool {
        // try_wait returns Ok(Some(status)) if exited, Ok(None) if still running
        match self.process.try_wait() {
            Ok(None) => true,     // still running
            Ok(Some(_)) => false, // exited
            Err(_) => false,      // error checking → assume dead
        }
    }

    /// Open a document if not already open, or send didChange if already open.
    async fn ensure_document_open(
        &mut self,
        uri: &str,
        language_id: &str,
        content: &str,
    ) -> Result<()> {
        if self.open_documents.contains(uri) {
            // Already open — no need to re-send didOpen
            return Ok(());
        }

        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": content,
                }
            }),
        )
        .await?;

        self.open_documents.insert(uri.to_string());
        Ok(())
    }

    async fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        self.send_message(&message).await?;
        self.read_response(id).await
    }

    async fn notify(&mut self, method: &str, params: serde_json::Value) -> Result<()> {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        self.send_message(&message).await
    }

    async fn send_message(&mut self, message: &serde_json::Value) -> Result<()> {
        let body = serde_json::to_string(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(body.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    async fn read_response(&mut self, expected_id: i64) -> Result<serde_json::Value> {
        // Read with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.read_response_inner(expected_id),
        )
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => anyhow::bail!("LSP response timeout after 30s"),
        }
    }

    async fn read_response_inner(&mut self, expected_id: i64) -> Result<serde_json::Value> {
        loop {
            // Read headers
            let mut content_length: usize = 0;
            loop {
                let mut line = String::new();
                let bytes_read = self.stdout.read_line(&mut line).await?;
                if bytes_read == 0 {
                    anyhow::bail!("LSP server closed connection (EOF)");
                }
                let line = line.trim();
                if line.is_empty() {
                    break;
                }
                if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                    content_length = len_str.parse()?;
                }
            }

            if content_length == 0 {
                anyhow::bail!("Invalid LSP message: missing or zero Content-Length header");
            }

            // Read body
            let mut body = vec![0u8; content_length];
            self.stdout.read_exact(&mut body).await?;
            let response: serde_json::Value = serde_json::from_slice(&body)?;

            // Skip notifications (no id field)
            if let Some(id) = response.get("id") {
                if id.as_i64() == Some(expected_id) {
                    if let Some(error) = response.get("error") {
                        let msg = error["message"]
                            .as_str()
                            .unwrap_or("Unknown LSP error");
                        anyhow::bail!("LSP error: {msg}");
                    }
                    return Ok(response["result"].clone());
                }
            }
            // Not our response, keep reading
        }
    }
}

impl Drop for LspManager {
    fn drop(&mut self) {
        // Kill all server processes
        for (_, mut server) in self.servers.drain() {
            let _ = server.process.start_kill();
        }
    }
}

/// Convert a filesystem path to a proper file:// URI with percent-encoding.
fn path_to_uri(path: &Path) -> String {
    let abs = if path.is_absolute() {
        path.to_string_lossy().to_string()
    } else {
        // This shouldn't happen in practice, but handle it
        format!("/{}", path.display())
    };

    // Percent-encode special characters (space, #, ?, %, etc.)
    let encoded: String = abs
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '#' => "%23".to_string(),
            '?' => "%3F".to_string(),
            '%' => "%25".to_string(),
            // Keep path separators and common safe chars
            '/' | ':' | '-' | '_' | '.' | '~' => c.to_string(),
            c if c.is_ascii_alphanumeric() => c.to_string(),
            c => format!("%{:02X}", c as u32),
        })
        .collect();

    format!("file://{encoded}")
}

pub struct LspTool;

const LSP_DESCRIPTION: &str = "Interact with Language Server Protocol (LSP) servers for code intelligence.\n\n\
REQUIRED: \"operation\", \"filePath\", \"line\" (1-based), \"character\" (1-based).\n\n\
Operations: goToDefinition, findReferences, hover, documentSymbol, workspaceSymbol, \
goToImplementation, prepareCallHierarchy, incomingCalls, outgoingCalls.\n\n\
Supported languages: Rust (.rs), TypeScript/JavaScript (.ts/.tsx/.js/.jsx), Python (.py), Go (.go).";

fn lsp_schema() -> &'static serde_json::Value {
    static CELL: OnceLock<serde_json::Value> = OnceLock::new();
    CELL.get_or_init(|| {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "LSP operation",
                    "enum": [
                        "goToDefinition",
                        "findReferences",
                        "hover",
                        "documentSymbol",
                        "workspaceSymbol",
                        "goToImplementation",
                        "prepareCallHierarchy",
                        "incomingCalls",
                        "outgoingCalls"
                    ]
                },
                "filePath": {
                    "type": "string",
                    "description": "Path to the file"
                },
                "line": {
                    "type": "number",
                    "description": "1-based line number"
                },
                "character": {
                    "type": "number",
                    "description": "1-based character offset"
                }
            },
            "required": ["operation", "filePath", "line", "character"]
        })
    })
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &'static str {
        "LSP"
    }

    fn description(&self) -> &'static str {
        LSP_DESCRIPTION
    }

    fn input_schema(&self) -> &'static serde_json::Value {
        lsp_schema()
    }

    fn permission(&self, _input: &serde_json::Value) -> PermissionClass {
        PermissionClass::ReadOnly
    }

    async fn call(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolOutput> {
        let operation = input["operation"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'operation' parameter"))?;
        let file_path_str = input["filePath"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'filePath' parameter"))?;
        let line = input["line"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing 'line' parameter"))? as u32;
        let character = input["character"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing 'character' parameter"))? as u32;

        // Resolve file path
        let file_path = if PathBuf::from(file_path_str).is_absolute() {
            PathBuf::from(file_path_str)
        } else {
            context.cwd.join(file_path_str)
        };

        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mut lsp_manager = context.lsp_manager.lock().await;
        let server = match lsp_manager.get_server(extension, &context.cwd).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                return Ok(ToolOutput::error(format!(
                    "No LSP server available for .{extension} files. \
                     Supported: .rs (rust-analyzer), .ts/.js (typescript-language-server), \
                     .py (pylsp), .go (gopls)"
                )));
            }
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to start LSP server for .{extension}: {e}"
                )));
            }
        };

        let file_uri = path_to_uri(&file_path);
        // LSP uses 0-based positions
        let lsp_line = line.saturating_sub(1);
        let lsp_char = character.saturating_sub(1);

        let position = json!({ "line": lsp_line, "character": lsp_char });
        let text_document = json!({ "uri": file_uri });
        let text_document_position = json!({
            "textDocument": text_document,
            "position": position,
        });

        // Ensure the document is open (only sends didOpen once per document)
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .unwrap_or_default();
        if let Err(e) = server
            .ensure_document_open(&file_uri, extension_to_language_id(extension), &content)
            .await
        {
            return Ok(ToolOutput::error(format!(
                "Failed to open document in LSP server: {e}"
            )));
        }

        let result = match operation {
            "goToDefinition" => {
                server
                    .request("textDocument/definition", text_document_position)
                    .await
            }
            "findReferences" => {
                server
                    .request(
                        "textDocument/references",
                        json!({
                            "textDocument": text_document,
                            "position": position,
                            "context": { "includeDeclaration": true },
                        }),
                    )
                    .await
            }
            "hover" => {
                server
                    .request("textDocument/hover", text_document_position)
                    .await
            }
            "documentSymbol" => {
                server
                    .request(
                        "textDocument/documentSymbol",
                        json!({ "textDocument": text_document }),
                    )
                    .await
            }
            "workspaceSymbol" => {
                server
                    .request("workspace/symbol", json!({ "query": "" }))
                    .await
            }
            "goToImplementation" => {
                server
                    .request("textDocument/implementation", text_document_position)
                    .await
            }
            "prepareCallHierarchy" => {
                server
                    .request(
                        "textDocument/prepareCallHierarchy",
                        text_document_position,
                    )
                    .await
            }
            "incomingCalls" => {
                let items = server
                    .request(
                        "textDocument/prepareCallHierarchy",
                        text_document_position.clone(),
                    )
                    .await?;
                if let Some(item) = items.as_array().and_then(|a| a.first()) {
                    server
                        .request(
                            "callHierarchy/incomingCalls",
                            json!({ "item": item }),
                        )
                        .await
                } else {
                    Ok(json!(null))
                }
            }
            "outgoingCalls" => {
                let items = server
                    .request(
                        "textDocument/prepareCallHierarchy",
                        text_document_position.clone(),
                    )
                    .await?;
                if let Some(item) = items.as_array().and_then(|a| a.first()) {
                    server
                        .request(
                            "callHierarchy/outgoingCalls",
                            json!({ "item": item }),
                        )
                        .await
                } else {
                    Ok(json!(null))
                }
            }
            _ => {
                return Ok(ToolOutput::error(format!(
                    "Unknown LSP operation '{operation}'. Supported: goToDefinition, \
                     findReferences, hover, documentSymbol, workspaceSymbol, \
                     goToImplementation, prepareCallHierarchy, incomingCalls, outgoingCalls"
                )));
            }
        };

        match result {
            Ok(value) => {
                let formatted = format_lsp_result(operation, &value);
                Ok(ToolOutput::success()
                    .with_summary(format!("LSP {operation}"))
                    .with_text(formatted))
            }
            Err(e) => Ok(ToolOutput::error(format!("LSP request failed: {e}"))),
        }
    }
}

fn extension_to_language_id(ext: &str) -> &str {
    match ext {
        "rs" => "rust",
        "ts" => "typescript",
        "tsx" => "typescriptreact",
        "js" => "javascript",
        "jsx" => "javascriptreact",
        "py" => "python",
        "go" => "go",
        _ => ext,
    }
}

fn format_lsp_result(operation: &str, result: &serde_json::Value) -> String {
    if result.is_null() {
        return format!("{operation}: No results found.");
    }

    match operation {
        "hover" => {
            if let Some(contents) = result.get("contents") {
                if let Some(value) = contents.get("value").and_then(|v| v.as_str()) {
                    return format!("Hover info:\n{value}");
                }
                if let Some(s) = contents.as_str() {
                    return format!("Hover info:\n{s}");
                }
            }
            format!("Hover:\n{}", serde_json::to_string_pretty(result).unwrap_or_default())
        }
        "goToDefinition" | "goToImplementation" => format_locations(result),
        "findReferences" => format_locations(result),
        "documentSymbol" => {
            format!(
                "Document symbols:\n{}",
                serde_json::to_string_pretty(result).unwrap_or_default()
            )
        }
        _ => serde_json::to_string_pretty(result).unwrap_or_else(|_| format!("{result:?}")),
    }
}

fn format_locations(result: &serde_json::Value) -> String {
    let locations = if result.is_array() {
        result.as_array().cloned().unwrap_or_default()
    } else {
        vec![result.clone()]
    };

    if locations.is_empty() {
        return "No locations found.".to_string();
    }

    let mut output = String::new();
    for loc in &locations {
        let uri = loc["uri"]
            .as_str()
            .or_else(|| loc["targetUri"].as_str())
            .unwrap_or("unknown");
        let range = loc
            .get("range")
            .or_else(|| loc.get("targetRange"))
            .cloned()
            .unwrap_or(json!({}));
        let start_line = range["start"]["line"].as_u64().unwrap_or(0) + 1;
        let start_char = range["start"]["character"].as_u64().unwrap_or(0) + 1;

        let path = uri.strip_prefix("file://").unwrap_or(uri);
        output.push_str(&format!("{path}:{start_line}:{start_char}\n"));
    }

    output
}
