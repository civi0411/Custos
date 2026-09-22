//! Fake provider adapter for testing
//!
//! This provider implements the ModelProvider from `custos-provider-sdk`
//! to provide deterministic, offline responses for testing workflows and CI.

use async_trait::async_trait;
use custos_core_domain::DomainError;
use custos_provider_sdk::{
    ModelProvider, ModelResponse, ProviderEvent, ProviderRequest, TokenUsage,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// A fake provider for testing, replay fixtures, and end-to-end simulation.
#[derive(Clone)]
pub struct FakeProvider {
    pub name: String,
    default_response: String,
    response_queue: Arc<Mutex<VecDeque<ModelResponse>>>,
}

impl FakeProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default_response: "Deterministic fake response".to_string(),
            response_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Configures a fixed default text response.
    pub fn with_default_text(name: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default_response: text.into(),
            response_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Queues a sequence of responses to be returned on consecutive calls.
    pub fn enqueue_response(&self, resp: ModelResponse) {
        if let Ok(mut queue) = self.response_queue.lock() {
            queue.push_back(resp);
        }
    }
}

#[async_trait]
impl ModelProvider for FakeProvider {
    fn provider_id(&self) -> &str {
        &self.name
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let queued = self
            .response_queue
            .lock()
            .ok()
            .and_then(|mut q| q.pop_front());

        if let Some(resp) = queued {
            return Ok(resp);
        }

        Ok(ModelResponse::text(
            format!(
                "{}: processed prompt length {}",
                self.default_response,
                req.prompt.len()
            ),
            format!("{}-model-v1", self.name),
            42,
        ))
    }

    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<mpsc::Receiver<ProviderEvent>, DomainError> {
        let resp = self.generate(req).await?;
        let (tx, rx) = mpsc::channel(4);
        let req_id = req.request_id.clone();

        tokio::spawn(async move {
            // Emit chunk
            let _ = tx.send(ProviderEvent::chunk(&req_id, &resp.content)).await;

            // Emit completed
            let _ = tx
                .send(ProviderEvent::completed(
                    &req_id,
                    Some(resp.content),
                    Some(TokenUsage {
                        input_tokens: Some(req_id.len()),
                        output_tokens: Some(resp.tokens_used),
                    }),
                ))
                .await;
        });

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_provider_generate_and_queue() {
        let provider = FakeProvider::new("test-fake");
        let req = ProviderRequest::simple("Hello Custos");

        let r1 = provider.generate(&req).await.unwrap();
        assert!(r1.content.contains("Deterministic fake response"));

        provider.enqueue_response(ModelResponse::text(
            "Custom queued response",
            "test-model",
            100,
        ));
        let r2 = provider.generate(&req).await.unwrap();
        assert_eq!(r2.content, "Custom queued response");
        assert_eq!(r2.tokens_used, 100);
    }

    #[tokio::test]
    async fn test_fake_provider_streaming() {
        let provider = FakeProvider::new("test-fake");
        let req = ProviderRequest::simple("Stream test");

        let mut rx = provider.stream(&req).await.unwrap();
        let chunk = rx.recv().await.unwrap();
        assert_eq!(
            chunk.event_type,
            custos_provider_sdk::ProviderEventType::Chunk
        );

        let completed = rx.recv().await.unwrap();
        assert_eq!(
            completed.event_type,
            custos_provider_sdk::ProviderEventType::Completed
        );
    }
}
