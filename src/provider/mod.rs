//! LLM provider abstraction.
//!
//! The [`Provider`] trait isolates the engine from any specific vendor API
//! so backends can be swapped without touching `engine::query_loop`. The
//! existing `api::client::ApiClient` — which speaks Arcee's
//! OpenAI-compatible endpoint — lives behind [`arcee::ArceeProvider`]. A
//! [`mock::MockProvider`] is included for engine unit tests.
//!
//! Implementations for Anthropic, OpenAI, and Ollama are sketched as stubs
//! that `todo!()` their transport; they document the intended shape without
//! committing this round to those network details.

pub mod anthropic;
pub mod arcee;
pub mod mock;
pub mod ollama;
pub mod openai;

use crate::api::types::{ChatMessage, ContentBlock, ModelInfo, StopReason, ToolDefinition, Usage};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Provider-level request shape. Wire-format translation happens inside
/// each provider implementation.
#[derive(Debug, Clone)]
pub struct ProviderRequest {
    pub model: String,
    pub system: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolDefinition>,
    pub max_tokens: u32,
    pub thinking_budget: Option<u32>,
}

/// Streaming event emitted by a provider during `stream_message`.
#[derive(Debug, Clone)]
pub enum ProviderEvent {
    /// A chunk of assistant text.
    TextDelta(String),
    /// A chunk of thinking/reasoning text (if the model exposes it).
    ThinkingDelta(String),
    /// A tool call began streaming.
    ToolUseStart { id: String, name: String },
    /// A fragment of the JSON arguments for a tool call.
    #[allow(dead_code)]
    ToolUseArgsDelta { id: String, delta: String },
}

/// Final response returned from a `stream_message` call.
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: Usage,
}

/// Core provider trait. Implementations are fully async and support
/// streaming with per-chunk callbacks plus an external cancellation flag.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Human-readable provider name (e.g. `"arcee"`).
    fn name(&self) -> &str;

    /// Default model name for this provider.
    fn default_model(&self) -> &str;

    /// List available models (and pricing, if exposed).
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    /// Stream a chat completion. The `on_event` callback is invoked for
    /// every incremental chunk. Providers must terminate the stream
    /// promptly when `cancel` flips to true.
    async fn stream_message(
        &self,
        req: ProviderRequest,
        on_event: &mut (dyn FnMut(ProviderEvent) + Send),
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<ProviderResponse>;
}
