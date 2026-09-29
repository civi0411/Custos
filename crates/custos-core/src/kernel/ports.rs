//! Persistence Ports for Task Kernel
//!
//! Decouples the kernel logic from underlying databases (SQLite, etc.).

use crate::events::TaskEvent;
use async_trait::async_trait;
use custos_domain::{
    ContinuationPacket, DomainError, Session, SessionId, SessionJournalEntry, Span, Task,
};

#[async_trait]
pub trait TaskStore: Send + Sync {
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError>;
    async fn list_tasks(&self) -> Result<Vec<Task>, DomainError>;
    async fn save_task(&self, task: &Task) -> Result<(), DomainError>;
    async fn commit_task_event(&self, event: &TaskEvent, task: &Task) -> Result<(), DomainError>;
    async fn get_task_events(&self, task_id: &str) -> Result<Vec<TaskEvent>, DomainError>;
    async fn get_span(&self, span_id: &str) -> Result<Option<Span>, DomainError>;
    async fn save_span(&self, span: &Span) -> Result<(), DomainError>;
    async fn list_spans(&self, task_id: &str) -> Result<Vec<Span>, DomainError>;
    async fn save_continuation(&self, packet: &ContinuationPacket) -> Result<(), DomainError>;
    async fn get_continuation(
        &self,
        task_id: &str,
        to_span: u32,
    ) -> Result<Option<ContinuationPacket>, DomainError>;
    async fn get_latest_continuation(
        &self,
        task_id: &str,
    ) -> Result<Option<ContinuationPacket>, DomainError>;
}

/// Persistence Port for Interactive Sessions & Interaction Journals
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn get_session(&self, session_id: &SessionId) -> Result<Option<Session>, DomainError>;
    async fn list_sessions(&self) -> Result<Vec<Session>, DomainError>;
    async fn save_session(&self, session: &Session) -> Result<(), DomainError>;
    async fn append_journal(&self, entry: &SessionJournalEntry) -> Result<i64, DomainError>;
    async fn get_journal(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<SessionJournalEntry>, DomainError>;
}
