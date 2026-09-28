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
}
