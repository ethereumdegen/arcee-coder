//! Mock provider for tests. Returns a scripted list of responses without
//! performing any network I/O.

use super::{Provider, ProviderEvent, ProviderRequest, ProviderResponse};
use crate::api::types::{ContentBlock, ModelInfo, StopReason, Usage};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// In-memory provider that pops one scripted response per call.
#[derive(Default)]
pub struct MockProvider {
    scripted: Mutex<Vec<ProviderResponse>>,
}

impl MockProvider {
    pub fn new(scripted: Vec<ProviderResponse>) -> Self {
        Self {
            scripted: Mutex::new(scripted),
        }
    }

    pub fn text_once(text: impl Into<String>) -> Self {
        Self::new(vec![ProviderResponse {
            content: vec![ContentBlock::Text { text: text.into() }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        }])
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn default_model(&self) -> &str {
        "mock-model"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![])
    }

    async fn stream_message(
        &self,
        _req: ProviderRequest,
        on_event: &mut (dyn FnMut(ProviderEvent) + Send),
        _cancel: Option<Arc<AtomicBool>>,
    ) -> Result<ProviderResponse> {
        let mut scripts = self.scripted.lock().unwrap();
        let response = scripts
            .pop()
            .unwrap_or_else(|| ProviderResponse {
                content: vec![ContentBlock::Text {
                    text: String::new(),
                }],
                stop_reason: StopReason::EndTurn,
                usage: Usage::default(),
            });

        // Emit any text content as a delta event for realism.
        for block in &response.content {
            if let ContentBlock::Text { text } = block {
                on_event(ProviderEvent::TextDelta(text.clone()));
            }
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_round_trip() {
        let provider = MockProvider::text_once("hello");
        let req = ProviderRequest {
            model: "mock".into(),
            system: String::new(),
            messages: vec![],
            tools: vec![],
            max_tokens: 16,
            thinking_budget: None,
        };
        let mut collected = String::new();
        let mut cb = |e: ProviderEvent| {
            if let ProviderEvent::TextDelta(t) = e {
                collected.push_str(&t);
            }
        };
        let response = provider.stream_message(req, &mut cb, None).await.unwrap();
        assert_eq!(collected, "hello");
        assert_eq!(response.stop_reason, StopReason::EndTurn);
    }
}
