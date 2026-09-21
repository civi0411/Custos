use async_trait::async_trait;
use custos_core_domain::DomainError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidenceBundle {
    pub id: String,
    pub verification_type: String,
    pub proof: String,
}

#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(&self, bundle: &EvidenceBundle) -> Result<bool, DomainError>;
}
