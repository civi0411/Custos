//! CodexProvider Adapter / OpenAI Live Model Provider

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelRequest, ModelResponse};
use crate::providers::openai_chat::OpenAiChatProvider;

pub struct CodexProvider {
    inner: OpenAiChatProvider,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self {
            inner: OpenAiChatProvider::new(),
        }
    }

    pub fn with_provider(inner: OpenAiChatProvider) -> Self {
        Self { inner }
    }

    pub fn with_token_provider(mut self, provider: std::sync::Arc<dyn crate::providers::openai_chat::TokenProvider>) -> Self {
        self.inner = self.inner.with_token_provider(provider);
        self
    }
}

impl Default for CodexProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelProvider for CodexProvider {
    fn provider_id(&self) -> &str {
        "openai-codex"
    }

    async fn generate(&self, req: &ModelRequest) -> Result<ModelResponse, DomainError> {
        self.inner.generate(req).await
    }
}
