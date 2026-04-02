use crate::api::errors::ApiError;
use crate::api::types::*;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

const MAX_BUFFER_SIZE: usize = 10 * 1024 * 1024; // 10 MB

/// Parses Server-Sent Events from an OpenAI-compatible streaming response.
pub struct SseStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    buffer: String,
    /// Leftover bytes from a partial UTF-8 sequence at a chunk boundary.
    utf8_remainder: Vec<u8>,
    /// Parsed events ready to be yielded (buffered from a single network chunk).
    pending: std::collections::VecDeque<Result<ChatCompletionResponse, ApiError>>,
}

impl SseStream {
    pub fn new(byte_stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static) -> Self {
        Self {
            inner: Box::pin(byte_stream),
            buffer: String::new(),
            utf8_remainder: Vec::new(),
            pending: std::collections::VecDeque::new(),
        }
    }

    fn parse_chunks(&mut self) -> Vec<Result<ChatCompletionResponse, ApiError>> {
        let mut chunks = Vec::new();

        while let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].trim_end_matches('\r').to_string();
            self.buffer = self.buffer[newline_pos + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            let data = if let Some(value) = line.strip_prefix("data: ") {
                value
            } else if line.starts_with('{') {
                // Some APIs send raw JSON without "data: " prefix (e.g. error responses)
                line.as_str()
            } else {
                continue;
            };

            // [DONE] signals end of stream
            if data.trim() == "[DONE]" {
                continue;
            }

            // Check if the response is an API error before trying to parse as a completion
            if let Ok(error_resp) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(error_obj) = error_resp.get("error") {
                    let message = error_obj
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error");
                    let code = error_obj
                        .get("code")
                        .and_then(|c| c.as_u64())
                        .map(|c| c.to_string())
                        .or_else(|| error_obj.get("type").and_then(|t| t.as_str()).map(String::from))
                        .unwrap_or_else(|| "unknown".to_string());
                    chunks.push(Err(ApiError::ApiResponse {
                        error_type: code,
                        message: message.to_string(),
                    }));
                    continue;
                }
            }

            match serde_json::from_str::<ChatCompletionResponse>(data) {
                Ok(chunk) => chunks.push(Ok(chunk)),
                Err(e) => {
                    // Try to ignore non-fatal parse issues
                    if !data.trim().is_empty() {
                        chunks.push(Err(ApiError::StreamParse(format!(
                            "Failed to parse chunk: {e}\nData: {data}"
                        ))));
                    }
                }
            }
        }

        chunks
    }
}

impl Stream for SseStream {
    type Item = Result<ChatCompletionResponse, ApiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // Drain any already-parsed events first before reading more from the network.
            if let Some(chunk) = self.pending.pop_front() {
                return Poll::Ready(Some(chunk));
            }

            let chunks = self.parse_chunks();
            if !chunks.is_empty() {
                self.pending.extend(chunks);
                continue;
            }

            match self.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    // Handle partial UTF-8 at chunk boundaries
                    let mut combined = std::mem::take(&mut self.utf8_remainder);
                    combined.extend_from_slice(&bytes);

                    match std::str::from_utf8(&combined) {
                        Ok(text) => {
                            self.buffer.push_str(text);
                        }
                        Err(e) => {
                            // Valid up to error index, remainder may be partial char
                            let valid_up_to = e.valid_up_to();
                            if valid_up_to > 0 {
                                // Safety: from_utf8 told us this range is valid
                                let valid = unsafe {
                                    std::str::from_utf8_unchecked(&combined[..valid_up_to])
                                };
                                self.buffer.push_str(valid);
                            }
                            // Save the remainder for the next chunk
                            self.utf8_remainder = combined[valid_up_to..].to_vec();
                        }
                    }

