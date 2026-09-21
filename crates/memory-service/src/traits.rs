use async_trait::async_trait;
use custos_core_domain::{DomainError, Fact};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTier {
    Working,
    Episodic,
    Semantic,
    Procedural,
    Judgment,
}

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn remember(&self, tier: MemoryTier, fact: Fact) -> Result<(), DomainError>;
    async fn recall(&self, tier: MemoryTier, query: &str) -> Result<Vec<Fact>, DomainError>;
}
