//! Execution Spine Vertical Slice (PR 1 - Gate 1 & 2)
//!
//! Reorganized into clear thematic test modules:
//! 1. `start_and_checkpoint_lifecycle`: Task creation, start_run, Running transition, and integrity checkpointing.
//! 2. `graceful_cancellation_lifecycle`: Cancellation request, zero uncertain side effects, and terminal cancellation transition.

use custos_adapters::custos_adapter_provider_fake::FakeProvider;
use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::contracts::workflow::WorkflowPort;
use custos_domain::{
    ContractEvidence, EvidenceKind, RunHandle, RunStatus, StartRunCommand, Task, TaskContract,
    TaskStatus,
};
use custos_persistence::SqliteTaskStore;
use custos_runtime::workflow::TaskRuntime;
use std::sync::Arc;

async fn setup_spine_fixture() -> (Arc<dyn KernelPort>, Arc<TaskRuntime>, Task) {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store));
    let model = Arc::new(FakeProvider::new("fake-model"));
    let runtime = Arc::new(TaskRuntime::with_ports(kernel.clone(), model));

    let contract = TaskContract {
        pack_id: "engineering".into(),
        name: "spine_verification".into(),
        description: "Test execution spine".into(),
        required_capabilities: vec!["read_repo".into()],
        evidence_requirements: vec![ContractEvidence {
            kind: EvidenceKind::FileAnchor,
            required: true,
        }],
    };
    let task = kernel
        .create_task_with_contract("Verify Execution Spine".into(), contract)
        .await
        .expect("task creation should succeed");

    (kernel, runtime, task)
}

// =========================================================================
// THEME 1: Start Run & Integrity Checkpoint Lifecycle
// =========================================================================
mod start_and_checkpoint_lifecycle {
    use super::*;

    #[tokio::test]
    async fn test_execution_spine_start_and_checkpoint() {
        let (kernel, runtime, task) = setup_spine_fixture().await;
        assert_eq!(task.status, TaskStatus::Draft);

        let start_cmd =
            StartRunCommand::new(&task.id, "orchestrator").with_workflow_revision("wf_rev_001");
        let run_handle: RunHandle = runtime
            .start_run(start_cmd)
            .await
            .expect("start_run must succeed");

        assert_eq!(run_handle.task_id, task.id);
        assert_eq!(run_handle.status, RunStatus::Active);

        // Verify Kernel state transitioned to Running
        let running_task = kernel
            .get_task(&task.id)
            .await
            .unwrap()
            .expect("task exists");
        assert_eq!(running_task.status, TaskStatus::Running);

        // Checkpoint via WorkflowPort
        let checkpoint = runtime
            .checkpoint(&run_handle.run_id)
            .await
            .expect("checkpoint should succeed");
        assert_eq!(checkpoint.task_id, task.id);
        assert!(!checkpoint.integrity_hash.is_empty());
    }
}

// =========================================================================
// THEME 2: Graceful Cancellation Lifecycle
// =========================================================================
mod graceful_cancellation_lifecycle {
    use super::*;

    #[tokio::test]
    async fn test_execution_spine_graceful_cancellation() {
        let (kernel, runtime, task) = setup_spine_fixture().await;

        let start_cmd =
            StartRunCommand::new(&task.id, "orchestrator").with_workflow_revision("wf_rev_001");
        let run_handle = runtime
            .start_run(start_cmd)
            .await
            .expect("start_run must succeed");

        // Request graceful cancellation
        let cancel_receipt = runtime
            .request_cancel(&run_handle.run_id, "User requested test halt")
            .await
            .expect("cancel should succeed");
        assert_eq!(cancel_receipt.run_id, run_handle.run_id);
        assert_eq!(cancel_receipt.uncertain_effects_count, 0);

        // Verify Task state in Kernel transitioned to Cancelled
        let cancelled_task = kernel
            .get_task(&task.id)
            .await
            .unwrap()
            .expect("task exists");
        assert_eq!(cancelled_task.status, TaskStatus::Cancelled);
    }
}

// =========================================================================
// THEME 3: Gate A Atomic Claim & Double-Dispatch Fencing (Orca Parity)
// =========================================================================
mod atomic_claim_and_fencing {
    use super::*;
    use custos_domain::DomainError;

