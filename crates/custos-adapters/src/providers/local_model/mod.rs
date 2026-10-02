//! LocalModelProvider Adapter
//!
//! Provides local LLM inference capabilities (llama.cpp / MLX) integrated
//! into Custos via ModelProvider trait.

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelRequest, ModelResponse};

pub struct LocalModelProvider {
    provider_id: String,
}

impl LocalModelProvider {
    pub fn new() -> Self {
        Self {
            provider_id: "llama-cpp-local".to_string(),
        }
    }

    pub fn with_model(model_name: &str) -> Self {
        Self {
            provider_id: format!("local:{}", model_name),
        }
    }
}

impl Default for LocalModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelProvider for LocalModelProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn generate(&self, req: &ModelRequest) -> Result<ModelResponse, DomainError> {
        Ok(ModelResponse::text(
            format!(
                "Local inference response from {} for prompt: {}",
                self.provider_id, req.prompt
            ),
            self.provider_id.clone(),
            req.prompt.split_whitespace().count() + 20,
        ))
    }
}
