//! Context Compiler Engine (Custos.md §6.4)
//!
//! Executes the deterministic 8-step Context Compilation Pipeline:
//! 1. Scope Resolution: Ensures all read files are contained within PathSandbox.
//! 2. Structural Extraction: Extracts structural code declarations and docs.
//! 3. Anchor Retrieval: Ranks relevant excerpts against task goals.
//! 4. Taint Tagging: Flags untrusted inputs with is_tainted to block system role pollution.
//! 5. Hash Deduplication: Removes duplicate chunks by SHA-256 digest.
//! 6. Progressive Compaction: Compacts comments/whitespace/prunes when exceeding token budget.
//! 7. Secret Redaction: Masks API keys and high-entropy secrets.
//! 8. Sealing & Digest: Computes immutable context_digest for semantic cache & CAS.

pub mod anchor;
pub mod compaction;
pub mod dedup;
pub mod redaction;
pub mod structural;

pub use anchor::AnchorRetriever;
pub use compaction::ProgressiveCompactor;
pub use dedup::HashDeduplicator;
pub use redaction::SecretRedactor;
pub use structural::StructuralExtractor;

use crate::sandbox::PathSandbox;
use custos_domain::{digest, new_id, ContextItem, ContextPack, DomainError};
use std::path::PathBuf;

/// Request configuration for compiling a context pack
#[derive(Debug, Clone)]
pub struct CompileContextRequest {
    pub task_id: String,
    pub task_goal: String,
    pub candidate_files: Vec<String>,
    pub max_tokens: usize,
    pub untrusted_sources: Vec<String>,
}

pub struct ContextCompiler {
    sandbox: PathSandbox,
}

impl ContextCompiler {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Result<Self, DomainError> {
        let sandbox = PathSandbox::new(workspace_root)?;
        Ok(Self { sandbox })
    }

    pub fn with_sandbox(sandbox: PathSandbox) -> Self {
        Self { sandbox }
    }