    #[tokio::test]
    async fn test_gate_a_double_dispatch_fencing() {
        let (kernel, runtime, task) = setup_spine_fixture().await;

        // Worker 1 starts run -> Successfully claims task
        let cmd1 = StartRunCommand::new(&task.id, "worker_1");
        let handle1 = runtime
            .start_run(cmd1)
            .await
            .expect("worker_1 should claim task");
        assert_eq!(handle1.task_id, task.id);

        // Worker 2 attempts start_run on the SAME task before completion -> MUST BE REFUSED
        let cmd2 = StartRunCommand::new(&task.id, "worker_2");
        let err2 = runtime
            .start_run(cmd2)
            .await
            .expect_err("worker_2 claim MUST be refused");

        match err2 {
            DomainError::Conflict(msg) => {
                assert!(msg.contains(&task.id));
                assert!(msg.contains("worker_1"));
            }
            other => panic!("Expected DomainError::Conflict, got: {:?}", other),
        }

        // Cancel worker 1's run -> Frees the claim
        runtime
            .request_cancel(&handle1.run_id, "done")
            .await
            .unwrap();

        // Verify task claim is freed in dispatcher
        assert!(runtime
            .dispatcher()
            .get_active_claim(&task.id, None)
            .await
            .is_none());

        // Create a new task to verify worker 2 can start a fresh run
        let task2 = kernel
            .create_task("Second Spine Task".into())
            .await
            .unwrap();
        let cmd3 = StartRunCommand::new(&task2.id, "worker_2");
        let handle3 = runtime
            .start_run(cmd3)
            .await
            .expect("worker_2 should succeed on task 2");
        assert_eq!(handle3.task_id, task2.id);
    }
}

// =========================================================================
// THEME 4: Execution Mode Negotiation & Metadata Recording
// =========================================================================
mod mode_negotiation_and_recording {
    use super::*;
    use custos_core::contracts::storage::RunPort;

    #[tokio::test]
    async fn test_gate_a_mode_negotiation_recording() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));
        let model = Arc::new(FakeProvider::new("fake-model"));

        let runtime = TaskRuntime::with_ports(kernel.clone(), model).with_run_store(store.clone());

        let task = kernel
            .create_task("Test Mode Negotiation".into())
            .await
            .unwrap();

        // Request native mode when only model is configured -> Expect recorded downgrade
        let cmd = StartRunCommand::new(&task.id, "developer")
            .with_preferred_mode("native")
            .with_workspace_root("/tmp/test_workspace");

        let handle = runtime
            .start_run(cmd)
            .await
            .expect("start_run should succeed");

        // Inspect durable run in persistence
        let stored_run = store
            .get_run(&handle.run_id)
            .await
            .unwrap()
            .expect("run exists");
        assert_eq!(stored_run.metadata["requested_mode"], "native");
        assert_eq!(stored_run.metadata["actual_mode"], "model");
        assert_eq!(stored_run.metadata["mode_downgraded"], true);
        assert_eq!(stored_run.metadata["workspace_root"], "/tmp/test_workspace");
    }
}

// =========================================================================
// THEME 5: Daemon Local API Execution Spine Roundtrip
// =========================================================================
mod daemon_api_spine_roundtrip {
    use super::*;
    use custos_daemon::{ApiRequest, CustosRuntime};

