//! Controlled Effects & Proof Closure E2E Test (PR-04)
//!
//! Organized into clear thematic test modules:
//! 1. `sandbox_and_preview_tools`: Read-only sandbox execution & zero-mutation patch preview.
//! 2. `security_containment_and_rejection`: Containment traversal denial, risk authorization gates, failure receipts.
//! 3. `daemon_proof_closure_completion`: Process-level proof closure gate requiring verified evidence bundles.

use custos_core::capability::{DeterministicGate, ToolGate};
use custos_core::evidence::{EvidenceBundle, EvidencePipeline};
use custos_core_domain::{
    Action, ContractEvidence, DomainError, EvidenceKind, ReceiptStatus, RiskLevel, TaskContract,
    TaskStatus, VerificationClaim,
};
use custos_daemon::custos_local_api;
use custos_domain as custos_core_domain;
use custos_local_api::{LocalApiClient, ProcessTransport};
use std::path::{Path, PathBuf};

fn resolve_daemon_binary() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_custos-daemon") {
        return PathBuf::from(p);
    }
    if let Ok(mut current) = std::env::current_exe() {
        current.pop();
        if current.file_name().and_then(|n| n.to_str()) == Some("deps") {
            current.pop();
        }
        let candidate = current.join(if cfg!(windows) {
            "custos-daemon.exe"
        } else {
            "custos-daemon"
        });
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from("target/debug/custos-daemon")
}

fn setup_test_workspace(dir: &Path) -> (PathBuf, PathBuf) {
    let repo_dir = dir.join("workspace");
    std::fs::create_dir_all(repo_dir.join("src")).unwrap();
    let sample_file = repo_dir.join("src/lib.rs");
    std::fs::write(
        &sample_file,
        "pub fn verify_sovereignty() -> bool {\n    true\n}\n",
    )
    .unwrap();
    (repo_dir, sample_file)
}

// =========================================================================
// THEME 1: Sandbox Read & Zero-Mutation Patch Preview Tools
// =========================================================================
mod sandbox_and_preview_tools {
    use super::*;

