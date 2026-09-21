use async_trait::async_trait;
use custos_core_domain::{ArtifactRef, DomainError};

#[async_trait]
pub trait ArtifactStore: Send + Sync {
    async fn put(&self, data: &[u8], mime: Option<String>) -> Result<ArtifactRef, DomainError>;
    async fn get(&self, hash: &str) -> Result<Option<Vec<u8>>, DomainError>;
    async fn exists(&self, hash: &str) -> Result<bool, DomainError>;
}