    #[tokio::test]
    async fn test_gate_a_daemon_api_roundtrip() {
        let temp_dir =
            std::env::temp_dir().join(format!("custos_spine_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("spine.db");

        let model = Arc::new(FakeProvider::new("fake-model"));
        let daemon = CustosRuntime::bootstrap_with_model(db_path.to_str().unwrap(), model).unwrap();

        // 1. Create task via Local API
        let create_req = ApiRequest {
            id: "req_task_create".into(),
            method: "v1.tasks.create".into(),
            params: serde_json::json!({
                "title": "Gate A End-to-End Spine Task"
            }),
        };
        let create_resp = daemon.local_api.handle_request(create_req).await;
        assert!(create_resp.error.is_none());
        let task: Task = serde_json::from_value(create_resp.result.unwrap()).unwrap();

        // 2. Start run via Local API
        let start_req = ApiRequest {
            id: "req_run_start".into(),
            method: "v1.workflow.start_run".into(),
            params: serde_json::json!({
                "task_id": task.id,
                "actor": "sade_orchestrator",
                "preferred_mode": "model"
            }),
        };
        let start_resp = daemon.local_api.handle_request(start_req).await;
        assert!(start_resp.error.is_none(), "Error: {:?}", start_resp.error);
        let handle: RunHandle = serde_json::from_value(start_resp.result.unwrap()).unwrap();
        assert_eq!(handle.task_id, task.id);
        assert_eq!(handle.status, RunStatus::Active);

        // 3. Verify double-dispatch via API is blocked
        let dup_req = ApiRequest {
            id: "req_run_dup".into(),
            method: "v1.workflow.start_run".into(),
            params: serde_json::json!({
                "task_id": task.id,
                "actor": "rogue_worker"
            }),
        };
        let dup_resp = daemon.local_api.handle_request(dup_req).await;
        assert!(dup_resp.error.is_some());
        assert!(dup_resp.error.unwrap().contains("already claimed"));

        // 4. Cancel run via Local API
        let cancel_req = ApiRequest {
            id: "req_run_cancel".into(),
            method: "v1.workflow.cancel_run".into(),
            params: serde_json::json!({
                "run_id": handle.run_id,
                "reason": "Gate A Verification Completed"
            }),
        };
        let cancel_resp = daemon.local_api.handle_request(cancel_req).await;
        assert!(cancel_resp.error.is_none());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

// =========================================================================
// THEME 6: Workspace Lease Allocation & Isolation in start_run (Phase 3 Gate C)
// =========================================================================
mod workspace_lease_isolation_in_start_run {
    use super::*;
    use custos_core::contracts::RunPort;
    use custos_runtime::workflow::WorkspaceLeaseManager;

    #[tokio::test]
    async fn test_gate_c_workspace_lease_allocation_and_release() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
        let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));
        let model = Arc::new(FakeProvider::new("fake-model"));

        let temp_dir = std::env::temp_dir().join(format!(
            "custos_lease_test_{}",
            custos_domain::new_id("test")
        ));
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");

        let lease_manager = Arc::new(WorkspaceLeaseManager::new(temp_dir.clone()));

        let runtime = Arc::new(
            TaskRuntime::new()
                .with_kernel(kernel.clone())
                .with_model(model)
                .with_run_store(store.clone())
                .with_lease_manager(lease_manager.clone()),
        );

        let contract = TaskContract {
            pack_id: "engineering".into(),
            name: "lease_verification".into(),
            description: "Test workspace lease allocation".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![],
        };
        let task = kernel
            .create_task_with_contract("Verify Workspace Lease".into(), contract)
            .await
            .expect("task creation should succeed");

        let start_cmd = StartRunCommand::new(&task.id, "worker_1")
            .with_workspace_root(temp_dir.display().to_string());

        let handle = runtime
            .start_run(start_cmd)
            .await
            .expect("start_run must succeed with lease manager");

        // Verify lease allocation in run store
        let run = store
            .get_run(&handle.run_id)
            .await
            .expect("get run")
            .expect("run exists");

        let lease_id = run.metadata.get("lease_id").and_then(|v| v.as_str());
        assert!(
            lease_id.is_some(),
            "lease_id must be stamped in run.metadata"
        );
        let lease_id_str = lease_id.unwrap();
        assert!(lease_id_str.starts_with("wls_"));

        let lease_path = run.metadata.get("lease_path").and_then(|v| v.as_str());
        assert!(
            lease_path.is_some(),
            "lease_path must be stamped in run.metadata"
        );

        // Verify lease is registered as active in lease manager
        let active_lease = lease_manager.get_lease(lease_id_str);
        assert!(active_lease.is_some());
        assert!(active_lease.unwrap().is_active());

        // Cancel run should release the lease
        let cancel_receipt = runtime
            .request_cancel(&handle.run_id, "Lease verification complete")
            .await
            .expect("cancellation should succeed");

        assert_eq!(cancel_receipt.run_id, handle.run_id);

        // Verify lease is now released / inactive
        let released_lease = lease_manager.get_lease(lease_id_str);
        assert!(released_lease.is_none() || !released_lease.unwrap().is_active());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
