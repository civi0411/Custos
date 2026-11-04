use std::sync::Arc;

use async_trait::async_trait;
use custos_provider::base::Provider;
use custos_provider::conversation::message::Message;
use custos_provider::conversation::token_usage::ProviderUsage;
use custos_provider::errors::ProviderError;
use custos_provider::model::ModelConfig;

/// The single completion call compaction needs. Implementations decide model
/// selection, fallbacks and session plumbing.
#[async_trait]
pub trait CompactionModel: Send + Sync {
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
    ) -> Result<(Message, ProviderUsage), ProviderError>;
}

/// Counts tokens for usage estimation and retained-context reporting.
#[async_trait]
pub trait TokenEstimator: Send + Sync {
    async fn count_chat_tokens(&self, system: &str, messages: &[Message]) -> usize;
    async fn count_text_tokens(&self, text: &str) -> usize;
}

pub struct ProviderModel {
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
}

impl ProviderModel {
    pub fn new(provider: Arc<dyn Provider>, model_config: ModelConfig) -> Self {
        Self {
            provider,
            model_config,
        }
    }

    /// Construct a `ProviderModel` from the canonical hexagonal `ModelProvider` (`ModelPort`).
    pub fn from_model_provider(
        provider: Arc<dyn custos_provider::port::ModelProvider>,
        model_config: ModelConfig,
    ) -> Self {
        Self::new(
            Arc::new(custos_provider::bridge::ModelPortToProvider::new(provider)),
            model_config,
        )
    }
}

#[async_trait]
impl CompactionModel for ProviderModel {
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        self.provider
            .complete(&self.model_config, system, messages, &[])
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::DomainError;
    use custos_provider::port::ModelProvider;
    use custos_provider::request::{ModelResponse, ProviderRequest};
    use custos_provider::events::ProviderEvent;
    use tokio::sync::mpsc;

    struct DummyModelProvider;

    #[async_trait]
    impl ModelProvider for DummyModelProvider {
        fn provider_id(&self) -> &str {
            "dummy"
        }

        async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
            Ok(ModelResponse::text(format!("compacted: {}", req.prompt), &req.model, 42))
        }

        async fn stream(
            &self,
            _req: &ProviderRequest,
        ) -> Result<mpsc::Receiver<ProviderEvent>, DomainError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_provider_model_from_model_provider() {
        let dummy = Arc::new(DummyModelProvider);
        let model_config = ModelConfig::new("test-compact-model");
        let compaction_model = ProviderModel::from_model_provider(dummy, model_config);

        let messages = vec![Message::user().with_text("history to compact")];
        let (msg, usage) = compaction_model.complete("summarize", &messages).await.unwrap();

        assert_eq!(msg.content[0].as_text().unwrap(), "compacted: System: summarize\n\nUser: history to compact\n");
        assert_eq!(usage.model, "test-compact-model");
    }
}

