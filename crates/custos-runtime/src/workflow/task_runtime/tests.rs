use super::*;

#[tokio::test]
async fn cancel_only_changes_workers_of_the_selected_run() {
    use custos_core::contracts::storage::RunPort;
    let store = std::sync::Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task = custos_core::TaskService::new(store.clone())
        .create_task("Cancel scope".into())
        .await
        .unwrap();
    let mut first = custos_domain::Run::new(task.id.clone(), 1);
    let mut second = custos_domain::Run::new(task.id.clone(), 2);
    first.transition(custos_domain::RunStatus::Active).unwrap();
    second.transition(custos_domain::RunStatus::Active).unwrap();
    store.save_run(&mut first).await.unwrap();
    store.save_run(&mut second).await.unwrap();
    let mut worker =
        custos_domain::WorkerRun::new(task.id.clone(), "first".into(), 3).with_run_id(&first.id);
    let mut peer =
        custos_domain::WorkerRun::new(task.id, "second".into(), 3).with_run_id(&second.id);
    worker.transition(custos_domain::RunStatus::Active).unwrap();
    peer.transition(custos_domain::RunStatus::Active).unwrap();
    store.save_worker_run(&mut worker).await.unwrap();
    store.save_worker_run(&mut peer).await.unwrap();
    let runtime = TaskRuntime::new().with_run_store(store.clone());
    custos_core::contracts::workflow::WorkflowPort::request_cancel(
        &runtime,
        &first.id,
        "user stop",
    )
    .await
    .unwrap();
    assert_eq!(
        store
            .get_worker_run(&worker.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        custos_domain::RunStatus::Cancelled
    );
    assert_eq!(
        store
            .get_worker_run(&peer.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        custos_domain::RunStatus::Active
    );
    assert_eq!(
        store
            .get_run(&second.id)
            .await
            .unwrap()
            .unwrap()
            .state_version,
        second.state_version
    );
}
use custos_core::contracts::harness::{HarnessExecutionResult, HarnessProfile, ToolMediationLevel};
use custos_domain::{ActionIntent, ContextPack};
use std::path::Path;
use std::sync::Mutex;

struct RecordingHarness {
    id: String,
    calls: Arc<Mutex<Vec<(String, String)>>>,
}

#[async_trait]
impl AgentRuntimePort for RecordingHarness {
    fn harness_id(&self) -> &str {
        &self.id
    }

    fn profile(&self) -> HarnessProfile {
        HarnessProfile::new(&self.id, ToolMediationLevel::ProviderGoverned)
    }

    async fn execute_turn(
        &self,
        _worker_run: &WorkerRun,
        _context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        Ok(Vec::new())
    }

    async fn run_native(
        &self,
        instruction: &str,
        cwd: &Path,
    ) -> Result<HarnessExecutionResult, DomainError> {
        self.calls
            .lock()
            .map_err(|_| DomainError::Validation("recording harness lock poisoned".into()))?
            .push((instruction.to_string(), cwd.display().to_string()));
        Ok(HarnessExecutionResult {
            success: true,
            exit_code: Some(0),
            stdout: format!("{} completed", self.id),
            stderr: String::new(),
            observed_effects: Vec::new(),
            mediation_level: ToolMediationLevel::ProviderGoverned,
        })
    }
}

#[tokio::test]
async fn test_task_runtime_lifecycle() {
    let runtime = TaskRuntime::new();
    let cmd = StartRunCommand::new("task_test_123", "test_actor");
    let handle = runtime.start_run(cmd).await.unwrap();
    assert_eq!(handle.task_id, "task_test_123");
    assert_eq!(handle.status, RunStatus::Active);

    // Checkpoint
    let cp = runtime.checkpoint(&handle.run_id).await.unwrap();
    assert_eq!(cp.task_id, "task_test_123");

    // Cancel
    let receipt = runtime
        .request_cancel(&handle.run_id, "user request")
        .await
        .unwrap();
    assert_eq!(receipt.run_id, handle.run_id);
    assert_eq!(receipt.reason, "user request");
}

#[tokio::test]
async fn test_task_runtime_double_dispatch_fencing() {
    let runtime = TaskRuntime::new();
    let cmd1 = StartRunCommand::new("task_fence_01", "actor_alpha");
    let handle1 = runtime
        .start_run(cmd1)
        .await
        .expect("First run should start");

    // Second start_run on the SAME task before completion/cancellation must be refused!
    let cmd2 = StartRunCommand::new("task_fence_01", "actor_beta");
    let err2 = runtime
        .start_run(cmd2)
        .await
        .expect_err("Double dispatch must be refused");

    match err2 {
        DomainError::Conflict(msg) => {
            assert!(msg.contains("task_fence_01"));
            assert!(msg.contains("actor_alpha"));
        }
        other => panic!("Expected Conflict, got {:?}", other),
    }

    // After cancelling run 1, claiming should succeed again
    runtime
        .request_cancel(&handle1.run_id, "cancelled for test")
        .await
        .unwrap();
    let cmd3 = StartRunCommand::new("task_fence_01", "actor_beta");
    let handle3 = runtime
        .start_run(cmd3)
        .await
        .expect("Run should succeed after cancellation");
    assert_eq!(handle3.task_id, "task_fence_01");
}

#[tokio::test]
async fn test_task_runtime_candidate_scoped_dispatch_fencing() {
    let runtime = TaskRuntime::new();

    // 1. Candidate 1 (Claude) starts on task_race_01
    let cmd_cand1 =
        StartRunCommand::new("task_race_01", "actor_alpha").with_node_id("candidate:cand_claude");
    let handle1 = runtime
        .start_run(cmd_cand1)
        .await
        .expect("Candidate 1 should start without contention");

    // 2. Candidate 2 (GPT-4o) starts concurrently on the SAME task_race_01
    let cmd_cand2 =
        StartRunCommand::new("task_race_01", "actor_beta").with_node_id("candidate:cand_gpt4o");
    let handle2 = runtime
        .start_run(cmd_cand2)
        .await
        .expect("Candidate 2 should start concurrently on distinct candidate node_id");

    assert_eq!(handle1.task_id, "task_race_01");
    assert_eq!(handle2.task_id, "task_race_01");
    assert_ne!(handle1.run_id, handle2.run_id);

    // Verify metadata records node_id
    let runs = runtime.active_runs.read().await;
    let r1 = runs.get(&handle1.run_id).unwrap();
    let r2 = runs.get(&handle2.run_id).unwrap();
    assert_eq!(r1.metadata["node_id"], "candidate:cand_claude");
    assert_eq!(r2.metadata["node_id"], "candidate:cand_gpt4o");
    drop(runs);

    // 3. Duplicate candidate dispatch on cand_claude MUST be refused with Conflict
    let cmd_cand1_dup =
        StartRunCommand::new("task_race_01", "actor_gamma").with_node_id("candidate:cand_claude");
    let err = runtime
        .start_run(cmd_cand1_dup)
        .await
        .expect_err("Duplicate candidate claim must be refused");

    match err {
        DomainError::Conflict(msg) => {
            assert!(msg.contains("task_race_01"));
            assert!(msg.contains("candidate:cand_claude"));
        }
        other => panic!("Expected DomainError::Conflict, got {:?}", other),
    }
}

#[tokio::test]
async fn test_task_runtime_mode_negotiation_downgrade_metadata() {
    let runtime = TaskRuntime::new();
    let cmd = StartRunCommand::new("task_mode_01", "actor_alpha").with_preferred_mode("native");

    let handle = runtime.start_run(cmd).await.unwrap();
    let runs = runtime.active_runs.read().await;
    let run = runs.get(&handle.run_id).unwrap();

    // Since no harness was configured on this TaskRuntime, it recorded downgrade
    assert_eq!(run.metadata["requested_mode"], "native");
    assert_eq!(run.metadata["actual_mode"], "none");
    assert_eq!(run.metadata["mode_downgraded"], true);
}

struct SlowStreamModel;
#[async_trait]
impl ModelPort for SlowStreamModel {
    fn provider_id(&self) -> &str {
        "slow"
    }
    async fn generate(
        &self,
        _: &ProviderRequest,
    ) -> Result<custos_provider::ModelResponse, DomainError> {
        Err(DomainError::Validation("Streaming only".into()))
    }
    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<custos_provider::ProviderEvent>, DomainError> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let id = req.request_id.clone();
        tokio::spawn(async move {
            tx.send(custos_provider::ProviderEvent::chunk(&id, "partial"))
                .await
                .unwrap();
            tokio::select! {_=tx.closed()=>{},_=tokio::time::sleep(std::time::Duration::from_secs(30))=>{}}
        });
        Ok(rx)
    }
}
#[tokio::test]
async fn background_returns_early_and_stop_preserves_partial() {
    let runtime = TaskRuntime::new().with_model(Arc::new(SlowStreamModel));
    let cmd = StartRunCommand::new("background-task", "chat")
        .with_preferred_mode("model")
        .with_prompt("hello")
        .with_session_id("session")
        .with_turn_id("turn");
    let handle = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        runtime.start_run_background(cmd),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(handle.status, RunStatus::Active);
    for _ in 0..100 {
        if runtime
            .live_run("session", &handle.run_id, 0)
            .await
            .unwrap()
            .snapshot
            .text
            == "partial"
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert_eq!(
        runtime
            .live_run("session", &handle.run_id, 0)
            .await
            .unwrap()
            .snapshot
            .text,
        "partial"
    );
    runtime
        .request_cancel(&handle.run_id, "test")
        .await
        .unwrap();
    for _ in 0..100 {
        if runtime
            .live_run("session", &handle.run_id, 0)
            .await
            .unwrap()
            .snapshot
            .status
            == "cancelled"
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let snapshot = runtime
        .live_run("session", &handle.run_id, 0)
        .await
        .unwrap()
        .snapshot;
    assert_eq!(snapshot.status, "cancelled");
    assert_eq!(snapshot.text, "partial");
}

#[tokio::test]
async fn durable_background_result_restores_without_duplicate_journal() {
    use custos_core::contracts::storage::RunPort;
    use custos_core::TaskStore;
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task = custos_domain::Task::new("durable-live-task".into(), "Durable live".into());
    store.save_task(&task).await.unwrap();
    let sessions = Arc::new(crate::session::SessionManager::with_store(store.clone()));
    let session = sessions
        .create_session(custos_domain::SessionMode::Assisted)
        .await;
    let runtime = TaskRuntime::new()
        .with_run_store(store.clone())
        .with_model(Arc::new(TestMockModel))
        .with_journal_manager(sessions.clone());
    let cmd = StartRunCommand::new(&task.id, "chat")
        .with_preferred_mode("model")
        .with_prompt("hello")
        .with_session_id(session.id.to_string())
        .with_turn_id("durable-turn");
    let handle = runtime.start_run_background(cmd).await.unwrap();
    for _ in 0..100 {
        if runtime
            .live_run(&session.id.to_string(), &handle.run_id, 0)
            .await
            .unwrap()
            .snapshot
            .status
            == "completed"
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let run = store.get_run(&handle.run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);
    assert_eq!(run.metadata["background"], true);
    assert!(run.metadata["journal_occurred_at"].is_string());
    let restarted = TaskRuntime::new().with_run_store(store.clone());
    let snapshot = restarted
        .live_run(&session.id.to_string(), &handle.run_id, 99)
        .await
        .unwrap();
    assert!(snapshot.gap);
    assert_eq!(snapshot.snapshot.status, "completed");
    assert!(snapshot.snapshot.text.contains("Completed turn response"));
    assert_eq!(
        sessions
            .get_journal(&session.id)
            .await
            .unwrap()
            .entries
            .iter()
            .filter(|e| e.entry_type == "assistant_message")
            .count(),
        1
    );
    let mut interrupted = custos_domain::Run::new(task.id.clone(), 2);
    interrupted.transition(RunStatus::Active).unwrap();
    interrupted.metadata = serde_json::json!({"session_id":session.id,"background":true});
    store.save_run(&mut interrupted).await.unwrap();
    assert_eq!(
        restarted
            .live_run(&session.id.to_string(), &interrupted.id, 0)
            .await
            .unwrap()
            .snapshot
            .status,
        "interrupted"
    );
}

#[tokio::test]
async fn live_capacity_rejection_releases_dispatch_claim() {
    let runtime = TaskRuntime::new().with_model(Arc::new(TestMockModel));
    for _ in 0..256 {
        let run = Run::new("retained".into(), 1);
        runtime
            .live
            .register(&run, &StartRunCommand::new("retained", "chat"))
            .unwrap();
    }
    assert!(runtime
        .start_run_background(
            StartRunCommand::new("capacity-task", "chat")
                .with_preferred_mode("model")
                .with_prompt("hello")
        )
        .await
        .is_err());
    assert!(runtime
        .dispatcher
        .get_active_claim("capacity-task", None)
        .await
        .is_none());
}

#[tokio::test]
async fn legacy_worker_without_chat_identity_remains_active() {
    let runtime = TaskRuntime::new().with_model(Arc::new(TestMockModel));
    let handle = runtime
        .start_run(
            StartRunCommand::new("legacy-worker", "worker")
                .with_preferred_mode("model")
                .with_prompt("hello"),
        )
        .await
        .unwrap();
    assert_eq!(handle.status, RunStatus::Active);
    runtime
        .request_cancel(&handle.run_id, "test cleanup")
        .await
        .unwrap();
}

#[tokio::test]
async fn rejected_background_mode_does_not_persist_a_phantom_run() {
    use custos_core::{contracts::storage::RunPort, TaskStore};
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task = custos_domain::Task::new("unconfigured-task".into(), "Unconfigured".into());
    store.save_task(&task).await.unwrap();
    let runtime = TaskRuntime::new().with_run_store(store.clone());
    assert!(runtime
        .start_run_background(StartRunCommand::new(&task.id, "chat").with_preferred_mode("model"))
        .await
        .is_err());
    assert!(store.list_runs_for_task(&task.id).await.unwrap().is_empty());
}

struct TestMockModel;

#[async_trait]
impl ModelPort for TestMockModel {
    fn provider_id(&self) -> &str {
        "test_model"
    }

    async fn generate(
        &self,
        req: &ProviderRequest,
    ) -> Result<custos_provider::request::ModelResponse, DomainError> {
        Ok(custos_provider::request::ModelResponse::text(
            format!(
                "Completed turn response for task {} with prompt: {}",
                req.task_id, req.prompt
            ),
            "test_model",
            42,
        ))
    }
}

#[tokio::test]
async fn test_task_runtime_model_turn_generates_and_completes() {
    let model = Arc::new(TestMockModel);
    let runtime = TaskRuntime::new().with_model(model);
    let cmd = StartRunCommand::new("task_chat_01", "actor_user")
        .with_preferred_mode("model")
        .with_prompt("Help me build the mobile simulator");

    let handle = runtime.start_run(cmd).await.unwrap();
    assert_eq!(handle.task_id, "task_chat_01");
    assert_eq!(handle.status, RunStatus::Completed);
    assert!(handle.completed_at.is_some());
    assert!(handle.output.is_some());
    let output = handle.output.unwrap();
    assert!(output.contains("Completed turn response for task task_chat_01"));
    assert!(output.contains("Help me build the mobile simulator"));

    let runs = runtime.active_runs.read().await;
    let run = runs.get(&handle.run_id).unwrap();
    assert_eq!(run.status, RunStatus::Completed);
    assert_eq!(run.metadata["actual_mode"], "model");
    assert_eq!(run.metadata["tokens_used"], 42);
    assert!(run.metadata["launch_attempt_id"].is_string());
    assert_eq!(run.metadata["completion_reason"], "model_turn_completed");
    drop(runs);

    // A completed model-only turn releases its claim, so a follow-up can reuse the task.
    let cmd2 = StartRunCommand::new("task_chat_01", "actor_user")
        .with_preferred_mode("model")
        .with_prompt("Second turn follow-up");
    let handle2 = runtime
        .start_run(cmd2)
        .await
        .expect("Follow-up model turn should succeed after claim release");
    assert_eq!(handle2.status, RunStatus::Completed);
}

#[tokio::test]
async fn test_task_runtime_recovers_persisted_completed_chat_turn_claim() {
    use custos_core::contracts::kernel::TrustedKernel;
    use custos_core::TaskStore;
    use custos_domain::{DispatchClaim, Task};
    use custos_persistence::SqliteTaskStore;

    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let mut task = Task::new(
        "task_chat_recover_01".into(),
        "Recover completed chat turn".into(),
    );
    task.metadata = serde_json::json!({ "kind": "conversation_chat_turn" });
    store.save_task(&task).await.unwrap();

    let mut stale_run = Run::new(task.id.clone(), 1);
    stale_run.transition(RunStatus::Active).unwrap();
    stale_run.metadata["actual_mode"] = serde_json::json!("model");
    stale_run.metadata["output"] = serde_json::json!("already completed output");
    store.save_run(&mut stale_run).await.unwrap();

    let mut stale_worker = WorkerRun::new(task.id.clone(), "daemon_user".into(), 3);
    stale_worker.run_id = Some(stale_run.id.clone());
    stale_worker.transition(RunStatus::Active).unwrap();
    store.save_worker_run(&mut stale_worker).await.unwrap();

    let mut stale_claim = DispatchClaim::new(&task.id, "daemon_user", 1);
    stale_claim.mark_dispatched(&stale_worker.id).unwrap();
    store.claim_ready(&stale_claim).await.unwrap();

    let kernel = Arc::new(TrustedKernel::new(store.clone()));
    let model = Arc::new(TestMockModel);
    let runtime = TaskRuntime::new()
        .with_kernel(kernel)
        .with_model(model)
        .with_run_store(store.clone());

    let handle = runtime
        .start_run(
            StartRunCommand::new(&task.id, "actor_user")
                .with_preferred_mode("model")
                .with_prompt("follow-up after stale claim"),
        )
        .await
        .expect("stale completed model chat turn should be recovered before claiming");

    assert_eq!(handle.status, RunStatus::Completed);
    let stale_run_after = store.get_run(&stale_run.id).await.unwrap().unwrap();
    assert_eq!(stale_run_after.status, RunStatus::Completed);
    assert_eq!(
        stale_run_after.metadata["completion_reason"],
        "recovered_model_turn_completed"
    );
    let stale_worker_after = store
        .get_worker_run(&stale_worker.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stale_worker_after.status, RunStatus::Completed);
    assert!(store
        .get_active_claim(&task.id, None)
        .await
        .unwrap()
        .is_none());
}

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
async fn test_native_turn_approval_conflict_suspends_run_and_blocks_task() {
    use custos_core::contracts::harness::{AgentRuntimePort, HarnessProfile, ToolMediationLevel};
    use custos_core::contracts::kernel::KernelPort;
    use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};
    use custos_domain::{
        Action, ActionLifecycleState, Assurance, ContextPack, RiskLevel, TaskStatus, WorkerRun,
    };
    use tokio::sync::Mutex;

    struct ConflictHarness;
    #[async_trait::async_trait]
    impl AgentRuntimePort for ConflictHarness {
        fn harness_id(&self) -> &str {
            "conflict-harness"
        }
        fn profile(&self) -> HarnessProfile {
            HarnessProfile::new("conflict-harness", ToolMediationLevel::CustosMediated)
        }
        async fn execute_turn(
            &self,
            worker_run: &WorkerRun,
            _ctx: &ContextPack,
        ) -> Result<Vec<custos_domain::ActionIntent>, DomainError> {
            let mut intent = Action::new(
                "act_delete".into(),
                "delete_database".into(),
                "prod.db".into(),
                serde_json::json!({}),
                RiskLevel::High,
            );
            intent.task_id = Some(worker_run.task_id.clone());
            intent.assurance = Assurance::CustosMediated;
            intent.lifecycle_state = ActionLifecycleState::Intent;
            Ok(vec![intent])
        }
    }

    struct TaskStatusKernel {
        task_status: Arc<Mutex<TaskStatus>>,
    }
    #[async_trait::async_trait]
    impl KernelPort for TaskStatusKernel {
        async fn create_task(&self, _: String) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn create_task_with_contract(
            &self,
            _: String,
            _: custos_domain::TaskContract,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn get_task(
            &self,
            task_id: &str,
        ) -> Result<Option<custos_domain::Task>, DomainError> {
            let status = *self.task_status.lock().await;
            let mut t = custos_domain::Task::new(task_id.into(), "Conflict".into());
            t.status = status;
            Ok(Some(t))
        }
        async fn transition_task(
            &self,
            _: &str,
            next: TaskStatus,
        ) -> Result<custos_domain::Task, DomainError> {
            let mut status = self.task_status.lock().await;
            *status = next;
            let mut t = custos_domain::Task::new("task_conflict_02".into(), "Conflict".into());
            t.status = next;
            Ok(t)
        }
        async fn request_permit(
            &self,
            _: &str,
            _: &custos_domain::ActionIntent,
            _: &str,
        ) -> Result<custos_domain::Permit, DomainError> {
            Err(DomainError::Conflict(
                "Approval required for high risk delete".into(),
            ))
        }
        async fn consume_permit(
            &self,
            _: &str,
            _: &custos_domain::ActionIntent,
        ) -> Result<custos_domain::Permit, DomainError> {
            unreachable!()
        }
        async fn verify_evidence(
            &self,
            _: &custos_core::evidence::EvidenceBundle,
        ) -> Result<custos_domain::VerificationClaim, DomainError> {
            unreachable!()
        }
        async fn grant_capability(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<custos_domain::Grant, DomainError> {
            unreachable!()
        }
        async fn find_active_grant(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<custos_domain::Grant>, DomainError> {
            unreachable!()
        }
        async fn complete_task(
            &self,
            _: &str,
            _: String,
            _: Vec<custos_domain::VerificationClaim>,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
    }

    struct InMemoryOutbox {
        entries: Arc<Mutex<Vec<OutboxEntry>>>,
    }
    #[async_trait::async_trait]
    impl OutboxPort for InMemoryOutbox {
        async fn enqueue(&self, entry: OutboxEntry) -> Result<(), DomainError> {
            self.entries.lock().await.push(entry);
            Ok(())
        }
        async fn get(&self, id: &str) -> Result<Option<OutboxEntry>, DomainError> {
            Ok(self
                .entries
                .lock()
                .await
                .iter()
                .find(|e| e.id == id)
                .cloned())
        }
        async fn list_by_status(
            &self,
            status: OutboxStatus,
        ) -> Result<Vec<OutboxEntry>, DomainError> {
            Ok(self
                .entries
                .lock()
                .await
                .iter()
                .filter(|e| e.status == status)
                .cloned()
                .collect())
        }
        async fn list_by_task_and_status(
            &self,
            task_id: &str,
            status: OutboxStatus,
        ) -> Result<Vec<OutboxEntry>, DomainError> {
            Ok(self
                .entries
                .lock()
                .await
                .iter()
                .filter(|e| e.task_id == task_id && e.status == status)
                .cloned()
                .collect())
        }
        async fn mark_approved(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn mark_dispatching(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn mark_receipted(
            &self,
            _: &str,
            _: custos_domain::Receipt,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn mark_uncertain(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn mark_failed(&self, _: &str, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task = custos_domain::Task::new("task_conflict_02".into(), "Conflict Test".into());
    custos_core::TaskStore::save_task(&*store, &task)
        .await
        .unwrap();

    let task_status = Arc::new(Mutex::new(TaskStatus::Running));
    let kernel = Arc::new(TaskStatusKernel {
        task_status: task_status.clone(),
    });
    let outbox_entries = Arc::new(Mutex::new(Vec::new()));
    let outbox = Arc::new(InMemoryOutbox {
        entries: outbox_entries.clone(),
    });

    let runtime = TaskRuntime::new()
        .with_harness(Arc::new(ConflictHarness))
        .with_kernel(kernel)
        .with_outbox(outbox)
        .with_run_store(store);

    let cmd = StartRunCommand::new(&task.id, "actor_user")
        .with_session_id("session_conflict_02")
        .with_preferred_mode("native")
        .with_harness_id("conflict-harness")
        .with_workspace_root("/tmp/custos-conflict-test");

    let handle = runtime.start_run(cmd).await.unwrap();
    // Crucial regression assert: MUST NOT be Completed!
    assert_eq!(
        handle.status,
        RunStatus::Suspended,
        "Pending approval must suspend run, not complete it"
    );
    assert_eq!(
        *task_status.lock().await,
        TaskStatus::Blocked,
        "Task must transition to Blocked"
    );

    {
        let entries = outbox_entries.lock().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, OutboxStatus::Pending);
    }

    // Test 1: live_run replay after daemon restart must preserve suspended status, not report "interrupted"
    let replay = runtime
        .live_run("session_conflict_02", &handle.run_id, 0)
        .await
        .unwrap();
    assert_eq!(
        replay.snapshot.status, "suspended",
        "Replay must preserve suspended status"
    );
    assert_eq!(
        replay.snapshot.error,
        Some("Approval required for high risk delete".into()),
        "Replay must preserve pending approval reason"
    );

    // Test 2: resume before approval must fail with conflict
    let early_resume = runtime.resume(&handle.run_id).await;
    assert!(
        early_resume.is_err(),
        "Resume must fail while approval is still pending"
    );
    assert!(matches!(
        early_resume.unwrap_err(),
        DomainError::Conflict(_)
    ));

    // Test 3: mark approval granted -> resume transitions to Active and task to Running
    outbox_entries.lock().await[0].status = OutboxStatus::Approved;
    let resumed = runtime.resume(&handle.run_id).await.unwrap();
    assert_eq!(
        resumed.status,
        RunStatus::Active,
        "Run must become Active upon approval resume"
    );
    assert_eq!(
        *task_status.lock().await,
        TaskStatus::Running,
        "Task must transition to Running upon approval resume"
    );

    // Test 4: background continuation executor completes the approved execution
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        *task_status.lock().await,
        TaskStatus::Succeeded,
        "Task must succeed after resumed approval execution"
    );
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
