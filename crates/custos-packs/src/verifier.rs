//! Domain Pack Verifiers (RFC 003 §4 / Gate 4 Proof-Closure)
//!
//! Provides independent oracles for validating task completion before
//! closing deliberation or committing external effects.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Result of an oracle verification evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum VerificationResult {
    /// Requirement fully verified by independent evidence.
    Pass { detail: String },
    /// Requirement failed with a reproducible violation.
    Fail { reason: String },
    /// Inconclusive evidence or environment error.
    Unknown { reason: String },
    /// Underlying evidence or artifact has become stale.
    Stale { reason: String },
}

impl VerificationResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, VerificationResult::Pass { .. })
    }
}

/// Context supplied to a verifier execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationContext {
    pub task_id: String,
    pub workspace_path: Option<String>,
    pub target_artifact: Option<String>,
    pub metadata: serde_json::Value,
}

impl VerificationContext {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            workspace_path: None,
            target_artifact: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_workspace(mut self, path: impl Into<String>) -> Self {
        self.workspace_path = Some(path.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Independent Oracle trait for Gate 4 / Proof-Closure validation.
#[async_trait]
pub trait PackVerifier: Send + Sync {
    /// Unique verifier ID matching `PackProfile.preferred_verifier`
    fn verifier_id(&self) -> &'static str;

    /// Human-readable explanation of this oracle's criteria
    fn description(&self) -> &'static str;

    /// Run the oracle check against current artifacts and environment
    async fn verify(&self, ctx: &VerificationContext) -> VerificationResult;
}
