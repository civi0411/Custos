//! Provider Port Trait
//!
//! Contract defining how the Custos runtime interacts with AI model providers.

use crate::events::ProviderEvent;
use crate::request::{ModelResponse, ProviderRequest};
use async_trait::async_trait;
use custos_core_domain::DomainError;
use tokio::sync::mpsc;

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
            let _ = tx
                .send(ProviderEvent::completed(
                    req_id,
                    Some(resp.content),
                    Some(crate::events::TokenUsage {
                        input_tokens: None,
                        output_tokens: Some(resp.tokens_used),
                    }),
                ))
                .await;
        });
        Ok(rx)
    }
}
