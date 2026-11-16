//! Personal Assistant Governed Outbox Application Service
//!
//! Enforces two-phase user approval and payload locks on assistant side-effects.

use custos_contracts::{
    authority::{OutboxEntry, OutboxPort, OutboxStatus},
    identity::error::DomainError,
};
use std::sync::Arc;

pub struct OutboxService {
    outbox: Arc<dyn OutboxPort>,
}

impl OutboxService {
    pub fn new(outbox: Arc<dyn OutboxPort>) -> Self {
        Self { outbox }
    }

    pub async fn list(&self, status: Option<OutboxStatus>) -> Result<Vec<OutboxEntry>, DomainError> {
        match status {
            Some(s) => self.outbox.list_by_status(s).await,
            None => self.outbox.list_by_status(OutboxStatus::Pending).await,
        }
    }

    pub async fn approve(&self, entry_id: &str) -> Result<(), DomainError> {
        self.outbox.mark_approved(entry_id).await
    }

    pub async fn get(&self, entry_id: &str) -> Result<Option<OutboxEntry>, DomainError> {
        self.outbox.get(entry_id).await
    }
}
