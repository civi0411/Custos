//! Identifiers & Cryptographic Digests
//!
//! Provides deterministic IDs, SHA-256 digests, and canonical JSON representations.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generates a prefixed unique ID: `{prefix}_{uuidv4_simple}`
pub fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

/// Task identifier
pub type TaskId = String;

/// Strongly-typed identifier for a workflow execution node
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WorkflowNodeId(pub String);

impl WorkflowNodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(new_id("node"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkflowNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for WorkflowNodeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for WorkflowNodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for WorkflowNodeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Strongly-typed identifier for a desktop workbench pane
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PaneId(pub String);

impl PaneId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(new_id("pane"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for PaneId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for PaneId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for PaneId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Strongly-typed identifier for an asynchronous or headless job
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct JobId(pub String);

impl JobId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(new_id("job"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for JobId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for JobId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for JobId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Strongly-typed identifier for an execution harness or test harness instance
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HarnessId(pub String);

impl HarnessId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(new_id("harness"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HarnessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for HarnessId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for HarnessId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for HarnessId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Computes a hex-encoded SHA-256 digest of arbitrary bytes
pub fn digest(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Serializes a JSON value into a canonical deterministic string
pub fn canonical_json(value: &serde_json::Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

/// Truncate a string slice to at most `max_bytes` without slicing inside a multi-byte UTF-8 char.
pub fn truncate_str_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    &s[..end]
}

/// Truncate a mutable String to at most `max_bytes` without slicing inside a multi-byte UTF-8 char.
pub fn truncate_to_char_boundary(s: &mut String, max_bytes: usize) {
    if s.len() > max_bytes {
        let mut end = max_bytes;
        while !s.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        s.truncate(end);
    }
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
    fn test_typed_identifiers() {
        let node_id = WorkflowNodeId::generate();
        assert!(node_id.as_str().starts_with("node_"));
        assert_eq!(format!("{}", node_id), node_id.as_str());

        let pane_id = PaneId::new("quant_execution");
        assert_eq!(pane_id.as_str(), "quant_execution");

        let job_id = JobId::generate();
        assert!(job_id.as_str().starts_with("job_"));

        let harness_id = HarnessId::new("cargo_test_harness");
        assert_eq!(harness_id.as_ref(), "cargo_test_harness");

        // Serde roundtrip
        let serialized = serde_json::to_string(&node_id).unwrap();
        let deserialized: WorkflowNodeId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(node_id, deserialized);
    }

    #[test]
    fn test_digest_deterministic() {
        let h1 = digest(b"custos-test");
        let h2 = digest(b"custos-test");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_utf8_truncation_safety() {
        // Vietnamese multi-byte: Tiếng Việt
        let s = "Tiếng Việt";
        // 'T' (1), 'i' (1), 'ế' (3 bytes: 0xE1, 0xBA, 0xBF)
        // If we truncate at 3 bytes (middle of 'ế'), it should fall back to 2 bytes ("Ti")
        let truncated = truncate_str_to_char_boundary(s, 3);
        assert_eq!(truncated, "Ti");

        // Truncate at exact boundary (5 bytes -> "Tiế")
        let truncated_exact = truncate_str_to_char_boundary(s, 5);
        assert_eq!(truncated_exact, "Tiế");

        // 4-byte emoji: 🦀 (0xF0, 0x9F, 0xA6, 0x80)
        let emoji_str = "Rust🦀Rules";
        // "Rust" is 4 bytes. If max_bytes is 6 (middle of crab), it should yield "Rust"
        assert_eq!(truncate_str_to_char_boundary(emoji_str, 6), "Rust");
        assert_eq!(truncate_str_to_char_boundary(emoji_str, 8), "Rust🦀");

        // Mut String version
        let mut mut_s = "Tiếng Việt".to_string();
        truncate_to_char_boundary(&mut mut_s, 3);
        assert_eq!(mut_s, "Ti");
    }
}
