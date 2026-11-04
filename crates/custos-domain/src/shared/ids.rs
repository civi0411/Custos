//! Identifiers & Cryptographic Digests
//!
//! Provides deterministic IDs, SHA-256 digests, and canonical JSON representations.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generates a prefixed unique ID: `{prefix}_{uuidv4_simple}`
pub fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

/// Task identifier
pub type TaskId = String;

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
