use async_trait::async_trait;
use custos_core_domain::DomainError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelRequest {
    pub prompt: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub stop_sequences: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub tokens_used: usize,
    pub model_id: String,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    async fn generate(&self, req: &ModelRequest) -> Result<ModelResponse, DomainError>;
}
