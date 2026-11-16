//! Provider Port Trait
//!
//! Contract defining how the Custos runtime interacts with AI model providers.

use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::identity::error::DomainError;
use crate::inference::events::ProviderEvent;
use crate::inference::request::{ModelResponse, ProviderRequest};

#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Unique provider identifier (e.g., "fake", "claude", "codex", "antigravity").
    fn provider_id(&self) -> &str;

    /// Unary synchronous generation call.
    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError>;

    /// Streaming generation returning an asynchronous receiver of ProviderEvents.
    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<mpsc::Receiver<ProviderEvent>, DomainError> {
        let (tx, rx) = mpsc::channel(4);
        let resp = self.generate(req).await?;
        let req_id = req.request_id.clone();
        tokio::spawn(async move {
            let mut event = ProviderEvent::completed(
                req_id,
                Some(resp.content),
                Some(crate::inference::events::TokenUsage {
                    input_tokens: resp.input_tokens,
                    output_tokens: resp.output_tokens,
                }),
            );
            event.model_id = Some(resp.model_id);
            event.tokens_used = Some(resp.tokens_used);
            let _ = tx.send(event).await;
        });
        Ok(rx)
    }
}

pub use ModelProvider as ModelPort;
