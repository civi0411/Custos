//! Personal Assistant Drafts Application Service
//!
//! Provides pure domain operations on assistant drafts.

use crate::domain::{AssistantDraft, AssistantDraftPort};
use custos_contracts::identity::error::DomainError;

pub struct DraftsService<P: AssistantDraftPort> {
    port: P,
}

impl<P: AssistantDraftPort> DraftsService<P> {
    pub fn new(port: P) -> Self {
        Self { port }
    }

    pub async fn list(&self, session_id: Option<&str>, task_id: Option<&str>) -> Result<Vec<AssistantDraft>, DomainError> {
        self.port.list_drafts(session_id, task_id).await
    }

    pub async fn get(&self, id: &str) -> Result<Option<AssistantDraft>, DomainError> {
        self.port.get_draft(id).await
    }

    pub async fn save(&self, draft: &AssistantDraft) -> Result<(), DomainError> {
        self.port.save_draft(draft).await
    }

    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        self.port.delete_draft(id).await
    }
}
