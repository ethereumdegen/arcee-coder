//! Ollama provider stub.
//!
//! TODO(phase: provider-expansion):
//! - POST to `http://localhost:11434/api/chat` with `stream: true`
//! - NDJSON stream: each line is a `{message: {role, content}, done: bool}`
//!   plus `done_reason`. Emit TextDelta on each chunk.
//! - Tool-calling support varies by model: fall back to no tools when the
//!   chosen model does not declare the `tools` capability in `/api/show`.
//! - No auth header; typical cancellation is abort-on-drop of the stream.

use super::{Provider, ProviderEvent, ProviderRequest, ProviderResponse};
use crate::api::types::ModelInfo;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[allow(dead_code)]
pub struct OllamaProvider {
    pub base_url: String,
    pub model: String,
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn default_model(&self) -> &str {
        &self.model
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        todo!("ollama: list_models not yet implemented")
    }

    async fn stream_message(
        &self,
        _req: ProviderRequest,
        _on_event: &mut (dyn FnMut(ProviderEvent) + Send),
        _cancel: Option<Arc<AtomicBool>>,
    ) -> Result<ProviderResponse> {
        todo!("ollama: stream_message not yet implemented")
    }
}