                    // Guard against unbounded buffer growth
                    if self.buffer.len() > MAX_BUFFER_SIZE {
                        return Poll::Ready(Some(Err(ApiError::StreamParse(
                            "Stream buffer exceeded 10 MB without complete events".to_string(),
                        ))));
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(ApiError::Request(e))));
                }
                Poll::Ready(None) => {
                    if !self.buffer.trim().is_empty() {
                        self.buffer.push('\n');
                        let chunks = self.parse_chunks();
                        if !chunks.is_empty() {
                            self.pending.extend(chunks);
                            continue;
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Accumulates OpenAI-compatible streaming chunks into a complete response.
pub struct StreamAccumulator {
    pub text: String,
    pub tool_calls: Vec<ToolCallAccum>,
    pub reasoning: String,
    pub finish_reason: Option<String>,
    pub usage: Usage,
    pub model: String,
    pub message_id: String,
}

pub struct ToolCallAccum {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            tool_calls: Vec::new(),
            reasoning: String::new(),
            finish_reason: None,
            usage: Default::default(),
            model: String::new(),
            message_id: String::new(),
        }
    }

    pub fn process_chunk(&mut self, chunk: &ChatCompletionResponse) {
        if self.message_id.is_empty() {
            self.message_id = chunk.id.clone();
            self.model = chunk.model.clone();
        }

        if let Some(ref usage) = chunk.usage {
            self.usage = Usage::from(usage);
        }

        for choice in &chunk.choices {
            if let Some(ref reason) = choice.finish_reason {
                self.finish_reason = Some(reason.clone());
            }

            if let Some(ref delta) = choice.delta {
                // Accumulate text content
                if let Some(ref content) = delta.content {
                    self.text.push_str(content);
                }

                // Accumulate reasoning
                if let Some(ref reasoning) = delta.reasoning {
                    self.reasoning.push_str(reasoning);
                }

                // Accumulate tool calls
                if let Some(ref tool_call_deltas) = delta.tool_calls {
                    for tc_delta in tool_call_deltas {
                        // Ensure we have enough slots
                        while self.tool_calls.len() <= tc_delta.index {
                            self.tool_calls.push(ToolCallAccum {
                                id: String::new(),
                                name: String::new(),
                                arguments: String::new(),
                            });
                        }

                        let tc = &mut self.tool_calls[tc_delta.index];

                        if let Some(ref id) = tc_delta.id {
                            tc.id = id.clone();
                        }

                        if let Some(ref func) = tc_delta.function {
                            if let Some(ref name) = func.name {
                                tc.name = name.clone();
                            }
                            if let Some(ref args) = func.arguments {
                                tc.arguments.push_str(args);
                            }
                        }
                    }
                }
            }

            // Handle non-streaming response (message instead of delta)
            if let Some(ref message) = choice.message {
                if let Some(ref content) = message.content {
                    self.text.push_str(content);
                }
                if let Some(ref reasoning) = message.reasoning {
                    self.reasoning.push_str(reasoning);
                }
                if let Some(ref tool_calls) = message.tool_calls {
                    for tc in tool_calls {
                        self.tool_calls.push(ToolCallAccum {
                            id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        });
                    }
                }
            }
        }
    }

    pub fn into_content_blocks(self) -> Vec<ContentBlock> {
        let mut blocks = Vec::new();

        // Add reasoning as thinking block if present
        if !self.reasoning.is_empty() {
            blocks.push(ContentBlock::Thinking {
                thinking: self.reasoning,
            });
        }

        // Add text content
        if !self.text.is_empty() {
            blocks.push(ContentBlock::Text { text: self.text });
        }

        // Add tool calls
        for tc in self.tool_calls {
            let input = if tc.arguments.trim().is_empty() {
                // Empty arguments — use empty object; the engine's input
                // validation will report this to the model.
                serde_json::Value::Object(serde_json::Map::new())
            } else {
                match serde_json::from_str(&tc.arguments) {
                    Ok(v) => v,
                    Err(_e) => {
                        // Parse failure — use empty object; the engine's input
                        // validation will catch missing required fields.
                        tracing::warn!(
                            tool = %tc.name,
                            error = %_e,
                            raw = &tc.arguments[..tc.arguments.len().min(200)],
                            "tool arguments failed to parse"
                        );
                        serde_json::Value::Object(serde_json::Map::new())
                    }
                }
            };
            blocks.push(ContentBlock::ToolUse {
                id: tc.id,
                name: tc.name,
                input,
            });
        }

        blocks
    }
}