    /// Compiles raw candidate files into a sealed, redacted, compact ContextPack.
    pub fn compile(&self, req: &CompileContextRequest) -> Result<ContextPack, DomainError> {
        // Step 1: Scope Resolution (Chặn đứng Path Traversal)
        let mut resolved_paths = Vec::new();
        for rel_path in &req.candidate_files {
            match self.sandbox.resolve_and_contain(rel_path) {
                Ok(path) => {
                    if path.exists() && path.is_file() {
                        resolved_paths.push((rel_path.clone(), path));
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        "Scope resolution rejected candidate path '{}': {}",
                        rel_path,
                        err
                    );
                    // Strictly skip out-of-scope files
                }
            }
        }

        // Step 2 & 3: Structural Extraction & Anchor Retrieval
        let mut raw_items = Vec::new();
        for (rel_path, abs_path) in resolved_paths {
            let raw_content = match std::fs::read_to_string(&abs_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Step 2: Structural outline extraction (lược trích chữ ký, kiểu, docstrings nếu tệp lớn)
            let structural_content = StructuralExtractor::extract_outline(&raw_content, &rel_path);

            // Step 3: Anchor retrieval (trích xuất lát cắt quanh các anchor liên quan mục tiêu tác vụ)
            let focused_content =
                AnchorRetriever::retrieve_anchors(&structural_content, &req.task_goal, 4);

            // Calculate relevance score against task goal
            let score = Self::calculate_relevance(&focused_content, &req.task_goal);
            let tokens = ProgressiveCompactor::estimate_tokens(&focused_content);

            raw_items.push(ContextItem {
                id: new_id("ctx"),
                source: rel_path,
                content: focused_content,
                score,
                tokens,
                provenance_hash: None,
                is_tainted: false,
            });
        }

        // Step 4: Taint Tagging (Gắn nhãn Provenance và Tainted nếu nguồn chưa tin cậy)
        for item in &mut raw_items {
            let is_untrusted = req
                .untrusted_sources
                .iter()
                .any(|u| item.source.contains(u));
            item.is_tainted = is_untrusted;
            let prov_hash = format!("sha256:{}", digest(item.content.as_bytes()));
            item.provenance_hash = Some(prov_hash);
        }

        // Step 5: Hash Deduplication (Khử trùng lặp qua SHA-256)
        let deduped_items = HashDeduplicator::deduplicate(raw_items);

        // Step 6: Progressive Compaction (Nén ngữ cảnh theo Token Budget)
        let mut compacted_items = ProgressiveCompactor::compact(deduped_items, req.max_tokens);

        // Step 7: Secret Redaction (Quét entropy và lọc sạch API Key/Token)
        for item in &mut compacted_items {
            item.content = SecretRedactor::redact(&item.content);
            item.tokens = ProgressiveCompactor::estimate_tokens(&item.content);
        }

        // Step 8: Sealing & Digest (Đóng gói ContextPack và tính Digest cố định)
        let total_tokens = compacted_items.iter().map(|it| it.tokens).sum();
        let pack_id = new_id("ctxpack");

        // Compute deterministic context_digest
        let mut digest_payload = String::new();
        for it in &compacted_items {
            digest_payload.push_str(&it.source);
            digest_payload.push(':');
            digest_payload.push_str(&it.content);
            digest_payload.push('\n');
        }
        let context_digest = format!("sha256:{}", digest(digest_payload.as_bytes()));

        Ok(ContextPack::new(
            pack_id,
            compacted_items,
            total_tokens,
            context_digest,
        ))
    }

    fn calculate_relevance(content: &str, goal: &str) -> f32 {
        if goal.trim().is_empty() {
            return 1.0;
        }

        let goal_words: Vec<&str> = goal.split_whitespace().collect();
        if goal_words.is_empty() {
            return 1.0;
        }

        let mut matches = 0;
        let content_lower = content.to_lowercase();
        for word in &goal_words {
            let clean_word = word
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if !clean_word.is_empty() && content_lower.contains(&clean_word) {
                matches += 1;
            }
        }

        0.5 + 0.5 * (matches as f32 / goal_words.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_compiler_full_pipeline() {
        let temp_dir = std::env::temp_dir().join(format!("custos_ctx_test_{}", new_id("test")));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let file1 = temp_dir.join("main.rs");
        let file2 = temp_dir.join("secret.rs");
        let file3 = temp_dir.join("dup.rs");

        std::fs::write(&file1, "fn main() {\n    println!(\"Target task\");\n}\n").unwrap();
        std::fs::write(
            &file2,
            "// Config with secret\nconst KEY: &str = \"sk-ant-test1234567890abcdef1234567890\";\n",
        )
        .unwrap();
        std::fs::write(&file3, "fn main() {\n    println!(\"Target task\");\n}\n").unwrap(); // Duplicate content of file1

        let compiler = ContextCompiler::new(&temp_dir).unwrap();

        let req = CompileContextRequest {
            task_id: "task_123".into(),
            task_goal: "Fix main target".into(),
            candidate_files: vec![
                "main.rs".into(),
                "secret.rs".into(),
                "dup.rs".into(),
                "../../etc/passwd".into(), // Traversal attempt: Step 1 should reject
            ],
            max_tokens: 500,
            untrusted_sources: vec!["secret.rs".into()],
        };

        let pack = compiler.compile(&req).expect("Compilation must succeed");

        // Step 1: Traversal is excluded
        assert!(!pack.items.iter().any(|it| it.source.contains("passwd")));

        // Step 5: Duplicate file3 is deduplicated
        assert_eq!(pack.items.len(), 2);

        // Step 4: Taint tagging on secret.rs
        let secret_item = pack
            .items
            .iter()
            .find(|it| it.source == "secret.rs")
            .unwrap();
        assert!(secret_item.is_tainted);

        // Step 7: Secret redaction
        assert!(!secret_item.content.contains("sk-ant-test"));
        assert!(secret_item.content.contains("[REDACTED_API_KEY]"));

        // Step 8: Sealing & Digest present
        assert!(pack.context_digest.starts_with("sha256:"));
        assert_eq!(pack.context_digest.len(), 7 + 64);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
