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

    /// Updates the draft content. Invariant: Editing the content of an approved draft
    /// immediately invalidates approval, resets status to "draft", and clears the payload lock.
    pub fn update_content(&mut self, new_content: impl Into<String>) {
        self.content = new_content.into();
        self.status = "draft".to_string();
        self.payload_lock = None;
        self.updated_at = Utc::now().to_rfc3339();
    }
}

/// The four distinct canonical delivery lifecycle states for outbox mutations:
/// Draft -> RequestedSend -> (ObservedSent | Uncertain)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxDeliveryState {
    Draft,
    RequestedSend,
    ObservedSent,
    Uncertain,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: String,
}

impl OutboxActionItem {
    pub fn delivery_state(&self) -> OutboxDeliveryState {
        match self.status.as_str() {
            "pending_approval" | "draft" => OutboxDeliveryState::Draft,
            "approved" | "dispatching" => OutboxDeliveryState::RequestedSend,
            "receipted" | "observed_sent" => OutboxDeliveryState::ObservedSent,
            _ => OutboxDeliveryState::Uncertain,
        }
    }

    /// Mark uncertain delivery on connector timeout or transport ambiguity.
    /// Blind resend is strictly prohibited when in Uncertain state.
    pub fn mark_uncertain(&mut self) {
        self.status = "uncertain".to_string();
    }
}

/// Durable standing automation grant with bounded scope, expiry, and run quota.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingGrant {
    pub grant_id: String,
    pub scope: String,
    pub max_runs: u32,
    pub runs_count: u32,
    pub expires_at: i64,
    pub is_revoked: bool,
}

impl StandingGrant {
    pub fn new(
        grant_id: impl Into<String>,
        scope: impl Into<String>,
        max_runs: u32,
        expires_at: i64,
    ) -> Self {
        Self {
            grant_id: grant_id.into(),
            scope: scope.into(),
            max_runs,
            runs_count: 0,
            expires_at,
            is_revoked: false,
        }
    }

    pub fn is_active(&self, current_time: i64) -> bool {
        !self.is_revoked && self.runs_count < self.max_runs && current_time < self.expires_at
    }

    pub fn revoke(&mut self) {
        self.is_revoked = true;
    }

    pub fn record_run(&mut self) -> Result<(), &'static str> {
        if self.is_revoked {
            return Err("Standing grant has been revoked");
        }
        if self.runs_count >= self.max_runs {
            return Err("Standing grant quota exceeded");
        }
        self.runs_count += 1;
        Ok(())
    }
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

    #[test]
    fn test_outbox_delivery_states_and_uncertainty() {
        let mut action = OutboxActionItem {
            id: "act-1".into(),
            task_id: "t-1".into(),
            title: "Send Notice".into(),
            action_type: "email_dispatch".into(),
            recipient: "team@example.com".into(),
            payload_summary: "Weekly summary".into(),
            payload_digest: "sha256:abc".into(),
            status: "pending_approval".into(),
            permit_id: None,
            receipt_id: None,
            payload_json: None,
            error_message: None,
            created_at: Utc::now().to_rfc3339(),
        };

        assert_eq!(action.delivery_state(), OutboxDeliveryState::Draft);

        action.status = "approved".into();
        assert_eq!(action.delivery_state(), OutboxDeliveryState::RequestedSend);

        action.status = "receipted".into();
        assert_eq!(action.delivery_state(), OutboxDeliveryState::ObservedSent);

        action.mark_uncertain();
        assert_eq!(action.delivery_state(), OutboxDeliveryState::Uncertain);
        assert_eq!(action.status, "uncertain");
    }

    #[test]
    fn test_standing_grant_quota_and_revocation() {
        let now = 1_000_000;
        let mut grant = StandingGrant::new("grant-1", "calendar.events", 2, now + 3600);
        assert!(grant.is_active(now));

        grant.record_run().expect("First run allowed");
        assert_eq!(grant.runs_count, 1);
        assert!(grant.is_active(now));

        grant.record_run().expect("Second run allowed");
        assert_eq!(grant.runs_count, 2);
        // Exceeded quota
        assert!(!grant.is_active(now));
        assert!(grant.record_run().is_err());

        // Revocation
        let mut grant2 = StandingGrant::new("grant-2", "email.send", 10, now + 3600);
        assert!(grant2.is_active(now));
        grant2.revoke();
        assert!(!grant2.is_active(now));
        assert!(grant2.record_run().is_err());
    }
}
