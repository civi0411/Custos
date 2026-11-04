//! Context Items & Evidence Anchors
//!
//! Sliced, ranked context elements provided to language models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub id: String,
    pub source: String,
    pub content: String,
    pub score: f32,
    pub tokens: usize,
    #[serde(default)]
    pub provenance_hash: Option<String>,
    #[serde(default)]
    pub is_tainted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPack {
    pub id: String,
    pub items: Vec<ContextItem>,
    pub total_tokens: usize,
    #[serde(default)]
    pub context_digest: String,
}

impl ContextPack {
    pub fn new(
        id: String,
        items: Vec<ContextItem>,
        total_tokens: usize,
        context_digest: String,
    ) -> Self {
        Self {
            id,
            items,
            total_tokens,
            context_digest,
        }
    }
}

impl Default for ContextPack {
    fn default() -> Self {
        Self {
            id: crate::ids::new_id("ctx"),
            items: Vec::new(),
            total_tokens: 0,
            context_digest: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OmittedContextReason {
    Stale,
    Unauthorized,
    Redacted,
    Deleted,
    BudgetLimit,
    Unsupported,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmittedContextRef {
    pub reference: String,
    pub reason: OmittedContextReason,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextDeliveryState {
    Prepared,
    Delivered,
    Blocked,
    Unknown,
}

/// Audit record for the exact references a context compiler included or omitted.
/// `Delivered` acknowledges transport only; it does not claim model comprehension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextReceipt {
    pub id: String,
    pub transfer_id: String,
    pub model_attempt_id: Option<String>,
    pub compiler_version: String,
    pub input_digest: String,
    pub included_turn_ids: Vec<String>,
    pub included_source_refs: Vec<String>,
    pub included_artifact_refs: Vec<String>,
    pub omitted_refs: Vec<OmittedContextRef>,
    pub redaction_digest: String,
    pub token_estimate: u64,
    pub destination_provider: String,
    pub destination_model: String,
    pub delivery_state: ContextDeliveryState,
    pub recorded_at: DateTime<Utc>,
}

impl ContextReceipt {
    pub fn new(
        transfer_id: impl Into<String>,
        compiler_version: impl Into<String>,
        input_digest: impl Into<String>,
        redaction_digest: impl Into<String>,
        token_estimate: u64,
        destination_provider: impl Into<String>,
        destination_model: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let transfer_id = transfer_id.into();
        let compiler_version = compiler_version.into();
        let input_digest = input_digest.into();
        let redaction_digest = redaction_digest.into();
        let destination_provider = destination_provider.into();
        let destination_model = destination_model.into();
        if [
            transfer_id.as_str(),
            compiler_version.as_str(),
            input_digest.as_str(),
            redaction_digest.as_str(),
            destination_provider.as_str(),
            destination_model.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(DomainError::Validation(
                "Context receipt identity, digests, compiler, and destination are required".into(),
            ));
        }

        Ok(Self {
            id: new_id("ctxrcpt"),
            transfer_id,
            model_attempt_id: None,
            compiler_version,
            input_digest,
            included_turn_ids: Vec::new(),
            included_source_refs: Vec::new(),
            included_artifact_refs: Vec::new(),
            omitted_refs: Vec::new(),
            redaction_digest,
            token_estimate,
            destination_provider,
            destination_model,
            delivery_state: ContextDeliveryState::Prepared,
            recorded_at: Utc::now(),
        })
    }

    pub fn mark_delivered(
        &mut self,
        model_attempt_id: impl Into<String>,
    ) -> Result<(), DomainError> {
        if self.delivery_state != ContextDeliveryState::Prepared {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.delivery_state).to_lowercase(),
                to: "delivered".into(),
            });
        }
        let model_attempt_id = model_attempt_id.into();
        if model_attempt_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Delivered context requires a model attempt ID".into(),
            ));
        }
        self.model_attempt_id = Some(model_attempt_id);
        self.delivery_state = ContextDeliveryState::Delivered;
        Ok(())
    }

    pub fn mark_blocked(&mut self) -> Result<(), DomainError> {
        if self.delivery_state != ContextDeliveryState::Prepared {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.delivery_state).to_lowercase(),
                to: "blocked".into(),
            });
        }
        self.delivery_state = ContextDeliveryState::Blocked;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_receipt_does_not_equate_delivery_with_usage() {
        let mut receipt = ContextReceipt::new(
            "transfer_1",
            "compiler_1",
            "input_digest",
            "redaction_digest",
            1200,
            "provider",
            "model",
        );
        assert!(receipt.is_ok());
        if let Ok(ref mut value) = receipt {
            assert!(value.mark_delivered("attempt_1").is_ok());
            assert_eq!(value.delivery_state, ContextDeliveryState::Delivered);
            assert_eq!(value.model_attempt_id.as_deref(), Some("attempt_1"));
            assert!(value.mark_blocked().is_err());
        }
    }
}