    #[tokio::test]
    async fn test_read_and_patch_preview_simulation_with_zero_disk_mutation() {
        let temp_dir =
            std::env::temp_dir().join(format!("custos_eff_preview_{}", uuid::Uuid::new_v4()));
        let (repo_dir, sample_file) = setup_test_workspace(&temp_dir);
        let sample_content = std::fs::read_to_string(&sample_file).unwrap();

        let gate = DeterministicGate::with_workspace(&repo_dir);
        let pipeline = EvidencePipeline::with_standard_verifiers();

        // 1. Read-only repo tool (`read_file`) under Sandbox
        let read_act = Action::new(
            "act_read".into(),
            "read_file".into(),
            "src/lib.rs".into(),
            serde_json::json!({"path": "src/lib.rs"}),
            RiskLevel::Low,
        );
        let read_res = gate
            .dispatch_for_task("t1", &read_act, "agent")
            .await
            .expect("read authorized");
        assert!(read_res.success);
        let read_digest = read_res.evidence.expect("must produce evidence digest");
        assert!(read_digest.starts_with("sha256:"));

        let read_receipt = read_res.receipt.expect("must produce receipt");
        assert_eq!(read_receipt.status, ReceiptStatus::Success);
        assert_eq!(read_receipt.output_digest, read_digest);

        let read_bundle = EvidenceBundle::new(
            "t1",
            "hash",
            "src/lib.rs content matches digest",
            serde_json::json!({
                "actual_digest": read_digest, "expected_digest": read_digest,
            }),
        );
        let read_claim = pipeline
            .verify_bundle(&read_bundle)
            .await
            .expect("verify bundle");
        assert!(read_claim.passed && read_claim.verifier_id == "hash");

        // 2. Patch preview simulation with ZERO disk mutation
        let patch_act = Action::new(
            "act_patch".into(),
            "patch_preview".into(),
            "src/lib.rs".into(),
            serde_json::json!({
                "path": "src/lib.rs", "patch": "+ pub fn extra_check() -> bool { true }"
            }),
            RiskLevel::Low,
        );
        let patch_res = gate
            .dispatch_for_task("t1", &patch_act, "agent")
            .await
            .expect("patch authorized");
        assert!(patch_res.success);
        assert_eq!(patch_res.output["simulated"], true);
        assert!(patch_res.output["preview"]
            .as_str()
            .unwrap()
            .contains("extra_check"));

        // Verify zero disk mutation
        let disk_after = std::fs::read_to_string(&sample_file).unwrap();
        assert_eq!(
            disk_after, sample_content,
            "patch preview must never mutate disk"
        );

        let patch_bundle = EvidenceBundle::new(
            "t1",
            "patch_preview",
            "Patch preview simulation verified",
            serde_json::json!({
                "simulated": true, "diff_digest": patch_res.evidence.unwrap(), "target": "src/lib.rs",
            }),
        );
        let patch_claim = pipeline
            .verify_bundle(&patch_bundle)
            .await
            .expect("verify patch bundle");
        assert!(patch_claim.passed && patch_claim.verifier_id == "patch_preview");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

// =========================================================================
// THEME 2: Security Containment, Traversal Denial & Failure Receipts
// =========================================================================
mod security_containment_and_rejection {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_containment_and_authorization_rejection() {
        let temp_dir =
            std::env::temp_dir().join(format!("custos_eff_sec_{}", uuid::Uuid::new_v4()));
        let (repo_dir, _) = setup_test_workspace(&temp_dir);
        let gate = DeterministicGate::with_workspace(&repo_dir);

        // 1. Sandbox containment traversal outside workspace
        let trav_act = Action::new(
            "act_trav".into(),
            "read_file".into(),
            "../../etc/shadow".into(),
            serde_json::json!({"path": "../../etc/shadow"}),
            RiskLevel::Low,
        );
        let trav_err = gate
            .dispatch_for_task("t1", &trav_act, "agent")
            .await
            .unwrap_err();
        match trav_err {
            DomainError::Unauthorized(msg) => assert!(
                msg.contains("Sandbox containment violation"),
                "unexpected: {msg}"
            ),
            other => panic!("Expected Unauthorized sandbox error, got: {other:?}"),
        }

        // 2. High risk action without approval
        let high_act = Action::new(
            "act_rm".into(),
            "delete_file".into(),
            "src/lib.rs".into(),
            serde_json::json!({"path": "src/lib.rs"}),
            RiskLevel::High,
        );
        let high_err = gate
            .dispatch_for_task("t1", &high_act, "agent")
            .await
            .unwrap_err();
        assert!(
            matches!(high_err, DomainError::Conflict(_)),
            "Expected Conflict for unapproved high risk, got {high_err:?}"
        );

        // 3. Critical risk action denied outright
        let crit_act = Action::new(
            "act_drop".into(),
            "drop_database".into(),
            "db.sqlite".into(),
            serde_json::json!({}),
            RiskLevel::Critical,
        );
        let crit_err = gate
            .dispatch_for_task("t1", &crit_act, "agent")
            .await
            .unwrap_err();
        assert!(
            matches!(crit_err, DomainError::Unauthorized(_)),
            "Expected Unauthorized for critical risk action, got {crit_err:?}"
        );

        // 4. Missing file execution produces Failure receipt
        let miss_act = Action::new(
            "act_miss".into(),
            "read_file".into(),
            "missing.rs".into(),
            serde_json::json!({"path": "missing.rs"}),
            RiskLevel::Low,
        );
        let miss_res = gate
            .dispatch_for_task("t1", &miss_act, "agent")
            .await
            .expect("dispatch completes with failure");
        assert!(!miss_res.success && miss_res.evidence.is_none());
        assert_eq!(
            miss_res.receipt.expect("receipt").status,
            ReceiptStatus::Failure
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

// =========================================================================
// THEME 3: Daemon Process Proof-Closure Completion Gate
// =========================================================================
mod daemon_proof_closure_completion {
    use super::*;

    async fn create_verified_claims(repo_dir: &Path) -> (VerificationClaim, VerificationClaim) {
        let gate = DeterministicGate::with_workspace(repo_dir);
        let pipeline = EvidencePipeline::with_standard_verifiers();

        let read_act = Action::new(
            "act_r".into(),
            "read_file".into(),
            "src/lib.rs".into(),
            serde_json::json!({"path": "src/lib.rs"}),
            RiskLevel::Low,
        );
        let read_res = gate
            .dispatch_for_task("t_claims", &read_act, "agent")
            .await
            .unwrap();
        let read_digest = read_res.evidence.unwrap();
        let read_claim = pipeline
            .verify_bundle(&EvidenceBundle::new(
                "t_claims",
                "hash",
                "audit hash",
                serde_json::json!({
                    "actual_digest": read_digest, "expected_digest": read_digest,
                }),
            ))
            .await
            .unwrap();

        let patch_act = Action::new(
            "act_p".into(),
            "patch_preview".into(),
            "src/lib.rs".into(),
            serde_json::json!({
                "path": "src/lib.rs", "patch": "+ pub fn audit_ok() -> bool { true }"
            }),
            RiskLevel::Low,
        );
        let patch_res = gate
            .dispatch_for_task("t_claims", &patch_act, "agent")
            .await
            .unwrap();
        let patch_claim = pipeline.verify_bundle(&EvidenceBundle::new("t_claims", "patch_preview", "audit patch", serde_json::json!({
            "simulated": true, "diff_digest": patch_res.evidence.unwrap(), "target": "src/lib.rs",
        }))).await.unwrap();

        (read_claim, patch_claim)
    }

    #[tokio::test]
    async fn test_daemon_process_proof_closure_gate_enforcement() {
        let daemon_bin = resolve_daemon_binary();
        assert!(
            daemon_bin.exists(),
            "daemon binary must exist at {:?}",
            daemon_bin
        );

        let temp_dir =
            std::env::temp_dir().join(format!("custos_eff_gate_{}", uuid::Uuid::new_v4()));
        let (repo_dir, _) = setup_test_workspace(&temp_dir);
        let db_path = temp_dir.join("effects_test.db");

        let transport = ProcessTransport::spawn(
            daemon_bin.to_str().unwrap(),
            Some(db_path.to_str().unwrap()),
        )
        .expect("Spawn daemon");
        let client = LocalApiClient::new(Box::new(transport));

        let contract = TaskContract {
            pack_id: "engineering".into(),
            name: "Security Audited Patch".into(),
            description: "Requires verified file content and patch preview".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![
                ContractEvidence {
                    kind: EvidenceKind::FileAnchor,
                    required: true,
                },
                ContractEvidence {
                    kind: EvidenceKind::Diff,
                    required: true,
                },
            ],
        };

        let task = client
            .create_task("req_1", "Audit and patch auth logic", Some(contract), None)
            .await
            .expect("Task created");
        assert_eq!(task.status, TaskStatus::Draft);

        client
            .advance_task("req_2", &task.id, TaskStatus::Queued)
            .await
            .expect("Queued");
        client
            .advance_task("req_3", &task.id, TaskStatus::Running)
            .await
            .expect("Running");

        let (read_claim, patch_claim) = create_verified_claims(&repo_dir).await;

        // Attempt 1: Complete without evidence -> MUST FAIL
        let err_empty = client
            .complete_task("req_e", &task.id, Some("Premature".into()))
            .await
            .unwrap_err();
        assert!(
            err_empty.contains("Proof-closure violation"),
            "got: {err_empty}"
        );

        // Attempt 2: Complete with partial claims -> MUST FAIL
        let err_partial = client
            .complete_task_with_evidence(
                "req_p",
                &task.id,
                Some("Partial".into()),
                vec![read_claim.clone()],
            )
            .await
            .unwrap_err();
        assert!(
            err_partial.contains("Proof-closure violation"),
            "got: {err_partial}"
        );

        // Attempt 3: Complete with all required verified claims -> MUST SUCCEED
        let succeeded = client
            .complete_task_with_evidence(
                "req_ok",
                &task.id,
                Some("Closed".into()),
                vec![read_claim, patch_claim],
            )
            .await
            .expect("Proof closed");
        assert_eq!(succeeded.status, TaskStatus::Succeeded);
        assert_eq!(succeeded.id, task.id);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
