use super::*;
use async_trait::async_trait;
use custos_domain::StartRunCommand;
use custos_provider::request::ProviderRequest;

struct MeasuredAccountModel;
#[async_trait]
impl ModelPort for MeasuredAccountModel {
    fn provider_id(&self) -> &str {
        "test"
    }
    async fn generate(
        &self,
        req: &ProviderRequest,
    ) -> Result<custos_provider::ModelResponse, DomainError> {
        assert_eq!(req.provider_account_id.as_deref(), Some("account-A"));
        assert_eq!(req.reasoning_effort.as_deref(), Some("ultra"));
        Ok(
            custos_provider::ModelResponse::text("answer", "ollama/test", 10).with_usage(
                Some(7),
                Some(3),
                Some(10),
            ),
        )
    }
}
#[tokio::test]
async fn measured_usage_and_account_are_persisted_with_model_run() {
    use custos_core::contracts::kernel::TrustedKernel;
    use custos_core::TaskStore;
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let mut task = custos_domain::Task::new("usage-task".into(), "Measured usage".into());
    task.metadata = serde_json::json!({"provider_account_id":"account-A"});
    store.save_task(&task).await.unwrap();
    let runtime = TaskRuntime::new()
        .with_kernel(Arc::new(TrustedKernel::new(store.clone())))
        .with_model(Arc::new(MeasuredAccountModel))
        .with_run_store(store.clone());
    let mut command = StartRunCommand::new(&task.id, "actor")
        .with_preferred_mode("model")
        .with_model("ollama/test")
        .with_prompt("hello");
    command.reasoning_effort = Some("ultra".into());
    command.provider_account_id = Some("account-A".into());
    let handle = runtime.start_run(command).await.unwrap();
    drop(runtime);
    let run = store.get_run(&handle.run_id).await.unwrap().unwrap();
    assert_eq!(run.metadata["provider_account_id"], "account-A");
    assert_eq!(run.metadata["usage"]["total_tokens"], 10);
    assert_eq!(run.metadata["usage"]["input_tokens"], 7);
    assert_eq!(run.metadata["requested_model"], "ollama/test");
    assert_eq!(run.metadata["reasoning_effort"], "ultra");
}

struct TestFailingModel;

#[async_trait]
impl custos_provider::ModelPort for TestFailingModel {
    fn provider_id(&self) -> &str {
        "test-fail"
    }

    async fn generate(
        &self,
        _req: &ProviderRequest,
    ) -> Result<custos_provider::request::ModelResponse, DomainError> {
        Err(DomainError::Validation(
            "Simulated model provider failure".into(),
        ))
    }
}

#[tokio::test]
async fn test_task_runtime_failed_model_turn_transitions_to_failed_and_releases_lock() {
    use custos_core::contracts::kernel::TrustedKernel;
    use custos_core::TaskStore;
    use custos_domain::Task;
    use custos_persistence::SqliteTaskStore;

    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let task = Task::new("task_fail_recover_01".into(), "Fail and retry task".into());
    store.save_task(&task).await.unwrap();

    let kernel = Arc::new(TrustedKernel::new(store.clone()));
    let model = Arc::new(TestFailingModel);
    let runtime = TaskRuntime::new()
        .with_kernel(kernel)
        .with_model(model)
        .with_run_store(store.clone());

    // First run fails due to model error
    let err = runtime
        .start_run(
            StartRunCommand::new(&task.id, "actor_user")
                .with_preferred_mode("model")
                .with_prompt("failing prompt"),
        )
        .await
        .expect_err("Should fail with model error");
    assert!(matches!(err, DomainError::Validation(_)));

    // Verify worker run in store is Failed, not left Active
    let wruns = store.list_worker_runs_for_task(&task.id).await.unwrap();
    assert_eq!(wruns.len(), 1);
    assert_eq!(wruns[0].status, RunStatus::Failed);

    let runs = store.list_runs_for_task(&task.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, RunStatus::Failed);

    // Second run can immediately proceed without Conflict error
    let working_model = Arc::new(TestMockModel);
    let runtime2 = TaskRuntime::new()
        .with_model(working_model)
        .with_run_store(store.clone());

    let handle2 = runtime2
        .start_run(
            StartRunCommand::new(&task.id, "actor_user")
                .with_preferred_mode("model")
                .with_prompt("retry prompt"),
        )
        .await
        .expect("Retry turn should succeed without Conflict error");
    assert_eq!(handle2.status, RunStatus::Completed);
}

