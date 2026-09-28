use async_trait::async_trait;
use custos_core_domain::DomainError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DomainPackManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

#[async_trait]
pub trait DomainPack: Send + Sync {
    fn manifest(&self) -> DomainPackManifest;
    async fn initialize(&self) -> Result<(), DomainError>;
}
