//! Execution Spine Vertical Slice (PR 1 - Gate 1 & 2)
//!
//! Reorganized into clear thematic test modules:
//! 1. `start_and_checkpoint_lifecycle`: Task creation, start_run, Running transition, and integrity checkpointing.
//! 2. `graceful_cancellation_lifecycle`: Cancellation request, zero uncertain side effects, and terminal cancellation transition.

use std::sync::Arc;
use custos_adapters::custos_adapter_provider_fake::FakeProvider;
use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::contracts::workflow::WorkflowPort;
use custos_domain::{
    ContractEvidence, EvidenceKind, RunHandle, RunStatus, StartRunCommand, Task, TaskContract,
    TaskStatus,
};
use custos_persistence::SqliteTaskStore;
use custos_runtime::workflow::TaskRuntime;

async fn setup_spine_fixture() -> (Arc<dyn KernelPort>, Arc<dyn WorkflowPort>, Task) {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store));
    let model = Arc::new(FakeProvider::new("fake-model"));
    let runtime: Arc<dyn WorkflowPort> = Arc::new(TaskRuntime::with_ports(kernel.clone(), model));

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

        let start_cmd = StartRunCommand::new(&task.id, "orchestrator")
            .with_workflow_revision("wf_rev_001");
        let run_handle: RunHandle = runtime
            .start_run(start_cmd)
            .await
            .expect("start_run must succeed");

        assert_eq!(run_handle.task_id, task.id);
        assert_eq!(run_handle.status, RunStatus::Active);

        // Verify Kernel state transitioned to Running
        let running_task = kernel.get_task(&task.id).await.unwrap().expect("task exists");
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

        let start_cmd = StartRunCommand::new(&task.id, "orchestrator")
            .with_workflow_revision("wf_rev_001");
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
        let cancelled_task = kernel.get_task(&task.id).await.unwrap().expect("task exists");
        assert_eq!(cancelled_task.status, TaskStatus::Cancelled);
    }
}
