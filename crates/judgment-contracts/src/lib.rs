use async_trait::async_trait;
use custos_core_domain::DomainError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FastJudgment {
    pub confidence: f32,
    pub route: String,
    pub reason: String,
}

#[async_trait]
pub trait JudgmentEngine: Send + Sync {
    async fn judge(&self, prompt: &str) -> Result<FastJudgment, DomainError>;
}
