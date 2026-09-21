use async_trait::async_trait;
use custos_core_domain::{ContextItem, DomainError};

#[derive(Debug, Clone)]
pub struct ContextSlice {
    pub items: Vec<ContextItem>,
    pub total_tokens: usize,
}

#[async_trait]
pub trait ContextBuilder: Send + Sync {
    async fn compile(&self, query: &str, max_tokens: usize) -> Result<ContextSlice, DomainError>;
}
