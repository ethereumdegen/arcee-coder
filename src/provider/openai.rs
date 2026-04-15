//! OpenAI-compatible provider.
//!
//! Speaks the standard OpenAI `/v1/chat/completions` wire format (with SSE
//! streaming).  Works with OpenAI, Azure OpenAI, Groq, Together, DeepSeek,
//! and any other host that exposes a compatible endpoint — just override
//! `base_url`.
//!
//! Reuses `SseStream` and `StreamAccumulator` from `crate::api::streaming`
//! so there is zero duplication of the SSE parsing logic.

use super::{Provider, ProviderEvent, ProviderRequest, ProviderResponse};
use crate::api::errors::ApiError;
use crate::api::streaming::{SseStream, StreamAccumulator};
use crate::api::types::*;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_stream::StreamExt;

pub struct OpenAiProvider {
    http: reqwest::Client,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http,
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com".to_string()),
            model: model.unwrap_or_else(|| "gpt-4o".to_string()),
        }
    }

    fn headers(&self) -> Result<HeaderMap, ApiError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key)).map_err(|_| {
                ApiError::Auth("API key contains invalid characters.".to_string())
            })?,
        );
        Ok(headers)
    }

    async fn stream_chat(&self, request: ChatCompletionRequest) -> Result<SseStream, ApiError> {
        let url = format!("{}/v1/chat/completions", self.base_url);
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
                    "Authentication failed (HTTP {status}). Check your API key.\n{body}"
                ))),
                429 => {
                    let retry_after = serde_json::from_str::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|v| v["retry_after"].as_u64());
                    Err(ApiError::RateLimit {
                        retry_after_secs: retry_after,
                    })
                }
                500..=599 => Err(ApiError::Server(format!("HTTP {status}: {body}"))),
                _ => Err(ApiError::Server(format!("HTTP {status}: {body}"))),
            };
        }

        Ok(SseStream::new(response.bytes_stream()))
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        &self.model
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/v1/models", self.base_url);
        let headers = self.headers().map_err(|e| anyhow::anyhow!(e))?;

        let response = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch models: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to fetch models (HTTP {status}): {body}");
        }

        // The OpenAI /v1/models response has { data: [{ id, ... }] }
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse models response: {e}"))?;

        let models = body["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        m["id"].as_str().map(|id| ModelInfo {
                            id: id.to_string(),
                            pricing: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }

    async fn stream_message(
        &self,
        req: ProviderRequest,
        on_event: &mut (dyn FnMut(ProviderEvent) + Send),
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<ProviderResponse> {
        // Build the request — prepend system message
        let mut all_messages = vec![ChatMessage::system(&req.system)];
        all_messages.extend(req.messages);

        let tool_choice = if req.tools.is_empty() {
            None
        } else {
            Some(serde_json::json!("auto"))
        };

        let request = ChatCompletionRequest {
            model: req.model.clone(),
            messages: all_messages,
            max_tokens: Some(req.max_tokens),
            temperature: None,
            stream: Some(true),
            tools: req.tools,
            tool_choice,
        };

        let mut stream = self.stream_chat(request).await.map_err(|e| anyhow::anyhow!(e))?;

        let mut accum = StreamAccumulator::new();
        let mut notified_tool_calls: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        const CHUNK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

        while let Some(chunk_result) = tokio::time::timeout(CHUNK_TIMEOUT, stream.next())
            .await
            .map_err(|_| anyhow::anyhow!("chunk timeout: no data received for 60s"))?
        {
            if let Some(flag) = &cancel {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
            }

            let chunk = chunk_result.map_err(|e| anyhow::anyhow!(e))?;

            // Emit streaming callbacks
            for choice in &chunk.choices {
                if let Some(ref delta) = choice.delta {
                    if let Some(ref content) = delta.content {
                        on_event(ProviderEvent::TextDelta(content.clone()));
                    }
                    if let Some(ref tool_calls) = delta.tool_calls {
                        for tc in tool_calls {
                            if !notified_tool_calls.contains(&tc.index) {
                                if let Some(ref func) = tc.function {
                                    if let Some(ref name) = func.name {
                                        let id = tc.id.as_deref().unwrap_or("");
                                        on_event(ProviderEvent::ToolUseStart {
                                            id: id.to_string(),
                                            name: name.clone(),
                                        });
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

        let usage = accum.usage.clone();
        let finish_reason = accum.finish_reason.clone();
        let content = accum.into_content_blocks();

        let has_tool_calls = content.iter().any(|b| matches!(b, ContentBlock::ToolUse { .. }));
        let stop_reason = if has_tool_calls {
            StopReason::ToolUse
        } else if let Some(ref reason) = finish_reason {
            StopReason::from_api(reason)
        } else {
            StopReason::EndTurn
        };

        Ok(ProviderResponse {
            content,
            stop_reason,
            usage,
        })
    }
}
