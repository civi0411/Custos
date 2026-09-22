//! Identifiers & Cryptographic Digests
//!
//! Provides deterministic IDs, SHA-256 digests, and canonical JSON representations.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generates a prefixed unique ID: `{prefix}_{uuidv4_simple}`
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
        let h1 = digest(b"custos-test");
        let h2 = digest(b"custos-test");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}
