//! First Vertical Slice: `repo_explain`
//!
//! Master Reference: dev_docs/README.md (Section 4)
//! Proves the entire architectural loop end-to-end:
//! CLI/Service -> Task Kernel -> SQLite Persistence -> Workspace Intelligence
//! -> Context Compiler -> FakeProvider -> Citation Verifier -> Artifact Store
//! -> Evidence Engine -> Crash Recovery.

use custos_adapter_provider_fake::FakeProvider;
use custos_artifact_store::{ArtifactStore, FsArtifactStore};
use custos_context_compiler::{ContextBuilder, SourceDocument, TokenAwareContextCompiler};
use custos_core_domain::{ContinuationPacket, Span, SpanState, TaskStatus};
use custos_evidence_engine::{EvidenceBundle, EvidencePipeline};
use custos_persistence_sqlite::SqliteTaskStore;
use custos_provider_sdk::{ModelProvider, ProviderRequest};
use custos_repo_intelligence::WorkspaceScanner;
use custos_task_kernel::{AdvanceTask, CompleteTask, CreateTask, TaskService, TaskStore};
use std::sync::Arc;

#[tokio::test]
async fn test_repo_explain_vertical_slice_end_to_end() {
    let test_run_id = uuid::Uuid::new_v4().to_string();
    let temp_root = std::env::temp_dir().join(format!("custos_repo_explain_{test_run_id}"));
    std::fs::create_dir_all(&temp_root).unwrap();

    let fixture_repo = temp_root.join("fixture_repo");
    let artifact_dir = temp_root.join("artifacts");
    let db_path = temp_root.join("custos.db");
    let db_path_str = db_path.to_str().unwrap();

    // -------------------------------------------------------------
    // Step 0: Setup a local fixture repository
    // -------------------------------------------------------------
    let persistence_mod_dir = fixture_repo.join("crates/persistence-sqlite/src");
    std::fs::create_dir_all(&persistence_mod_dir).unwrap();

    let persistence_code = r#"// Persistence SQLite Implementation
// Responsible for persisting tasks into local SQLite database.
pub struct SqliteTaskStore {
    db_path: String,
}
impl SqliteTaskStore {
    pub fn new(path: &str) -> Self {
        Self { db_path: path.to_string() }
    }
}
"#;
    std::fs::write(persistence_mod_dir.join("lib.rs"), persistence_code).unwrap();

    let core_domain_dir = fixture_repo.join("crates/core-domain/src");
    std::fs::create_dir_all(&core_domain_dir).unwrap();
    let core_domain_code = r#"// Core Domain Architecture
// UI interacts with TaskKernel via typed commands and local IPC,
// never touching SQLite directly to enforce auditability and invariant gates.
pub struct Task;
"#;
    std::fs::write(core_domain_dir.join("lib.rs"), core_domain_code).unwrap();

    let (task_id, artifact_ref) = {
        // -------------------------------------------------------------
        // Step 1: Initialize Task Kernel & Persistence
        // -------------------------------------------------------------
        let store = Arc::new(SqliteTaskStore::new(db_path_str).expect("Failed to create SQLite store"));
        let service = TaskService::new(store.clone());

        // Step 2: Create task via Task Kernel
        let prompt_title = "Which module is responsible for persisting tasks, and why can the UI not directly access the database?".to_string();
        let (task, _) = service
            .execute_create(CreateTask {
                title: prompt_title.clone(),
                metadata: Some(serde_json::json!({
                    "domain": "coding",
                    "scenario": "repo_explain",
                    "repo_path": fixture_repo.to_str().unwrap(),
                })),
            })
            .await
            .expect("Task creation must succeed");

        assert_eq!(task.status, TaskStatus::Draft);
        assert_eq!(task.epoch, 0);

        // Step 3: Advance state Draft -> Queued -> Running
        let (task, _) = service
            .execute_advance(AdvanceTask {
                task_id: task.id.clone(),
                next_status: TaskStatus::Queued,
                expected_epoch: 0,
                rationale: Some("Queued for repository intelligence worker".into()),
            })
            .await
            .expect("Advance to Queued must succeed");
        assert_eq!(task.status, TaskStatus::Queued);

        let (task, _) = service
            .execute_advance(AdvanceTask {
                task_id: task.id.clone(),
                next_status: TaskStatus::Running,
                expected_epoch: 1,
                rationale: Some("Worker scanning repo and compiling context".into()),
            })
            .await
            .expect("Advance to Running must succeed");
        assert_eq!(task.status, TaskStatus::Running);

        // -------------------------------------------------------------
        // Step 4: Workspace Engine scans fixture repository
        // -------------------------------------------------------------
        let scanner = WorkspaceScanner::new();
        let inventory = scanner
            .scan_inventory(&fixture_repo)
            .expect("Inventory scan must succeed");
        assert_eq!(inventory.len(), 2);

        // -------------------------------------------------------------
        // Step 5: Context Compiler indexes and selects token-budgeted slice
        // -------------------------------------------------------------
        let mut compiler = TokenAwareContextCompiler::new();
        for file in &inventory {
            let doc = SourceDocument::from_file(&file.absolute_path).expect("Read file");
            compiler.add_document(doc);
        }

        let context_slice = compiler
            .compile("persisting tasks database UI access", 1000)
            .await
            .expect("Context compilation must succeed");

        assert!(!context_slice.items.is_empty());
        assert!(context_slice.total_tokens <= 1000);

        // -------------------------------------------------------------
        // Step 6: FakeProvider generates explanation citing exact file ranges
        // -------------------------------------------------------------
        let cited_rel_path = "crates/persistence-sqlite/src/lib.rs";
        let provider = FakeProvider::with_default_text(
            "fake-provider",
            format!(
                "The module responsible for persisting tasks is `crates/persistence-sqlite`. \
                The UI cannot directly access SQLite because all mutations must pass through the TaskKernel \
                to enforce invariant validation, cryptographic execution permits, and event sourcing. \
                Citation: {} lines 1-6.",
                cited_rel_path
            ),
        );

        let request = ProviderRequest::new(
            "explain-req-1",
            &task.id,
            1,
            &prompt_title,
            "fake-model-v1",
        );
        let model_resp = provider.generate(&request).await.expect("Model generation");
        assert!(model_resp.content.contains("crates/persistence-sqlite"));

        // Record execution span
        let span = Span::new(
            format!("span_explain_{}", task.id),
            task.id.clone(),
            1,
            "provider-fake".into(),
            "fake-model-v1".into(),
            "sha256:prompt_digest".into(),
        );
        store.save_span(&span).await.expect("Save span");

        // Record continuation packet
        let cont = ContinuationPacket::create(
            task.id.clone(),
            1,
            2,
            "provider-fake".into(),
            "fake-model-v1".into(),
            "Completed repo_explain deliberation".into(),
            serde_json::json!({
                "explanation": model_resp.content,
                "cited_files": [cited_rel_path],
            }),
        )
        .expect("ContinuationPacket create");
        store.save_continuation(&cont).await.expect("Save continuation");

        // -------------------------------------------------------------
        // Step 7: Artifact Store saves response deliverable
        // -------------------------------------------------------------
        let artifact_store = FsArtifactStore::new(artifact_dir.clone());
        let deliverable_bytes = model_resp.content.as_bytes();
        let art_ref = artifact_store
            .put(deliverable_bytes, Some("text/markdown".into()))
            .await
            .expect("Artifact put must succeed");
        assert!(artifact_store.exists(&art_ref.hash).await.unwrap());

        // -------------------------------------------------------------
        // Step 8: Evidence Engine verifies citations
        // -------------------------------------------------------------
        let pipeline = EvidencePipeline::with_standard_verifiers();
        let evidence_bundle = EvidenceBundle::new(
            task.id.clone(),
            "citation",
            "Cited persistence implementation file exists and contains SqliteTaskStore",
            serde_json::json!({
                "base_path": fixture_repo.to_str().unwrap(),
                "citations": [
                    {
                        "path": cited_rel_path,
                        "start_line": 1,
                        "end_line": 6,
                        "snippet": "SqliteTaskStore"
                    }
                ]
            }),
        );

        let verification_claim = pipeline
            .verify_bundle(&evidence_bundle)
            .await
            .expect("Evidence bundle verification");

        assert!(verification_claim.passed, "Citation verification must pass");
        assert_eq!(verification_claim.verifier_id, "citation");

        // -------------------------------------------------------------
        // Step 9: Complete Task in Kernel (Succeeded)
        // -------------------------------------------------------------
        let (task, _) = service
            .execute_complete(CompleteTask {
                task_id: task.id.clone(),
                summary: "repo_explain completed with verified citation evidence".into(),
                expected_epoch: 2,
            })
            .await
            .expect("Complete task must succeed");

        assert_eq!(task.status, TaskStatus::Succeeded);
        assert_eq!(task.epoch, 3);

        (task.id, art_ref)
    };

    // -------------------------------------------------------------
    // Step 10: Crash Test / Restart Resilience
    // Simulate complete process crash and reboot by opening a fresh connection
    // -------------------------------------------------------------
    {
        let store2 = Arc::new(SqliteTaskStore::new(db_path_str).expect("Reopen db after crash"));
        let service2 = TaskService::new(store2.clone());

        // Verify task state is fully preserved
        let recovered = service2
            .get_task(&task_id)
            .await
            .unwrap()
            .expect("Task must exist after restart");
        assert_eq!(recovered.id, task_id);
        assert_eq!(recovered.status, TaskStatus::Succeeded);
        assert_eq!(recovered.epoch, 3);

        // Verify span history preserved
        let spans = store2.list_spans(&task_id).await.unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].span_num, 1);
        assert_eq!(spans[0].state, SpanState::Started);

        // Verify continuation packet preserved and cryptographically valid
        let cont = store2
            .get_latest_continuation(&task_id)
            .await
            .unwrap()
            .expect("Continuation exists");
        assert_eq!(cont.task_id, task_id);
        assert!(cont.verify().is_ok(), "Continuation hash integrity verified");

        // Verify artifact store content-addressing preserved
        let artifact_store2 = FsArtifactStore::new(artifact_dir);
        let loaded_bytes = artifact_store2
            .get(&artifact_ref.hash)
            .await
            .unwrap()
            .expect("Artifact exists");
        let content_str = String::from_utf8(loaded_bytes).unwrap();
        assert!(content_str.contains("crates/persistence-sqlite"));
    }

    // Clean up temporary directory
    let _ = std::fs::remove_dir_all(&temp_root);
}
