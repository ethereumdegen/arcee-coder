use crate::api::errors::ApiError;
use crate::api::retry::{with_retry, RetryConfig};
use crate::api::streaming::{SseStream, StreamAccumulator};
use crate::api::types::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_stream::StreamExt;

pub struct ApiClient {
    http: reqwest::Client,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl ApiClient {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http,
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.arcee.ai".to_string()),
            model: model.unwrap_or_else(|| "trinity-large-thinking".to_string()),
        }
    }

    fn headers(&self) -> Result<HeaderMap, ApiError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key)).map_err(|_| {
                ApiError::Auth(
                    "API key contains invalid characters. Keys must be ASCII-only.".to_string(),
                )
            })?,
        );
        Ok(headers)
    }

    /// Fetch the list of available models (and their pricing) from the API.
    pub async fn fetch_models(&self) -> Result<Vec<ModelInfo>, ApiError> {
        let url = format!("{}/api/v1/models", self.base_url);
        let headers = self.headers()?;

        let response = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Server(format!(
                "Failed to fetch models (HTTP {status}): {body}"
            )));
        }

        let body: ModelsListResponse = response.json().await.map_err(|e| {
            ApiError::Server(format!("Failed to parse models response: {e}"))
        })?;

        Ok(body.data)
    }

    /// Send a streaming chat completion request, returning the raw SSE stream.
    pub async fn stream_chat(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<SseStream, ApiError> {
        let url = format!("{}/api/v1/chat/completions", self.base_url);
        let headers = self.headers()?;

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();

            return match status.as_u16() {
                401 | 403 => Err(ApiError::Auth(format!(
                    "Authentication failed (HTTP {status}). Check your ARCEE_API_KEY.\n{body}"
                ))),
                402 => Err(ApiError::Auth(
                    "Insufficient balance. Top up your Arcee account at https://app.arcee.ai/"
                        .to_string(),
                )),
                429 => {
                    let retry_after = serde_json::from_str::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|v| v["retry_after"].as_u64());
                    Err(ApiError::RateLimit {
                        retry_after_secs: retry_after,
                    })
                }
                422 => Err(ApiError::ApiResponse {
                    error_type: "invalid_parameters".to_string(),
                    message: body,
                }),
                500..=599 => Err(ApiError::Server(format!("HTTP {status}: {body}"))),
                _ => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(err) = parsed.get("error") {
                            let msg = err
                                .as_str()
                                .map(String::from)
                                .or_else(|| err["message"].as_str().map(String::from))
                                .unwrap_or_else(|| body.clone());
                            return Err(ApiError::ApiResponse {
                                error_type: "api_error".to_string(),
                                message: msg,
                            });
                        }
                    }
                    Err(ApiError::Server(format!("HTTP {status}: {body}")))
                }
            };
        }

        Ok(SseStream::new(response.bytes_stream()))
    }

    /// Send a streaming request with retry logic, returning the accumulated response.
    /// `model_override` allows per-call model selection (for adaptive routing).
    pub async fn send_message(
        &self,
        system_prompt: &str,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        max_tokens: u32,
        on_text: &mut (dyn FnMut(&str) + Send),
        on_tool_use_start: &mut (dyn FnMut(&str, &str) + Send),
        escape_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<(Vec<ContentBlock>, StopReason, Usage), ApiError> {
        self.send_message_with_model(
            &self.model,
            system_prompt,
            messages,
            tools,
            max_tokens,
            on_text,
            on_tool_use_start,
            escape_flag,
        )
        .await
    }

    /// Send with an explicit model (used by the model router).
    pub async fn send_message_with_model(
        &self,
        model: &str,
        system_prompt: &str,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        max_tokens: u32,
        on_text: &mut (dyn FnMut(&str) + Send),
        on_tool_use_start: &mut (dyn FnMut(&str, &str) + Send),
        escape_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<(Vec<ContentBlock>, StopReason, Usage), ApiError> {
        let retry_config = RetryConfig::default();

        // Prepend system message
        let mut all_messages = vec![ChatMessage::system(system_prompt)];
        all_messages.extend(messages);

        let tool_choice = if tools.is_empty() {
            None
        } else {
            Some(serde_json::json!("auto"))
        };

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: all_messages,
            max_tokens: Some(max_tokens),
            temperature: None,
            stream: Some(true),
            tools,
            tool_choice,
        };

        // Debug: log request summary when ARCEE_DEBUG=1
        if std::env::var("ARCEE_DEBUG").is_ok() {
            eprintln!("\x1b[90m[DEBUG] API request: model={}, messages={}, tools={}\x1b[0m",
                request.model,
                request.messages.len(),
                request.tools.len(),
            );
            for tool in &request.tools {
                eprintln!("\x1b[90m[DEBUG]   tool: {} (params: {})\x1b[0m",
                    tool.function.name,
                    serde_json::to_string(&tool.function.parameters)
                        .unwrap_or_default()
                        .chars()
                        .take(200)
                        .collect::<String>()
                );
            }
        }

        let mut stream = with_retry(&retry_config, || {
            let req = request.clone();
            async move { self.stream_chat(req).await }
        })
        .await?;

        let mut accum = StreamAccumulator::new();
        let mut notified_tool_calls: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        while let Some(chunk_result) = stream.next().await {
            // Check escape flag during streaming — break immediately so the
            // stream is dropped (cancelling the HTTP connection, like AbortController.abort())
            if let Some(flag) = escape_flag {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
            }

            let chunk = chunk_result?;

            // Emit streaming callbacks before accumulating
            for choice in &chunk.choices {
                if let Some(ref delta) = choice.delta {
                    if let Some(ref content) = delta.content {
                        on_text(content);
                    }
                    if let Some(ref tool_calls) = delta.tool_calls {
                        for tc in tool_calls {
                            if !notified_tool_calls.contains(&tc.index) {
                                if let Some(ref func) = tc.function {
                                    if let Some(ref name) = func.name {
                                        let id = tc.id.as_deref().unwrap_or("");
                                        on_tool_use_start(id, name);
                                        notified_tool_calls.insert(tc.index);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            accum.process_chunk(&chunk);
        }

        // Debug: log accumulated tool calls before converting
        if std::env::var("ARCEE_DEBUG").is_ok() {
            for (i, tc) in accum.tool_calls.iter().enumerate() {
                eprintln!(
                    "\x1b[90m[DEBUG] tool_call[{i}]: name={:?}, id={:?}, args_len={}, args={:?}\x1b[0m",
                    tc.name,
                    tc.id,
                    tc.arguments.len(),
                    &tc.arguments[..tc.arguments.len().min(300)]
                );
            }
            eprintln!(
                "\x1b[90m[DEBUG] finish_reason={:?}, text_len={}\x1b[0m",
                accum.finish_reason,
                accum.text.len()
            );
        }

        let usage = accum.usage.clone();
        let finish_reason = accum.finish_reason.clone();
        let content = accum.into_content_blocks();

        // Determine stop reason: if there are tool calls in content, treat as ToolUse
        // regardless of what finish_reason says (some APIs return "stop" with tool calls).
        // If finish_reason is missing (stream disconnect), check content to decide.
        let has_tool_calls = content.iter().any(|b| matches!(b, ContentBlock::ToolUse { .. }));
        let stop_reason = if has_tool_calls {
            StopReason::ToolUse
        } else if let Some(ref reason) = finish_reason {
            StopReason::from_api(reason)
        } else {
            // No finish_reason received — stream may have ended prematurely
            eprintln!(
                "\x1b[33m[warning: stream ended without finish_reason]\x1b[0m"
            );
            StopReason::EndTurn
        };

        Ok((content, stop_reason, usage))
    }
}
