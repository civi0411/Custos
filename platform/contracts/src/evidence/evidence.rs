use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Pass,
    Fail,
    Unknown,
    Stale,
}

impl std::fmt::Display for EvidenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceStatus::Pass => write!(f, "pass"),
            EvidenceStatus::Fail => write!(f, "fail"),
            EvidenceStatus::Unknown => write!(f, "unknown"),
            EvidenceStatus::Stale => write!(f, "stale"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRequirement {
    pub kind: String,
    pub description: String,
    pub required: bool,
    pub satisfied: bool,
    pub evidence_ref: Option<String>,
}

/// An EvidenceRecord captures a deterministic verification artifact or execution proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: String,
    pub task_id: String,
    pub action_id: Option<String>,
    pub kind: String,
    pub digest: String,
    pub status: EvidenceStatus,
    pub source_version: Option<String>,
    pub payload: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
}

/// Version 1 Canonical Contract alias for SSOT.
pub type EvidenceRecordV1 = EvidenceRecord;

impl EvidenceRecord {
    pub fn new(
        task_id: impl Into<String>,
        kind: impl Into<String>,
        digest: impl Into<String>,
        status: EvidenceStatus,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: new_id("ev"),
            task_id: task_id.into(),
            action_id: None,
            kind: kind.into(),
            digest: digest.into(),
            status,
            source_version: None,
            payload,
            recorded_at: Utc::now(),
        }
    }

    pub fn with_action_id(mut self, action_id: impl Into<String>) -> Self {
        self.action_id = Some(action_id.into());
        self
    }

    pub fn with_source_version(mut self, source_version: impl Into<String>) -> Self {
        self.source_version = Some(source_version.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationClaim {
    pub id: String,
    pub task_id: String,
    pub claim_statement: String,
    pub criterion_id: String,
    pub verifier_id: String,
    pub status: EvidenceStatus,
    pub source_version: Option<String>,
    pub passed: bool,
    pub details: serde_json::Value,
    pub verified_at: DateTime<Utc>,
}

impl VerificationClaim {
    pub fn new(
        task_id: String,
        claim_statement: String,
        verifier_id: String,
        passed: bool,
        details: serde_json::Value,
    ) -> Self {
        let status = if passed {
            EvidenceStatus::Pass
        } else {
            EvidenceStatus::Fail
        };
        Self {
            id: new_id("vclaim"),
            task_id,
            claim_statement,
            criterion_id: "default".into(),
            verifier_id,
            status,
            source_version: None,
            passed,
            details,
            verified_at: Utc::now(),
        }
    }

    pub fn with_criterion(mut self, criterion_id: impl Into<String>) -> Self {
        self.criterion_id = criterion_id.into();
        self
    }

    pub fn with_source_version(mut self, source_version: impl Into<String>) -> Self {
        self.source_version = Some(source_version.into());
        self
    }

    pub fn mark_stale_if_version_changed(&mut self, current_source_version: &str) -> bool {
        if let Some(v) = &self.source_version {
            if v != current_source_version {
                self.status = EvidenceStatus::Stale;
                self.passed = false;
                return true;
            }
        }
        false
    }
}

/// Canonical verifier output for one acceptance criterion.
/// Unlike the legacy `VerificationClaim`, unknown and stale are first-class states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriterionVerificationRecord {
    pub id: String,
    pub task_id: String,
    pub task_revision: u64,
    pub criterion_id: String,
    pub verifier_method: String,
    pub verifier_version: String,
    pub status: EvidenceStatus,
    pub evidence_refs: Vec<String>,
    pub source_version: Option<String>,
    pub details: serde_json::Value,
    pub verified_at: DateTime<Utc>,
}

impl CriterionVerificationRecord {
    pub fn new(
        task_id: impl Into<String>,
        task_revision: u64,
        criterion_id: impl Into<String>,
        verifier_method: impl Into<String>,
        verifier_version: impl Into<String>,
        status: EvidenceStatus,
        details: serde_json::Value,
    ) -> Result<Self, crate::error::DomainError> {
        let task_id = task_id.into();
        let criterion_id = criterion_id.into();
        let verifier_method = verifier_method.into();
        let verifier_version = verifier_version.into();
        if task_revision == 0
            || [
                task_id.as_str(),
                criterion_id.as_str(),
                verifier_method.as_str(),
                verifier_version.as_str(),
            ]
            .iter()
            .any(|value| value.trim().is_empty())
        {
            return Err(crate::error::DomainError::Validation(
                "Criterion verification requires task revision, criterion, method, and version"
                    .into(),
            ));
        }

        Ok(Self {
            id: new_id("verify"),
            task_id,
            task_revision,
            criterion_id,
            verifier_method,
            verifier_version,
            status,
            evidence_refs: Vec::new(),
            source_version: None,
            details,
            verified_at: Utc::now(),
        })
    }

    pub fn mark_stale_if_version_changed(&mut self, current_source_version: &str) -> bool {
        if self
            .source_version
            .as_deref()
            .is_some_and(|version| version != current_source_version)
        {
            self.status = EvidenceStatus::Stale;
            return true;
        }
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStatus {
    Accepted,
    Partial,
    Rejected,
    Unknown,
}

/// Persistable outcome projection. Core remains responsible for deciding whether
/// the referenced verifier records satisfy the Task completion gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskOutcome {
    pub id: String,
    pub task_id: String,
    pub task_revision: u64,
    pub run_id: String,
    pub status: OutcomeStatus,
    pub criterion_verification_ids: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub summary: String,
    pub recorded_at: DateTime<Utc>,
}

impl TaskOutcome {
    pub fn new(
        task_id: impl Into<String>,
        task_revision: u64,
        run_id: impl Into<String>,
        status: OutcomeStatus,
        criterion_verification_ids: Vec<String>,
        summary: impl Into<String>,
    ) -> Result<Self, crate::error::DomainError> {
        let task_id = task_id.into();
        let run_id = run_id.into();
        let summary = summary.into();
        if task_revision == 0
            || task_id.trim().is_empty()
            || run_id.trim().is_empty()
            || summary.trim().is_empty()
            || criterion_verification_ids.is_empty()
            || criterion_verification_ids
                .iter()
                .any(|id| id.trim().is_empty())
        {
            return Err(crate::error::DomainError::Validation(
                "Task outcome requires task/run identity, revision, verifier records, and summary"
                    .into(),
            ));
        }

        Ok(Self {
            id: new_id("outcome"),
            task_id,
            task_revision,
            run_id,
            status,
            criterion_verification_ids,
            artifact_refs: Vec::new(),
            summary,
            recorded_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_claim_stale_transition() {
        let mut claim = VerificationClaim::new(
            "task_ev".into(),
            "Unit tests pass".into(),
            "exit_code".into(),
            true,
            serde_json::json!({"exit_code": 0}),
        )
        .with_criterion("crit_tests_pass")
        .with_source_version("commit_sha_base");

        assert_eq!(claim.status, EvidenceStatus::Pass);
        assert!(claim.passed);

        // Version unchanged
        assert!(!claim.mark_stale_if_version_changed("commit_sha_base"));
        assert_eq!(claim.status, EvidenceStatus::Pass);

        // Version changed -> must mark stale and fail verification
        assert!(claim.mark_stale_if_version_changed("commit_sha_modified"));
        assert_eq!(claim.status, EvidenceStatus::Stale);
        assert!(!claim.passed);
    }

    #[test]
    fn criterion_verification_preserves_unknown_and_stale_states() {
        let record = CriterionVerificationRecord::new(
            "task_1",
            3,
            "criterion_support",
            "semantic_support_review",
            "1",
            EvidenceStatus::Unknown,
            serde_json::json!({"reason": "insufficient evidence"}),
        );
        assert!(record.is_ok());
        if let Ok(mut value) = record {
            value.source_version = Some("source_v1".into());
            assert_eq!(value.status, EvidenceStatus::Unknown);
            assert!(value.mark_stale_if_version_changed("source_v2"));
            assert_eq!(value.status, EvidenceStatus::Stale);
        }
    }

    #[test]
    fn outcome_requires_verifier_lineage() {
        let outcome = TaskOutcome::new(
            "task_1",
            1,
            "run_1",
            OutcomeStatus::Accepted,
            Vec::new(),
            "Done",
        );
        assert!(matches!(
            outcome,
            Err(crate::error::DomainError::Validation(_))
        ));
    }
}
