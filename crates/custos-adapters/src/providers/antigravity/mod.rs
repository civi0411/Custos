//! AntigravityProvider Adapter

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::{ModelProvider, ModelRequest, ModelResponse};

pub struct AntigravityProvider {
    provider_id: String,
}

impl AntigravityProvider {
    pub fn new() -> Self {
        Self {
            provider_id: "google-antigravity".to_string(),
        }
    }
}

impl Default for AntigravityProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelProvider for AntigravityProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn generate(&self, _req: &ModelRequest) -> Result<ModelResponse, DomainError> {
        Err(DomainError::Validation(
            "Antigravity transport is not configured; this placeholder cannot perform inference"
                .into(),
        ))
    }
}
