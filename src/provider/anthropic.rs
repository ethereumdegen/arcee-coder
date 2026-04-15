//! Anthropic provider stub.
//!
//! TODO(phase: provider-expansion):
//! - POST to `https://api.anthropic.com/v1/messages` with `stream: true`
//! - Translate `ProviderRequest` → Anthropic Messages schema:
//!     * `system` → top-level `system`
//!     * `messages` role mapping (`assistant` / `user`)
//!     * tool_use / tool_result content blocks
//! - Parse SSE event stream into `ProviderEvent`s:
//!     * `content_block_delta` text → TextDelta
//!     * `content_block_start` with `tool_use` → ToolUseStart
//!     * `content_block_delta` partial_json → ToolUseArgsDelta
//! - Honor the `cancel` flag by dropping the response stream.
//! - Extract usage + stop_reason from `message_delta` / `message_stop`.

use super::{Provider, ProviderEvent, ProviderRequest, ProviderResponse};
use crate::api::types::ModelInfo;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[allow(dead_code)]
pub struct AnthropicProvider {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn default_model(&self) -> &str {
        &self.model
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        todo!("anthropic: list_models not yet implemented")
    }

    async fn stream_message(
        &self,
        _req: ProviderRequest,
        _on_event: &mut (dyn FnMut(ProviderEvent) + Send),
        _cancel: Option<Arc<AtomicBool>>,
    ) -> Result<ProviderResponse> {
        todo!("anthropic: stream_message not yet implemented")
    }
}
