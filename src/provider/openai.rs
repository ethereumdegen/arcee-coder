//! OpenAI provider stub.
//!
//! TODO(phase: provider-expansion):
//! - POST to `https://api.openai.com/v1/chat/completions` with `stream: true`
//! - Map `ProviderRequest` directly — the Arcee client already speaks
//!   OpenAI-compatible wire format, so this is nearly a copy of
//!   `ArceeProvider` with a different base URL and auth header.
//! - Parse SSE `delta.content` → TextDelta,
//!   `delta.tool_calls` → ToolUseStart / ToolUseArgsDelta.
//! - Handle `finish_reason` → `StopReason`.

use super::{Provider, ProviderEvent, ProviderRequest, ProviderResponse};
use crate::api::types::ModelInfo;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[allow(dead_code)]
pub struct OpenAiProvider {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
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
        todo!("openai: list_models not yet implemented")
    }

    async fn stream_message(
        &self,
        _req: ProviderRequest,
        _on_event: &mut (dyn FnMut(ProviderEvent) + Send),
        _cancel: Option<Arc<AtomicBool>>,
    ) -> Result<ProviderResponse> {
        todo!("openai: stream_message not yet implemented")
    }
}
