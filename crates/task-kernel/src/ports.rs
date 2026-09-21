//! Persistence Ports for Task Kernel
//!
//! Decouples the kernel logic from underlying databases (SQLite, etc.).

use async_trait::async_trait;
use custos_core_domain::{ContinuationPacket, DomainError, Span, Task};

#[async_trait]
pub trait TaskStore: Send + Sync {
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError>;
    async fn save_task(&self, task: &Task) -> Result<(), DomainError>;
    async fn get_span(&self, span_id: &str) -> Result<Option<Span>, DomainError>;
    async fn save_span(&self, span: &Span) -> Result<(), DomainError>;
    async fn list_spans(&self, task_id: &str) -> Result<Vec<Span>, DomainError>;
    async fn save_continuation(&self, packet: &ContinuationPacket) -> Result<(), DomainError>;
    async fn get_latest_continuation(
        &self,
        task_id: &str,
    ) -> Result<Option<ContinuationPacket>, DomainError>;
}
