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

use crate::contracts::workspace_files::WorkspaceFilesPort;
use crate::sandbox::PathSandbox;
use custos_domain::{digest, new_id, ContextItem, ContextPack, DomainError, ExecutionWorkspace};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

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
    workspace: Option<ExecutionWorkspace>,
    files_port: Option<Arc<dyn WorkspaceFilesPort>>,
    memory_files: HashMap<String, String>,
}

impl ContextCompiler {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Result<Self, DomainError> {
        let sandbox = PathSandbox::new(workspace_root)?;
        Ok(Self {
            sandbox,
            workspace: None,
            files_port: None,
            memory_files: HashMap::new(),
        })
    }

    pub fn with_sandbox(sandbox: PathSandbox) -> Self {
        Self {
            sandbox,
            workspace: None,
            files_port: None,
            memory_files: HashMap::new(),
        }
    }

    pub fn with_files_port(
        sandbox: PathSandbox,
        workspace: ExecutionWorkspace,
        files_port: Arc<dyn WorkspaceFilesPort>,
    ) -> Self {
        Self {
            sandbox,
            workspace: Some(workspace),
            files_port: Some(files_port),
            memory_files: HashMap::new(),
        }
    }

    pub fn with_memory_files(
        workspace_root: impl Into<PathBuf>,
        files: HashMap<String, String>,
    ) -> Result<Self, DomainError> {
        let sandbox = PathSandbox::new(workspace_root)?;
        Ok(Self {
            sandbox,
            workspace: None,
            files_port: None,
            memory_files: files,
        })
    }

    /// Compiles raw candidate files into a sealed, redacted, compact ContextPack.
    pub async fn compile(&self, req: &CompileContextRequest) -> Result<ContextPack, DomainError> {
        // Step 1: Scope Resolution (Chặn đứng Path Traversal)
        let mut valid_rel_paths = Vec::new();
        for rel_path in &req.candidate_files {
            match self.sandbox.resolve_and_contain(rel_path) {
                Ok(_resolved_abs) => {
                    valid_rel_paths.push(rel_path.clone());
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
        for rel_path in valid_rel_paths {
            let raw_content = if let (Some(port), Some(ws)) = (&self.files_port, &self.workspace) {
                match port.read_file(ws, &rel_path, None).await {
                    Ok(file_content) => file_content.content,
                    Err(_) => continue,
                }
            } else if let Some(content) = self.memory_files.get(&rel_path) {
                content.clone()
            } else {
                continue;
            };

            // Step 2: Structural outline extraction
            let structural_content = StructuralExtractor::extract_outline(&raw_content, &rel_path);

            // Step 3: Anchor retrieval
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
        let goal_keywords: Vec<&str> = goal
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .collect();

        if goal_keywords.is_empty() {
            return 0.5;
        }

        let mut matches = 0;
        let content_lower = content.to_lowercase();
        for kw in &goal_keywords {
            if content_lower.contains(&kw.to_lowercase()) {
                matches += 1;
            }
        }

        (matches as f32) / (goal_keywords.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use custos_domain::{
        WorkspaceDiffSummary, WorkspaceFileContent, WorkspaceFileDiff, WorkspaceFileTree,
        WorkspaceId, WorkspaceKind, WriteWorkspaceFileParams,
    };

    struct MockWorkspaceFilesPort {
        files: HashMap<String, String>,
    }

    #[async_trait]
    impl WorkspaceFilesPort for MockWorkspaceFilesPort {
        async fn get_file_tree(
            &self,
            _workspace: &ExecutionWorkspace,
            _relative_dir: Option<&str>,
            _max_depth: Option<usize>,
        ) -> Result<WorkspaceFileTree, DomainError> {
            unimplemented!()
        }

        async fn read_file(
            &self,
            workspace: &ExecutionWorkspace,
            relative_path: &str,
            _max_bytes: Option<usize>,
        ) -> Result<WorkspaceFileContent, DomainError> {
            if let Some(content) = self.files.get(relative_path) {
                Ok(WorkspaceFileContent {
                    workspace_id: workspace.id.clone(),
                    path: relative_path.to_string(),
                    content: content.clone(),
                    is_binary: false,
                    size_bytes: content.len(),
                    truncated: false,
                    line_count: content.lines().count(),
                })
            } else {
                Err(DomainError::Validation(format!(
                    "File not found: {relative_path}"
                )))
            }
        }

        async fn write_file(
            &self,
            _workspace: &ExecutionWorkspace,
            _params: WriteWorkspaceFileParams,
        ) -> Result<WorkspaceFileContent, DomainError> {
            unimplemented!()
        }

        async fn get_diff(
            &self,
            _workspace: &ExecutionWorkspace,
            _staged: Option<bool>,
        ) -> Result<WorkspaceDiffSummary, DomainError> {
            unimplemented!()
        }

        async fn get_file_diff(
            &self,
            _workspace: &ExecutionWorkspace,
            _relative_path: &str,
            _staged: Option<bool>,
        ) -> Result<WorkspaceFileDiff, DomainError> {
            unimplemented!()
        }

        async fn stage_file(
            &self,
            _workspace: &ExecutionWorkspace,
            _relative_path: &str,
        ) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn unstage_file(
            &self,
            _workspace: &ExecutionWorkspace,
            _relative_path: &str,
        ) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn discard_file(
            &self,
            _workspace: &ExecutionWorkspace,
            _relative_path: &str,
        ) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn commit(
            &self,
            _workspace: &ExecutionWorkspace,
            _message: &str,
        ) -> Result<String, DomainError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_context_compiler_full_pipeline_with_mock_port() {
        let mut files = HashMap::new();
        files.insert(
            "main.rs".into(),
            "fn main() {\n    println!(\"Target task\");\n}\n".into(),
        );
        files.insert(
            "secret.rs".into(),
            "// Config with secret\nconst KEY: &str = \"sk-ant-test1234567890abcdef1234567890\";\n".into(),
        );
        files.insert(
            "dup.rs".into(),
            "fn main() {\n    println!(\"Target task\");\n}\n".into(),
        );

        let port = Arc::new(MockWorkspaceFilesPort { files });
        let workspace = ExecutionWorkspace::new(
            WorkspaceId("ws_test".to_string()),
            "test-ws",
            WorkspaceKind::Folder {
                path: "/workspace".into(),
            },
            "/workspace",
        );
        let sandbox = PathSandbox::new("/workspace").unwrap();
        let compiler = ContextCompiler::with_files_port(sandbox, workspace, port);

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

        let pack = compiler.compile(&req).await.expect("Compilation must succeed");

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
    }
}
