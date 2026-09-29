//! Evidence Bundle Definitions
//!
//! Encapsulates proof artifacts and claims for deterministic verification.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub id: String,
    pub task_id: String,
    pub verifier_type: String,
    pub claim_statement: String,
    pub proof: serde_json::Value,
}

impl EvidenceBundle {
    pub fn new(
        task_id: impl Into<String>,
        verifier_type: impl Into<String>,
        claim_statement: impl Into<String>,
        proof: serde_json::Value,
    ) -> Self {
        Self {
            id: custos_core_domain::new_id("ebundle"),
            task_id: task_id.into(),
            verifier_type: verifier_type.into(),
            claim_statement: claim_statement.into(),
            proof,
        }
    }
}
