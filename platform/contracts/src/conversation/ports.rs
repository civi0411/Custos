//! Conversation SPI Trait Ports

use async_trait::async_trait;
use crate::identity::error::DomainError;
use crate::conversation::session::{Session, SessionId, SessionJournalEntry};

/// Persistence Port for Interactive Sessions & Interaction Journals
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn get_session(&self, session_id: &SessionId) -> Result<Option<Session>, DomainError>;
    async fn list_sessions(&self) -> Result<Vec<Session>, DomainError>;
    async fn save_session(&self, session: &Session) -> Result<(), DomainError>;
    async fn delete_session(&self, session_id: &SessionId) -> Result<bool, DomainError>;
    async fn append_journal(&self, entry: &SessionJournalEntry) -> Result<i64, DomainError>;
    async fn get_journal(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<SessionJournalEntry>, DomainError>;
}

