use super::*;
use custos_domain::{new_id, Assurance, RiskLevel};

#[tokio::test]
async fn test_generic_cli_adapter_mock_mode() {
    let temp_dir =
        std::env::temp_dir().join(format!("custos_cli_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let adapter = CliHarness::claude_code(&temp_dir).with_mock_mode();
    let res = adapter
        .run_native("compile project", &temp_dir)
        .await
        .expect("Mock execution succeeds");

    assert!(res.success);
    assert_eq!(res.exit_code, Some(0));
    assert!(res.stdout.contains("cargo check"));
    assert_eq!(res.observed_effects.len(), 2);
    assert_eq!(res.observed_effects[0].name, "shell_exec");
    assert_eq!(res.observed_effects[1].name, "file_write");
}

#[tokio::test]
async fn test_generic_cli_profile_and_assurance_invariants() {
    let temp_dir =
        std::env::temp_dir().join(format!("custos_cli_test_{}", uuid::Uuid::new_v4()));
    let adapter = CliHarness::claude_code(&temp_dir);
    let profile = adapter.profile();

    assert_eq!(profile.harness_id, "claude-code");
    assert_eq!(profile.tool_mediation, ToolMediationLevel::ProviderGoverned);
    assert!(!profile.is_custos_governed());
    assert!(profile.supports_cancel);
    assert!(!profile.supports_steer);
    assert_eq!(profile.cost_visibility, CostVisibility::Estimated);

    let codex = CliHarness::codex(&temp_dir);
    assert_eq!(codex.profile().cost_visibility, CostVisibility::ExactTokens);

    let opencode = CliHarness::opencode(&temp_dir);
    assert_eq!(
        opencode.profile().cost_visibility,
        CostVisibility::Estimated
    );
}

#[tokio::test]
async fn test_generic_cli_gate_4_fraud_rejection() {
    let temp_dir =
        std::env::temp_dir().join(format!("custos_cli_test_{}", uuid::Uuid::new_v4()));
    let adapter = CliHarness::claude_code(&temp_dir).with_mock_mode();
    let profile = adapter.profile();

    let fraudulent_intent = ActionIntent::new(
        new_id("act_fake"),
        "shell_exec".into(),
        "echo 1".into(),
        serde_json::json!({}),
        RiskLevel::High,
    )
    .with_assurance(Assurance::CustosMediated);

    let res = profile.validate_intent_assurance(&fraudulent_intent);
    assert!(
        res.is_err(),
        "Gate 4 must reject fraudulent CustosMediated assurance"
    );
    let err = res.unwrap_err();
    assert!(err.to_string().contains("Gate 4 Violation"));
}

#[tokio::test]
async fn test_generic_cli_cancel_and_steer() {
    let temp_dir =
        std::env::temp_dir().join(format!("custos_cli_test_{}", uuid::Uuid::new_v4()));
    let adapter = CliHarness::claude_code(&temp_dir).with_mock_mode();

    let run_id = "run_test_123";
    adapter.cancel_run(run_id).await.unwrap();

    adapter
        .steer_run(run_id, "Avoid modifying src/main.rs")
        .await
        .unwrap();
    let steers = adapter.steer_messages.lock().await;
    assert_eq!(
        steers.get(run_id).unwrap(),
        &vec!["Avoid modifying src/main.rs"]
    );
    drop(steers);

    let mut worker_run = WorkerRun::new("task_001".into(), "claude-code".into(), 1);
    worker_run.run_id = Some(run_id.to_string());
    let pack = ContextPack::new("pack_001".into(), vec![], 0, "sha256:test".into());

    let effects = adapter.execute_turn(&worker_run, &pack).await.unwrap();
    assert_eq!(effects.len(), 2);

    let steers_after = adapter.steer_messages.lock().await;
    assert!(steers_after.get(run_id).is_none());
}

#[tokio::test]
async fn test_generic_cli_live_binary_missing_error() {
    let temp_dir =
        std::env::temp_dir().join(format!("custos_cli_test_{}", uuid::Uuid::new_v4()));
    let adapter = CliHarness::new(
        "nonexistent-agent",
        "nonexistent_bin_12345",
        &temp_dir,
        ToolMediationLevel::ProviderGoverned,
    );

    struct MockObserver;
    impl custos_core::contracts::harness::RunObserver for MockObserver {
        fn emit(&self, _kind: &str, _payload: serde_json::Value) {}
        fn is_cancelled(&self) -> bool {
            false
        }
    }

    let res = adapter
        .run_native_observed("test", &temp_dir, Arc::new(MockObserver))
        .await;
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("is not installed or not found on PATH"));
}

