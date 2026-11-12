use super::*;
use async_trait::async_trait;
use custos_domain::StartRunCommand;
use custos_provider::request::ProviderRequest;

mod approval_conflict;
mod background;


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

pub(super) struct RecordingHarness {
    pub(super) id: String,
    pub(super) calls: Arc<Mutex<Vec<(String, String)>>>,
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

pub(super) struct TestMockModel;

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


mod turns;
