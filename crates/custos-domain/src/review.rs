//! Reviewer Record & Evidence Domain Models
//!
//! Models reviewer records with method, evidence, status, criteria checks,
//! and audit trails across tasks, artifacts, runs, diffs, and notes.

use serde::{Deserialize, Serialize};
use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewTargetType {
    Task,
    Artifact,
    Diff,
    Run,
    Note,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewMethod {
    AutomatedVerifier,
    PeerReview,
    ModelEvaluation,
    ContractProof,
    RuntimeInspection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Approved,
    Rejected,
    Degraded,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
    Blocker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub severity: FindingSeverity,
    pub criterion: String,
    pub message: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewerRecord {
    pub id: String,
    pub target_type: ReviewTargetType,
    pub target_id: String,
    pub reviewer: String,
    pub method: ReviewMethod,
    pub status: ReviewStatus,
    pub evidence_summary: String,
    pub evidence_digest: Option<String>,
    pub findings: Vec<ReviewFinding>,
    pub is_fresh: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordReviewParams {
    pub target_type: ReviewTargetType,
    pub target_id: String,
    pub reviewer: String,
    pub method: ReviewMethod,
    pub status: ReviewStatus,
    pub evidence_summary: String,
    pub evidence_digest: Option<String>,
    pub findings: Option<Vec<ReviewFinding>>,
}

impl ReviewerRecord {
    pub fn new(
        target_type: ReviewTargetType,
        target_id: impl Into<String>,
        reviewer: impl Into<String>,
        method: ReviewMethod,
        status: ReviewStatus,
        evidence_summary: impl Into<String>,
        evidence_digest: Option<String>,
        findings: Vec<ReviewFinding>,
    ) -> Result<Self, DomainError> {
        let target_id = target_id.into();
        let reviewer = reviewer.into();
        let evidence_summary = evidence_summary.into();

        if target_id.trim().is_empty() {
            return Err(DomainError::Validation("Target ID cannot be empty".into()));
        }
        if reviewer.trim().is_empty() {
            return Err(DomainError::Validation("Reviewer identifier cannot be empty".into()));
        }
        if evidence_summary.trim().is_empty() {
            return Err(DomainError::Validation("Evidence summary cannot be empty".into()));
        }

        let now = chrono::Utc::now().timestamp();
        Ok(Self {
            id: new_id("rev"),
            target_type,
            target_id,
            reviewer,
            method,
            status,
            evidence_summary,
            evidence_digest,
            findings,
            is_fresh: true,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn mark_stale(&mut self) {
        self.is_fresh = false;
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reviewer_record_creation_and_validation() {
        let rev = ReviewerRecord::new(
            ReviewTargetType::Diff,
            "diff_hash_123",
            "cargo-clippy",
            ReviewMethod::AutomatedVerifier,
            ReviewStatus::Approved,
            "Zero lints or warnings detected on diff",
            Some("sha256:abc1234".into()),
            vec![ReviewFinding {
                severity: FindingSeverity::Info,
                criterion: "no_clippy_warnings".into(),
                message: "Passed clean".into(),
                file_path: Some("crates/custos-domain/src/lib.rs".into()),
                line_number: Some(1),
            }],
        ).unwrap();

        assert_eq!(rev.target_type, ReviewTargetType::Diff);
        assert_eq!(rev.status, ReviewStatus::Approved);
        assert!(rev.is_fresh);
        assert_eq!(rev.findings.len(), 1);

        let err = ReviewerRecord::new(
            ReviewTargetType::Task,
            "",
            "verifier",
            ReviewMethod::ContractProof,
            ReviewStatus::Pending,
            "Summary",
            None,
            vec![],
        );
        assert!(err.is_err());
    }
}
