use super::*;
use custos_domain::{Action, Permit, RiskClass, RiskLevel};

#[tokio::test]
async fn legitimate_new_nested_files_remain_readable() {
    let root = tempfile::tempdir().unwrap();
    let adapter = SovereignDeveloperAdapter::new(root.path());
    adapter
        .handle_write(&serde_json::json!({
            "path": "new/nested/file.txt", "content": "scoped content",
        }))
        .await
        .unwrap();
    let read = adapter
        .handle_read(&serde_json::json!({"path": "new/nested/file.txt"}))
        .await
        .unwrap();
    assert_eq!(read["content"], "scoped content");
    let absolute = root.path().join("new/nested/file.txt");
    assert!(adapter
        .handle_read(&serde_json::json!({"path": absolute}))
        .await
        .is_ok());
}

#[cfg(unix)]
#[tokio::test]
async fn new_file_below_escaping_symlink_is_rejected_before_creation() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let outside = dir.path().join("outside");
    tokio::fs::create_dir(&root).await.unwrap();
    tokio::fs::create_dir(&outside).await.unwrap();
    std::os::unix::fs::symlink(&outside, root.join("escape")).unwrap();
    let adapter = SovereignDeveloperAdapter::new(root);
    let result = adapter
        .handle_write(&serde_json::json!({
            "path": "escape/new/file.txt", "content": "must not be written",
        }))
        .await;
    assert!(matches!(result, Err(DomainError::Unauthorized(_))));
    assert!(!outside.join("new").exists());
}

#[tokio::test]
async fn test_sovereign_developer_adapter_read_write_flow() {
    let temp_dir =
        std::env::temp_dir().join(format!("custos_dev_test_{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&temp_dir).await.unwrap();

    let adapter = SovereignDeveloperAdapter::new(&temp_dir);

    let write_action = Action::new(
        "act_write_1".into(),
        "developer__write".into(),
        "hello.txt".into(),
        serde_json::json!({
            "path": "hello.txt",
            "content": "Sovereign Custos Execution",
        }),
        RiskLevel::Medium,
    )
    .with_task_id("task_1");

    let write_permit = Permit::new(
        "task_1".into(),
        "act_write_1".into(),
        "developer".into(),
        None,
        RiskClass::Medium,
        60,
    );

    let write_receipt = adapter.execute(&write_action, &write_permit).await.unwrap();
    assert_eq!(write_receipt.status, ReceiptStatus::Success);
    assert!(write_receipt.output_digest.starts_with("blake3:"));

    let read_action = Action::new(
        "act_read_1".into(),
        "developer__read".into(),
        "hello.txt".into(),
        serde_json::json!({
            "path": "hello.txt",
        }),
        RiskLevel::Low,
    )
    .with_task_id("task_1");

    let read_permit = Permit::new(
        "task_1".into(),
        "act_read_1".into(),
        "developer".into(),
        None,
        RiskClass::Low,
        60,
    );

    let read_receipt = adapter.execute(&read_action, &read_permit).await.unwrap();
    assert_eq!(read_receipt.status, ReceiptStatus::Success);
    let data = read_receipt.output_data.unwrap();
    assert_eq!(data["content"], "Sovereign Custos Execution");

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}

#[tokio::test]
async fn test_sovereign_developer_adapter_rejects_path_traversal() {
    let temp_dir =
        std::env::temp_dir().join(format!("custos_dev_test_{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&temp_dir).await.unwrap();

    let adapter = SovereignDeveloperAdapter::new(&temp_dir);

    let action = Action::new(
        "act_escape".into(),
        "developer__read".into(),
        "../../../etc/passwd".into(),
        serde_json::json!({
            "path": "../../../etc/passwd",
        }),
        RiskLevel::High,
    )
    .with_task_id("task_1");

    let permit = Permit::new(
        "task_1".into(),
        "act_escape".into(),
        "developer".into(),
        None,
        RiskClass::High,
        60,
    );

    let receipt = adapter.execute(&action, &permit).await.unwrap();
    assert_eq!(receipt.status, ReceiptStatus::Failure);
    assert!(receipt
        .error_message
        .unwrap()
        .contains("Path traversal violation"));

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}

