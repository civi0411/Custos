use super::*;
use custos_core::contracts::ToolGate;
use custos_domain::{Action, DomainError, ReceiptStatus, RiskLevel};
use std::path::PathBuf;

#[tokio::test]
async fn test_low_risk_read_file_allowed_with_permit_sandbox_and_receipt() {
    let temp_dir = PathBuf::from("/workspace/test_env");
    let gate = DeterministicGate::with_workspace(&temp_dir);
    let file_path = temp_dir.join("sample.txt");
    gate.add_memory_file(&file_path, "Hello Custos Sovereign!");

    let action = Action::new(
        "act_read_1".into(),
        "read_file".into(),
        "sample.txt".into(),
        serde_json::json!({"path": "sample.txt"}),
        RiskLevel::Low,
    );

    let res = gate
        .dispatch_for_task("task_test_1", &action, "developer")
        .await
        .expect("Low-risk read_file must be authorized and executed");

    assert!(res.success);
    assert!(res.permit_id.is_some());
    assert!(res.evidence.is_some());
    assert!(res.receipt.is_some());
    let receipt = res.receipt.unwrap();
    assert_eq!(receipt.status, ReceiptStatus::Success);
    assert!(receipt.output_digest.starts_with("sha256:"));
    assert_eq!(res.output["content"], "Hello Custos Sovereign!");
}

#[tokio::test]
async fn test_unknown_low_risk_action_does_not_fabricate_success() {
    let gate = DeterministicGate::default();
    let action = Action::new(
        "act_unknown_1".into(),
        "unknown_tool".into(),
        "target".into(),
        serde_json::json!({"target": "target"}),
        RiskLevel::Low,
    );

    let result = gate
        .dispatch_for_task("task_unknown", &action, "developer")
        .await;
    assert!(matches!(
        result,
        Err(DomainError::Unauthorized(message)) if message.contains("Unsupported capability")
    ));
}

#[tokio::test]
async fn test_low_risk_patch_preview_simulated_without_disk_mutation() {
    let temp_dir = PathBuf::from("/workspace/test_env");
    let gate = DeterministicGate::with_workspace(&temp_dir);
    let file_path = temp_dir.join("code.rs");
    let initial_content = "fn main() { println!(\"old\"); }";
    gate.add_memory_file(&file_path, initial_content);

    let action = Action::new(
        "act_patch_1".into(),
        "patch_preview".into(),
        "code.rs".into(),
        serde_json::json!({
            "path": "code.rs",
            "patch": "+ fn main() { println!(\"new\"); }"
        }),
        RiskLevel::Low,
    );

    let res = gate
        .dispatch_for_task("task_patch_task", &action, "developer")
        .await
        .expect("Patch preview must be authorized");

    assert!(res.success);
    assert_eq!(res.output["simulated"], true);
    assert!(res.output["preview"]
        .as_str()
        .unwrap()
        .contains("+ fn main()"));
    assert!(res.evidence.is_some());

    // CRITICAL CHECK: Verify zero disk mutations occurred!
    let disk_content = gate.get_memory_file(&file_path).unwrap();
    assert_eq!(disk_content, initial_content);
}

#[tokio::test]
async fn test_sandbox_path_traversal_denied() {
    let temp_dir = PathBuf::from("/workspace/test_env");
    let gate = DeterministicGate::with_workspace(&temp_dir);
    let action = Action::new(
        "act_trav_1".into(),
        "read_file".into(),
        "../../etc/passwd".into(),
        serde_json::json!({"path": "../../etc/passwd"}),
        RiskLevel::Low,
    );

    let err = gate
        .dispatch_for_task("task_trav", &action, "developer")
        .await
        .unwrap_err();

    match err {
        DomainError::Unauthorized(msg) => {
            assert!(msg.contains("Sandbox containment violation"));
        }
        other => panic!("Expected Unauthorized sandbox error, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_uncertain_outcome_missing_file_fails_safely() {
    let temp_dir = PathBuf::from("/workspace/test_env");
    let gate = DeterministicGate::with_workspace(&temp_dir);
    let action = Action::new(
        "act_miss_1".into(),
        "read_file".into(),
        "non_existent.txt".into(),
        serde_json::json!({"path": "non_existent.txt"}),
        RiskLevel::Low,
    );

    let res = gate
        .dispatch_for_task("task_miss", &action, "developer")
        .await
        .expect("Gate dispatch completes with failure outcome");

    assert!(!res.success);
    assert!(res.evidence.is_none());
    assert_eq!(res.output["status"], "failed");
    let receipt = res.receipt.unwrap();
    assert_eq!(receipt.status, ReceiptStatus::Failure);
}

#[tokio::test]
async fn test_high_risk_action_requires_approval() {
    let gate = DeterministicGate::default();
    let action = Action::new(
        "act_del_1".into(),
        "delete_file".into(),
        "src/main.rs".into(),
        serde_json::json!({"path": "src/main.rs"}),
        RiskLevel::High,
    );

    let err = gate
        .dispatch_for_task("task_del", &action, "developer")
        .await
        .unwrap_err();

    match err {
        DomainError::Conflict(msg) => {
            assert!(msg.contains("requires human approval"));
        }
        other => panic!("Expected Conflict error requiring approval, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_critical_risk_action_denied() {
    let gate = DeterministicGate::default();
    let action = Action::new(
        "act_drop_1".into(),
        "drop_database".into(),
        "production.db".into(),
        serde_json::json!({"force": true}),
        RiskLevel::Critical,
    );

    let err = gate
        .dispatch_for_task("task_crit", &action, "developer")
        .await
        .unwrap_err();

    match err {
        DomainError::Unauthorized(msg) => {
            assert!(msg.contains("denied"));
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}