#[tokio::test]
async fn native_run_uses_requested_harness_and_workspace() {
    let alpha_calls = Arc::new(Mutex::new(Vec::new()));
    let beta_calls = Arc::new(Mutex::new(Vec::new()));
    let mut harnesses: HashMap<String, Arc<dyn AgentRuntimePort>> = HashMap::new();
    harnesses.insert(
        "alpha".into(),
        Arc::new(RecordingHarness {
            id: "alpha".into(),
            calls: alpha_calls.clone(),
        }),
    );
    harnesses.insert(
        "beta".into(),
        Arc::new(RecordingHarness {
            id: "beta".into(),
            calls: beta_calls.clone(),
        }),
    );

    let runtime = TaskRuntime::new().with_harnesses(harnesses, Some("alpha"));
    let command = StartRunCommand::new("task_native", "actor_user")
        .with_preferred_mode("native")
        .with_harness_id("beta")
        .with_workspace_root("/tmp/custos-native-test")
        .with_prompt("inspect the repository");

    let handle = runtime.start_run(command).await.unwrap();
    assert_eq!(handle.status, RunStatus::Completed);
    assert_eq!(handle.output.as_deref(), Some("beta completed"));
    assert!(alpha_calls.lock().unwrap().is_empty());
    assert_eq!(
        beta_calls.lock().unwrap().as_slice(),
        &[(
            "inspect the repository".to_string(),
            "/tmp/custos-native-test".to_string()
        )]
    );
}

#[tokio::test]
async fn test_terminal_run_evicts_from_active_maps_to_persistence() {
    use custos_core::TaskStore;
    use custos_domain::Task;
    use custos_persistence::SqliteTaskStore;

    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let task = Task::new("task_evict_01".into(), "Evict test".into());
    store.save_task(&task).await.unwrap();

    let model = Arc::new(TestMockModel);
    let runtime = TaskRuntime::new()
        .with_model(model)
        .with_run_store(store.clone());

    let cmd = StartRunCommand::new(&task.id, "actor_user")
        .with_preferred_mode("model")
        .with_prompt("Help me test eviction");

    let handle = runtime.start_run(cmd).await.unwrap();
    assert_eq!(handle.status, RunStatus::Completed);

    // Terminal run must be evicted from active memory
    let active = runtime.active_runs.read().await;
    assert!(
        active.get(&handle.run_id).is_none(),
        "Terminal run must not linger in active_runs"
    );
    drop(active);

    // But must be durably resolvable via checkpoint/run_store
    let checkpoint = runtime.checkpoint(&handle.run_id).await.unwrap();
    assert_eq!(checkpoint.task_id, task.id);
}

#[tokio::test]
async fn test_tool_receipt_not_reexecuted_double_execution_prevention() {
    use custos_core::contracts::harness::{AgentRuntimePort, HarnessProfile, ToolMediationLevel};
    use custos_domain::{
        Action, ActionLifecycleState, Assurance, ContextPack, RiskLevel, WorkerRun,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    let execution_count = Arc::new(AtomicUsize::new(0));
    let exec_count_clone = execution_count.clone();

    struct ReceiptEmittingHarness {
        exec_count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl AgentRuntimePort for ReceiptEmittingHarness {
        fn harness_id(&self) -> &str {
            "receipt-harness"
        }
        fn profile(&self) -> HarnessProfile {
            HarnessProfile::new("receipt-harness", ToolMediationLevel::CustosMediated)
        }
        async fn execute_turn(
            &self,
            worker_run: &WorkerRun,
            _ctx: &ContextPack,
        ) -> Result<Vec<custos_domain::ActionIntent>, DomainError> {
            self.exec_count.fetch_add(1, Ordering::SeqCst);
            let mut intent = Action::new(
                "act_executed".into(),
                "write_file".into(),
                "target.txt".into(),
                serde_json::json!({}),
                RiskLevel::Low,
            );
            intent.task_id = Some(worker_run.task_id.clone());
            intent.assurance = Assurance::CustosMediated;
            // Pre-mark as Receipt (already executed by GovernedToolExecutor in agent loop)
            intent.lifecycle_state = ActionLifecycleState::Receipt;
            Ok(vec![intent])
        }
    }

    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task = custos_domain::Task::new("task_receipt_01".into(), "Receipt Test".into());
    custos_core::TaskStore::save_task(&*store, &task)
        .await
        .unwrap();

    let harness = Arc::new(ReceiptEmittingHarness {
        exec_count: exec_count_clone,
    });
    let runtime = TaskRuntime::new()
        .with_harness(harness)
        .with_run_store(store);

    let cmd = StartRunCommand::new(&task.id, "actor_user")
        .with_preferred_mode("native")
        .with_harness_id("receipt-harness")
        .with_workspace_root("/tmp/custos-receipt-test");

    let handle = runtime.start_run(cmd).await.unwrap();
    assert_eq!(handle.status, RunStatus::Completed);
    // Harness executed once in loop
    assert_eq!(execution_count.load(Ordering::SeqCst), 1);
    // Intent had lifecycle_state = Receipt so it was NOT double-executed outside loop
}

#[tokio::test]
async fn test_task_runtime_execute_candidate_race() {
    use custos_domain::{CandidateRaceSpec, ModelCandidateConfig};
    let runtime = TaskRuntime::new();
    let candidates = vec![
        ModelCandidateConfig::new("cand_a", "model_alpha", "mock_harness"),
        ModelCandidateConfig::new("cand_b", "model_beta", "mock_harness"),
    ];
    let spec = CandidateRaceSpec::new("task_race_e2e", "ws_test_01", candidates);

    let result = runtime.execute_candidate_race(spec).await.unwrap();
    assert_eq!(result.task_id, "task_race_e2e");
    assert_eq!(result.candidate_reports.len(), 2);
    assert!(result.winner_candidate_id.is_some());
}
