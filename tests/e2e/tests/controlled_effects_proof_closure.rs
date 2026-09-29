//! Controlled Effects & Proof Closure E2E Test (PR-04)
//!
//! Validates:
//! 1. One read-only repo tool (`read_file`, `list_files`) under sandbox with real SHA256 digests.
//! 2. Patch preview tool (`patch_preview`) generates simulated unified diff without disk mutation.
//! 3. Requires full chain: Intent -> Permit -> Sandbox -> Receipt -> Evidence -> Completion Gate.
//! 4. Proves Denied outcomes:
//!    - High/critical risk actions without approval are denied/require approval.
//!    - Sandbox containment violations (`../` escaping workspace or absolute paths outside) are denied.
//! 5. Proves Uncertain outcomes:
//!    - Execution failure (missing file) generates `success: false` and `ReceiptStatus::Failure`.
//!    - Completion gate blocks task completion when claims fail or are missing.
//! 6. Proves Success outcome:
//!    - Real daemon process accepts verified evidence claims and closes proof for a contract task.

use custos_core::capability::{DeterministicGate, ToolGate};
use custos_core::evidence::{EvidenceBundle, EvidencePipeline};
use custos_core_domain::{
    Action, ContractEvidence, DomainError, EvidenceKind, ReceiptStatus, RiskLevel, TaskContract,
    TaskStatus,
};
use custos_daemon::custos_local_api;
use custos_domain as custos_core_domain;
use custos_local_api::{LocalApiClient, ProcessTransport};
use std::path::PathBuf;

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

