//! Continuation Packet & Cross-Span State Preservation
//!
//! Enables swapping models and resuming tasks without losing state.
//! Verifies state integrity using SHA-256 over canonical payloads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::{canonical_json, digest};

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
}
