use crate::api::errors::ApiError;
use crate::api::retry::{with_retry, RetryConfig};
use crate::api::streaming::{SseStream, StreamAccumulator};
use crate::api::types::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
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
        on_text: &mut dyn FnMut(&str),
        on_tool_use_start: &mut dyn FnMut(&str, &str),
    ) -> Result<(Vec<ContentBlock>, StopReason, Usage), ApiError> {
        self.send_message_with_model(
            &self.model,
            system_prompt,
            messages,
            tools,
            max_tokens,
            on_text,
            on_tool_use_start,
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
        on_text: &mut dyn FnMut(&str),
        on_tool_use_start: &mut dyn FnMut(&str, &str),
    ) -> Result<(Vec<ContentBlock>, StopReason, Usage), ApiError> {
        let retry_config = RetryConfig::default();

        // Prepend system message
        let mut all_messages = vec![ChatMessage::system(system_prompt)];
        all_messages.extend(messages);

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: all_messages,
            max_tokens: Some(max_tokens),
            temperature: None,
            stream: Some(true),
            tools,
            tool_choice: None,
        };

        let mut stream = with_retry(&retry_config, || {
            let req = request.clone();
            async move { self.stream_chat(req).await }
        })
        .await?;

        let mut accum = StreamAccumulator::new();
        let mut notified_tool_calls: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        while let Some(chunk_result) = stream.next().await {
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

        let stop_reason = accum
            .finish_reason
            .as_deref()
            .map(StopReason::from_api)
            .unwrap_or(StopReason::EndTurn);

        let usage = accum.usage.clone();
        let content = accum.into_content_blocks();

        Ok((content, stop_reason, usage))
    }
}
