//! Evidence Requirements & Verification Claims
//!
//! Structured evidence tracking ensuring that actions and task completions
//! satisfy rigorous verification criteria.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::new_id;

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
    pub verifier_id: String,
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
        Self {
            id: new_id("vclaim"),
            task_id,
            claim_statement,
            verifier_id,
            passed,
            details,
            verified_at: Utc::now(),
        }
    }
}
