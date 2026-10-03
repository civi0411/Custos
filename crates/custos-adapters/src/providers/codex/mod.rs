//! CodexProvider Adapter

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelRequest, ModelResponse};

pub struct CodexProvider {
    provider_id: String,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self {
            provider_id: "openai-codex".to_string(),
        }
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
        &self.provider_id
    }

    async fn generate(&self, _req: &ModelRequest) -> Result<ModelResponse, DomainError> {
        Err(DomainError::Validation(
            "Codex transport is not configured; this placeholder cannot perform inference".into(),
        ))
    }
}
