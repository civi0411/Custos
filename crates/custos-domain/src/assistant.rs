//! Assistant Domain Models (Copilot Lens)
//!
//! Entities for safe communications drafting, payload locking,
//! governed outbox mutations, and truthful action capabilities.

use chrono::Utc;
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
}

/// Governed Outbox Action Item requiring User Acceptance
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxActionItem {
    pub id: String,
    pub task_id: String,
    pub title: String,
    /// Action category: "email_dispatch" | "calendar_schedule" | "webhook_mutate"
    pub action_type: String,
    pub recipient: String,
    pub payload_summary: String,
    pub payload_digest: String,
    /// Status: "pending_approval" | "approved" | "dispatching" | "receipted" | "failed" | "uncertain"
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permit_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
    pub created_at: String,
}

/// Truthful Action Capability returned by backend to prevent phantom UI triggers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceActionCapability {
    pub id: String,
    pub label: String,
    /// Status: "available" | "degraded" | "unavailable"
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub primary: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
}
