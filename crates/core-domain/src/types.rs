//! Core Primitives & Utility Functions
//!
//! Provides deterministic IDs, SHA-256 digests, canonical JSON representations,
//! and common domain errors.

use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Integrity check failed: expected {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },

    #[error("Budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation failed: {0}")]
    Validation(String),
}

/// Generates a prefixed unique ID: `{prefix}_{uuidv4}`
pub fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

/// Computes a hex-encoded SHA-256 digest of arbitrary bytes
pub fn digest(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Serializes a JSON value into a canonical deterministic string
pub fn canonical_json(value: &serde_json::Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

/// Immutable reference to a content-addressed or file-system artifact
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactRef {
    pub id: String,
    pub hash: String,
    pub path: String,
    pub mime: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_id_has_prefix() {
        let id = new_id("task");
        assert!(id.starts_with("task_"));
        assert_eq!(id.len(), 5 + 32);
    }

    #[test]
    fn test_digest_deterministic() {
        let h1 = digest(b"custos-continuation");
        let h2 = digest(b"custos-continuation");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}
