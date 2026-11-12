//! Governed Outbox Mutation Contracts
//!
//! Shared across workbenches (Assistant communications, Quant execution tickets)
//! to enforce Draft -> RequestedSend -> (ObservedSent | Uncertain) state transitions
//! and prohibit blind retries under uncertainty.

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

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
}

