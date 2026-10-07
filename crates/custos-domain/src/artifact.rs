//! Artifact References & Storage Metadata
//!
//! Immutable references to content-addressed or file-system artifacts
//! generated throughout the task execution lifecycle.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    FileDiff,
    DiagnosticLog,
    VerificationReport,
    ExecutionTrace,
    Snapshot,
    ContinuationState,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: String,
    pub kind: ArtifactKind,
    pub hash: String,
    pub path: String,
    pub size_bytes: u64,
    pub mime: Option<String>,
}

impl ArtifactRef {
    pub fn new(
        id: String,
        kind: ArtifactKind,
        hash: String,
        path: String,
        size_bytes: u64,
    ) -> Self {
        Self {
            id,
            kind,
            hash,
            path,
            size_bytes,
            mime: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffConsent {
    Pending,
    Granted,
    Denied,
}

/// Selected artifact transfer between pack obligations.
/// Authority never follows this value: grants, permits, and secrets are intentionally absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactHandoff {
    pub id: String,
    pub parent_task_id: String,
    pub source_node_id: Option<String>,
    pub producer_pack: String,
    pub consumer_pack: String,
    pub artifact_ref: String,
    pub artifact_type: String,
    pub artifact_version: String,
    pub artifact_digest: String,
    pub source_dependencies: Vec<String>,
    pub selected_content_refs: Vec<String>,
    pub redacted_refs: Vec<String>,
    pub privacy_class: String,
    pub intent: String,
    pub consent: HandoffConsent,
    pub caveats: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl ArtifactHandoff {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parent_task_id: impl Into<String>,
        producer_pack: impl Into<String>,
        consumer_pack: impl Into<String>,
        artifact_ref: impl Into<String>,
        artifact_type: impl Into<String>,
        artifact_version: impl Into<String>,
        artifact_digest: impl Into<String>,
        privacy_class: impl Into<String>,
        intent: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let parent_task_id = parent_task_id.into();
        let producer_pack = producer_pack.into();
        let consumer_pack = consumer_pack.into();
        let artifact_ref = artifact_ref.into();
        let artifact_type = artifact_type.into();
        let artifact_version = artifact_version.into();
        let artifact_digest = artifact_digest.into();
        let privacy_class = privacy_class.into();
        let intent = intent.into();
        if [
            parent_task_id.as_str(),
            producer_pack.as_str(),
            consumer_pack.as_str(),
            artifact_ref.as_str(),
            artifact_type.as_str(),
            artifact_version.as_str(),
            artifact_digest.as_str(),
            privacy_class.as_str(),
            intent.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(DomainError::Validation(
                "Artifact handoff identity, provenance, digest, privacy, and intent are required"
                    .into(),
            ));
        }
        if producer_pack == consumer_pack {
            return Err(DomainError::Validation(
                "Cross-pack artifact handoff requires distinct producer and consumer packs".into(),
            ));
        }

        Ok(Self {
            id: new_id("handoff"),
            parent_task_id,
            source_node_id: None,
            producer_pack,
            consumer_pack,
            artifact_ref,
            artifact_type,
            artifact_version,
            artifact_digest,
            source_dependencies: Vec::new(),
            selected_content_refs: Vec::new(),
            redacted_refs: Vec::new(),
            privacy_class,
            intent,
            consent: HandoffConsent::Pending,
            caveats: Vec::new(),
            created_at: Utc::now(),
        })
    }

    pub fn grant_consent(&mut self) -> Result<(), DomainError> {
        if self.consent != HandoffConsent::Pending {
            return Err(DomainError::Conflict(format!(
                "Artifact handoff {} consent is already resolved",
                self.id
            )));
        }
        self.consent = HandoffConsent::Granted;
        Ok(())
    }

    pub fn deny_consent(&mut self) -> Result<(), DomainError> {
        if self.consent != HandoffConsent::Pending {
            return Err(DomainError::Conflict(format!(
                "Artifact handoff {} consent is already resolved",
                self.id
            )));
        }
        self.consent = HandoffConsent::Denied;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_requires_cross_pack_provenance_and_single_consent_decision() {
        let invalid = ArtifactHandoff::new(
            "task_1",
            "research",
            "research",
            "artifact_1",
            "engineering_brief",
            "1",
            "digest",
            "user_private",
            "Implement the selected finding",
        );
        assert!(matches!(invalid, Err(DomainError::Validation(_))));

        let mut handoff = ArtifactHandoff::new(
            "task_1",
            "research",
            "engineering",
            "artifact_1",
            "engineering_brief",
            "1",
            "digest",
            "user_private",
            "Implement the selected finding",
        );
        assert!(handoff.is_ok());
        if let Ok(ref mut value) = handoff {
            assert!(value.grant_consent().is_ok());
            assert!(value.deny_consent().is_err());
        }
    }
}
