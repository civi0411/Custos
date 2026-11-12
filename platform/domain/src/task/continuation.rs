//! Continuation Packet & Cross-Span State Preservation
//!
//! Enables swapping models and resuming tasks without losing state.
//! Verifies state integrity using SHA-256 over canonical payloads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::{canonical_json, digest, new_id, TaskId};
use crate::session::{SessionId, WorkbenchLens};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationPacket {
    pub task_id: String,
    pub from_span: u32,
    pub to_span: u32,
    pub provider: String,
    pub model: String,
    pub task_summary: String,
    pub current_state: serde_json::Value,
    pub integrity_hash: String,
    pub created_at: DateTime<Utc>,
}

impl ContinuationPacket {
    pub fn compute_hash(
        task_id: &str,
        from_span: u32,
        to_span: u32,
        task_summary: &str,
        current_state: &serde_json::Value,
    ) -> Result<String, DomainError> {
        let state_canonical =
            canonical_json(current_state).map_err(|e| DomainError::Validation(e.to_string()))?;
        let payload = format!("{task_id}|{from_span}|{to_span}|{task_summary}|{state_canonical}");
        Ok(digest(payload.as_bytes()))
    }

    pub fn create(
        task_id: String,
        from_span: u32,
        to_span: u32,
        provider: String,
        model: String,
        task_summary: String,
        current_state: serde_json::Value,
    ) -> Result<Self, DomainError> {
        let integrity_hash =
            Self::compute_hash(&task_id, from_span, to_span, &task_summary, &current_state)?;

        Ok(Self {
            task_id,
            from_span,
            to_span,
            provider,
            model,
            task_summary,
            current_state,
            integrity_hash,
            created_at: Utc::now(),
        })
    }

    pub fn verify(&self) -> Result<(), DomainError> {
        let computed = Self::compute_hash(
            &self.task_id,
            self.from_span,
            self.to_span,
            &self.task_summary,
            &self.current_state,
        )?;

        if computed != self.integrity_hash {
            return Err(DomainError::IntegrityCheckFailed {
                expected: self.integrity_hash.clone(),
                actual: computed,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationDisplayScope {
    SelectedOnly,
    FullSourceTimeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestedModelScope {
    Selected,
    SummaryPlusSelected,
    BoundedFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferPrivacyDecision {
    Approved,
    Redacted,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationState {
    Committed,
    NeedsContext,
    Ready,
    Delivered,
    Blocked,
}

impl ContinuationState {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Committed, Self::NeedsContext)
                | (Self::NeedsContext, Self::Ready | Self::Blocked)
                | (Self::Ready, Self::Delivered | Self::Blocked)
                | (Self::Blocked, Self::NeedsContext)
        )
    }
}

/// Immutable selection and lineage for a linked continuation.
/// This record deliberately contains no reusable permit, grant, credential, or native checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationManifest {
    pub transfer_id: String,
    pub source_session_id: SessionId,
    pub source_checkpoint_entry_id: String,
    pub destination_session_id: SessionId,
    pub source_task_id: Option<TaskId>,
    pub destination_task_id: Option<TaskId>,
    pub target_lens: WorkbenchLens,
    pub actor_id: String,
    pub command_id: String,
    pub created_at: DateTime<Utc>,
    pub selected_turn_ids: Vec<String>,
    pub selected_source_refs: Vec<String>,
    pub selected_artifact_refs: Vec<String>,
    pub display_scope: ContinuationDisplayScope,
    pub requested_model_scope: RequestedModelScope,
    pub privacy_decision: TransferPrivacyDecision,
    pub source_task_revision: Option<u64>,
    pub manifest_version: u32,
    pub state: ContinuationState,
}

impl ContinuationManifest {
    pub fn new(
        source_session_id: SessionId,
        source_checkpoint_entry_id: impl Into<String>,
        destination_session_id: SessionId,
        target_lens: WorkbenchLens,
        actor_id: impl Into<String>,
        command_id: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let source_checkpoint_entry_id = source_checkpoint_entry_id.into();
        let actor_id = actor_id.into();
        let command_id = command_id.into();
        if source_session_id == destination_session_id {
            return Err(DomainError::Validation(
                "Linked continuation requires a distinct destination session".into(),
            ));
        }
        if source_checkpoint_entry_id.trim().is_empty()
            || actor_id.trim().is_empty()
            || command_id.trim().is_empty()
        {
            return Err(DomainError::Validation(
                "Continuation checkpoint, actor, and command IDs are required".into(),
            ));
        }

        Ok(Self {
            transfer_id: new_id("transfer"),
            source_session_id,
            source_checkpoint_entry_id,
            destination_session_id,
            source_task_id: None,
            destination_task_id: None,
            target_lens,
            actor_id,
            command_id,
            created_at: Utc::now(),
            selected_turn_ids: Vec::new(),
            selected_source_refs: Vec::new(),
            selected_artifact_refs: Vec::new(),
            display_scope: ContinuationDisplayScope::SelectedOnly,
            requested_model_scope: RequestedModelScope::Selected,
            privacy_decision: TransferPrivacyDecision::Denied,
            source_task_revision: None,
            manifest_version: 1,
            state: ContinuationState::Committed,
        })
    }

    pub fn bind_tasks(
        mut self,
        source_task_id: Option<TaskId>,
        destination_task_id: Option<TaskId>,
        source_task_revision: Option<u64>,
    ) -> Result<Self, DomainError> {
        if source_task_revision == Some(0) {
            return Err(DomainError::Validation(
                "Continuation source task revision must be non-zero".into(),
            ));
        }
        if source_task_revision.is_some() && source_task_id.is_none() {
            return Err(DomainError::Validation(
                "Continuation revision requires a source task".into(),
            ));
        }
        self.source_task_id = source_task_id;
        self.destination_task_id = destination_task_id;
        self.source_task_revision = source_task_revision;
        Ok(self)
    }

    pub fn transition(&mut self, next: ContinuationState) -> Result<(), DomainError> {
        if !self.state.can_transition_to(next) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.state).to_lowercase(),
                to: format!("{next:?}").to_lowercase(),
            });
        }
        self.state = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_creation_and_verification() {
        let packet = ContinuationPacket::create(
            "task_001".into(),
            1,
            2,
            "anthropic".into(),
            "claude-3-5-sonnet".into(),
            "Refactoring parser module".into(),
            serde_json::json!({"step": "ast_generated"}),
        )
        .expect("Must create packet");

        assert!(packet.verify().is_ok());

        let mut tampered = packet.clone();
        tampered.task_summary = "Malicious edit".into();
        assert!(tampered.verify().is_err());
    }

    #[test]
    fn continuation_manifest_rejects_same_session_and_invalid_transitions() {
        let same_session = SessionId::new("session_1");
        let invalid = ContinuationManifest::new(
            same_session.clone(),
            "entry_4",
            same_session,
            WorkbenchLens::Coding,
            "operator",
            "command_1",
        );
        assert!(matches!(invalid, Err(DomainError::Validation(_))));

        let mut manifest = ContinuationManifest::new(
            SessionId::new("session_1"),
            "entry_4",
            SessionId::new("session_2"),
            WorkbenchLens::Coding,
            "operator",
            "command_1",
        );
        assert!(manifest.is_ok());
        if let Ok(ref mut value) = manifest {
            assert!(value.transition(ContinuationState::Delivered).is_err());
            assert!(value.transition(ContinuationState::NeedsContext).is_ok());
            assert!(value.transition(ContinuationState::Ready).is_ok());
            assert!(value.transition(ContinuationState::Delivered).is_ok());
        }
    }
}
