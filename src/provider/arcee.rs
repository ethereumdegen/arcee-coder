//! Arcee provider — adapter over the existing `api::client::ApiClient`.
//!
//! The transport layer (`src/api/client.rs`, `streaming.rs`, `types.rs`) is
//! unchanged; this module merely wraps it in the `Provider` trait so the
//! engine no longer depends on `ApiClient` directly.

use super::{Provider, ProviderEvent, ProviderRequest, ProviderResponse};
use crate::api::client::ApiClient;
use crate::api::types::ModelInfo;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// `Provider` implementation backed by the existing Arcee OpenAI-compatible
/// transport.
pub struct ArceeProvider {
    client: Arc<ApiClient>,
}

impl ArceeProvider {
    pub fn new(client: Arc<ApiClient>) -> Self {
        Self { client }
    }

    /// Expose the underlying client for the few code paths that still need it
    /// (e.g. compaction which uses a dedicated helper).
    pub fn inner(&self) -> &ApiClient {
        &self.client
    }

    pub fn client(&self) -> Arc<ApiClient> {
        self.client.clone()
    }
}

#[async_trait]
impl Provider for ArceeProvider {
    fn name(&self) -> &str {
        "arcee"
    }

    fn default_model(&self) -> &str {
        &self.client.model
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.client
            .fetch_models()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn stream_message(
        &self,
        req: ProviderRequest,
        on_event: &mut (dyn FnMut(ProviderEvent) + Send),
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<ProviderResponse> {
        // The underlying ApiClient takes two distinct callbacks (text + tool).
        // We route both into the single `on_event` via a Mutex so the two
        // closures can share exclusive access across await boundaries.
        use std::sync::Mutex;
        let on_event_lock: Mutex<&mut (dyn FnMut(ProviderEvent) + Send)> = Mutex::new(on_event);

        let mut on_text_cb = |text: &str| {
            if let Ok(mut cb) = on_event_lock.lock() {
                (cb)(ProviderEvent::TextDelta(text.to_string()));
            }
        };
        let mut on_tool_cb = |id: &str, name: &str| {
            if let Ok(mut cb) = on_event_lock.lock() {
                (cb)(ProviderEvent::ToolUseStart {
                    id: id.to_string(),
                    name: name.to_string(),
                });
            }
        };

        let escape_ref = cancel.as_ref();

        let result = self
            .client
            .send_message_with_model(
                &req.model,
                &req.system,
                req.messages,
                req.tools,
                req.max_tokens,
                &mut on_text_cb,
                &mut on_tool_cb,
                escape_ref,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(ProviderResponse {
            content: result.0,
            stop_reason: result.1,
            usage: result.2,
        })
    }
}
