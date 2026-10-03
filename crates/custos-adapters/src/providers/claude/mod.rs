//! ClaudeProvider Adapter

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelRequest, ModelResponse};

pub struct ClaudeProvider {
    provider_id: String,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        Self {
            provider_id: "anthropic-claude".to_string(),
        }
    }
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelProvider for ClaudeProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn generate(&self, _req: &ModelRequest) -> Result<ModelResponse, DomainError> {
        Err(DomainError::Validation(
            "Claude transport is not configured; this placeholder cannot perform inference".into(),
        ))
    }
}
