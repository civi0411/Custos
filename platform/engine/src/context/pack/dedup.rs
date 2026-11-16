//! Hash Deduplication for Context Compilation
//!
//! Eliminates identical content chunks based on cryptographic digests (Custos.md §6.4 Step 5).

use custos_domain::ContextItem;
use std::collections::HashSet;

pub struct HashDeduplicator;

impl HashDeduplicator {
    /// Filters out duplicate items that produce identical normalized SHA-256 digests.
    pub fn deduplicate(items: Vec<ContextItem>) -> Vec<ContextItem> {
        let mut seen_hashes = HashSet::new();
        let mut deduped = Vec::with_capacity(items.len());

        for item in items {
            // Compute deterministic digest of item content
            let item_hash = format!(
                "sha256:{}",
                custos_domain::digest(item.content.trim().as_bytes())
            );

            if seen_hashes.insert(item_hash) {
                deduped.push(item);
            }
        }

        deduped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplicate_identical_content() {
        let items = vec![
            ContextItem {
                id: "item1".into(),
                source: "file_a.rs".into(),
                content: "pub fn common_helper() {}".into(),
                score: 0.9,
                tokens: 10,
                provenance_hash: None,
                is_tainted: false,
            },
            ContextItem {
                id: "item2".into(),
                source: "file_b.rs".into(),
                content: "pub fn common_helper() {}".into(), // Duplicate!
                score: 0.8,
                tokens: 10,
                provenance_hash: None,
                is_tainted: false,
            },
            ContextItem {
                id: "item3".into(),
                source: "file_c.rs".into(),
                content: "pub fn unique_function() {}".into(),
                score: 0.7,
                tokens: 12,
                provenance_hash: None,
                is_tainted: false,
            },
        ];

        let deduped = HashDeduplicator::deduplicate(items);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].id, "item1");
        assert_eq!(deduped[1].id, "item3");
    }
}
