//! ClaudeProvider Adapter

use async_trait::async_trait;
use custos_core_domain::DomainError;
use custos_provider_sdk::{ModelProvider, ModelRequest, ModelResponse};

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

    async fn generate(&self, req: &ModelRequest) -> Result<ModelResponse, DomainError> {
        Ok(ModelResponse {
            content: format!(
                "Stub response from {} for: {}",
                self.provider_id, req.prompt
            ),
            tokens_used: 42,
            model_id: self.provider_id.clone(),
        })
    }
}