#[tokio::test]
async fn test_controlled_effects_and_proof_closure_full_flow() {
    let daemon_bin = resolve_daemon_binary();
    assert!(
        daemon_bin.exists(),
        "custos-daemon binary must exist at {:?}. Run `cargo build --bin custos-daemon` first.",
        daemon_bin
    );

    let test_run_id = uuid::Uuid::new_v4();
    let temp_dir = std::env::temp_dir().join(format!("custos_effects_e2e_{test_run_id}"));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let db_path = temp_dir.join("effects_test.db");
    let db_path_str = db_path.to_str().unwrap();

    // Prepare a mock repository inside sandbox workspace
    let repo_dir = temp_dir.join("workspace");
    std::fs::create_dir_all(repo_dir.join("src")).unwrap();

    let sample_file = repo_dir.join("src/lib.rs");
    let sample_content = "pub fn verify_sovereignty() -> bool {\n    true\n}\n";
    std::fs::write(&sample_file, sample_content).unwrap();

    let gate = DeterministicGate::with_workspace(&repo_dir);
    let pipeline = EvidencePipeline::with_standard_verifiers();

    // =========================================================================
    // STEP 1: Read-Only Repo Tool (`read_file`) under Sandbox
    // =========================================================================
    let read_action = Action::new(
        "act_read_repo".into(),
        "read_file".into(),
        "src/lib.rs".into(),
        serde_json::json!({"path": "src/lib.rs"}),
        RiskLevel::Low,
    );

    let read_result = gate
        .dispatch_for_task("task_effects_1", &read_action, "agent_developer")
        .await
        .expect("Low-risk read_file must be authorized by AuthorityEngine");

    assert!(read_result.success);
    assert!(read_result.permit_id.is_some());
    assert!(read_result.evidence.is_some());
    let read_evidence = read_result.evidence.clone().unwrap();
    assert!(read_evidence.starts_with("sha256:"));

    let read_receipt = read_result.receipt.expect("Must produce ExecutionReceipt");
    assert_eq!(read_receipt.status, ReceiptStatus::Success);
    assert_eq!(read_receipt.output_digest, read_evidence);
    assert_eq!(read_result.output["content"], sample_content);

    // Form verified claim from read evidence
    let read_bundle = EvidenceBundle::new(
        "task_effects_1",
        "hash",
        "src/lib.rs content matches digest",
        serde_json::json!({
            "actual_digest": read_evidence,
            "expected_digest": read_evidence,
        }),
    );
    let read_claim = pipeline
        .verify_bundle(&read_bundle)
        .await
        .expect("Bundle verification succeeds");
    assert!(read_claim.passed);
    assert_eq!(read_claim.verifier_id, "hash");

    // =========================================================================
    // STEP 2: Patch Preview Tool (`patch_preview`) Simulated Without Disk Mutation
    // =========================================================================
    let patch_action = Action::new(
        "act_patch_repo".into(),
        "patch_preview".into(),
        "src/lib.rs".into(),
        serde_json::json!({
            "path": "src/lib.rs",
            "patch": "+ pub fn additional_security_check() -> bool { true }"
        }),
        RiskLevel::Low,
    );

    let patch_result = gate
        .dispatch_for_task("task_effects_1", &patch_action, "agent_developer")
        .await
        .expect("Low-risk patch_preview must be authorized");

    assert!(patch_result.success);
    assert_eq!(patch_result.output["simulated"], true);
    assert!(patch_result.output["preview"]
        .as_str()
        .unwrap()
        .contains("additional_security_check"));
    let patch_evidence = patch_result.evidence.clone().unwrap();

    let patch_receipt = patch_result.receipt.expect("Must produce ExecutionReceipt");
    assert_eq!(patch_receipt.status, ReceiptStatus::Success);

    // CRITICAL: Prove zero disk mutations!
    let disk_content_after_preview = std::fs::read_to_string(&sample_file).unwrap();
    assert_eq!(disk_content_after_preview, sample_content);

    // Form verified claim from patch preview evidence
    let patch_bundle = EvidenceBundle::new(
        "task_effects_1",
        "patch_preview",
        "Patch preview simulation verified",
        serde_json::json!({
            "simulated": true,
            "diff_digest": patch_evidence,
            "target": "src/lib.rs",
        }),
    );
    let patch_claim = pipeline
        .verify_bundle(&patch_bundle)
        .await
        .expect("Patch bundle verification succeeds");
    assert!(patch_claim.passed);
    assert_eq!(patch_claim.verifier_id, "patch_preview");

    // =========================================================================
    // STEP 3: Prove Denied Outcomes
    // =========================================================================
    // 3a: Sandbox containment traversal outside workspace
    let traversal_action = Action::new(
        "act_trav".into(),
        "read_file".into(),
        "../../etc/shadow".into(),
        serde_json::json!({"path": "../../etc/shadow"}),
        RiskLevel::Low,
    );
    let trav_err = gate
        .dispatch_for_task("task_effects_1", &traversal_action, "agent_developer")
        .await
        .unwrap_err();
    match trav_err {
        DomainError::Unauthorized(msg) => {
            assert!(
                msg.contains("Sandbox containment violation"),
                "Unexpected msg: {msg}"
            );
        }
        other => panic!("Expected Unauthorized sandbox error, got: {other:?}"),
    }

    // 3b: High risk action without approval
    let high_risk_action = Action::new(
        "act_rm".into(),
        "delete_file".into(),
        "src/lib.rs".into(),
        serde_json::json!({"path": "src/lib.rs"}),
        RiskLevel::High,
    );
    let high_err = gate
        .dispatch_for_task("task_effects_1", &high_risk_action, "agent_developer")
        .await
        .unwrap_err();
    assert!(
        matches!(high_err, DomainError::Conflict(_)),
        "Expected Conflict for high risk without approval, got {high_err:?}"
    );

    // 3c: Critical risk action denied
    let crit_action = Action::new(
        "act_drop".into(),
        "drop_database".into(),
        "effects_test.db".into(),
        serde_json::json!({}),
        RiskLevel::Critical,
    );
    let crit_err = gate
        .dispatch_for_task("task_effects_1", &crit_action, "agent_developer")
        .await
        .unwrap_err();
    assert!(
        matches!(crit_err, DomainError::Unauthorized(_)),
        "Expected Unauthorized for critical risk action, got {crit_err:?}"
    );

    // =========================================================================
    // STEP 4: Prove Uncertain / Failure Outcomes
    // =========================================================================
    let missing_action = Action::new(
        "act_missing".into(),
        "read_file".into(),
        "does_not_exist.rs".into(),
        serde_json::json!({"path": "does_not_exist.rs"}),
        RiskLevel::Low,
    );
    let miss_result = gate
        .dispatch_for_task("task_effects_1", &missing_action, "agent_developer")
        .await
        .expect("Gate dispatch completes with failure outcome");

    assert!(!miss_result.success);
    assert!(miss_result.evidence.is_none());
    let miss_receipt = miss_result.receipt.expect("Failure receipt generated");
    assert_eq!(miss_receipt.status, ReceiptStatus::Failure);

    // =========================================================================
    // STEP 5: Real Daemon Process Proof Closure Integration
    // =========================================================================
    let transport = ProcessTransport::spawn(daemon_bin.to_str().unwrap(), Some(db_path_str))
        .expect("Spawn daemon");
    let client = LocalApiClient::new(Box::new(transport));

    // Contract requires both FileAnchor (verified by hash/citation) and Diff (verified by patch_preview/diff)
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
        .create_task(
            "req_task_1",
            "Audit and patch auth logic",
            Some(contract),
            None,
        )
        .await
        .expect("Task created");
    assert_eq!(task.status, TaskStatus::Draft);

    // Advance to Running
    let queued = client
        .advance_task("req_adv_1", &task.id, TaskStatus::Queued)
        .await
        .expect("Advance to Queued");
    assert_eq!(queued.status, TaskStatus::Queued);

    let running = client
        .advance_task("req_adv_2", &task.id, TaskStatus::Running)
        .await
        .expect("Advance to Running");
    assert_eq!(running.status, TaskStatus::Running);

    // Attempt 1: Complete without evidence -> MUST FAIL
    let err_empty_claims = client
        .complete_task(
            "req_comp_empty",
            &task.id,
            Some("Premature complete".into()),
        )
        .await
        .unwrap_err();
    assert!(
        err_empty_claims.contains("Proof-closure violation"),
        "Expected Proof-closure violation, got: {err_empty_claims}"
    );

    // Attempt 2: Complete with only 1 of 2 claims -> MUST FAIL
    let err_partial_claims = client
        .complete_task_with_evidence(
            "req_comp_partial",
            &task.id,
            Some("Partial proof".into()),
            vec![read_claim.clone()],
        )
        .await
        .unwrap_err();
    assert!(
        err_partial_claims.contains("Proof-closure violation"),
        "Expected Proof-closure violation for missing Diff, got: {err_partial_claims}"
    );

    // Attempt 3: Complete with all required verified claims -> MUST SUCCEED
    let succeeded = client
        .complete_task_with_evidence(
            "req_comp_full",
            &task.id,
            Some("All evidence verified and proof closed".into()),
            vec![read_claim, patch_claim],
        )
        .await
        .expect("Task completes successfully with full proof closure");

    assert_eq!(succeeded.status, TaskStatus::Succeeded);
    assert_eq!(succeeded.id, task.id);

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}
