//! Assistant Workbench Domain Models & Persistence Port
//!
//! Pure domain entity and port trait for communication/document drafts
//! in the Copilot / Assistant Workbench Lens.

use async_trait::async_trait;
use chrono::Utc;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

/// Safe Communication / Document Draft
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantDraft {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub title: String,
    /// Type of draft: "email" | "document" | "schedule" | "note"
    pub kind: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient: Option<String>,
    /// Status: "draft" | "approved" | "archived"
    pub status: String,
    /// SHA256 or cryptographic digest locking the payload content
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_lock: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl AssistantDraft {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        kind: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            task_id: None,
            session_id: None,
            title: title.into(),
            kind: kind.into(),
            content: content.into(),
            recipient: None,
            status: "draft".to_string(),
            payload_lock: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_recipient(mut self, recipient: impl Into<String>) -> Self {
        self.recipient = Some(recipient.into());
        self
    }

    pub fn with_payload_lock(mut self, digest: impl Into<String>) -> Self {
        self.payload_lock = Some(digest.into());
        self
    }

    /// Updates the draft content. Invariant: Editing the content of an approved draft
    /// immediately invalidates approval, resets status to "draft", and clears the payload lock.
    pub fn update_content(&mut self, new_content: impl Into<String>) {
        self.content = new_content.into();
        self.status = "draft".to_string();
        self.payload_lock = None;
        self.updated_at = Utc::now().to_rfc3339();
    }
}

/// Persistence Port for Assistant Communication & Document Drafts.
#[async_trait]
pub trait AssistantDraftPort: Send + Sync {
    async fn save_draft(&self, draft: &AssistantDraft) -> Result<(), DomainError>;
    async fn get_draft(&self, id: &str) -> Result<Option<AssistantDraft>, DomainError>;
    async fn list_drafts(
        &self,
        session_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<AssistantDraft>, DomainError>;
    async fn delete_draft(&self, id: &str) -> Result<bool, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draft_edit_invalidates_approval_and_lock() {
        let mut draft = AssistantDraft::new("d1", "Status Update", "email", "Initial draft");
        draft.status = "approved".to_string();
        draft.payload_lock = Some("sha256:123".to_string());

        // Modifying content must invalidate approval
        draft.update_content("Modified draft after approval");
        assert_eq!(draft.status, "draft");
        assert_eq!(draft.payload_lock, None);
        assert_eq!(draft.content, "Modified draft after approval");
    }
}

