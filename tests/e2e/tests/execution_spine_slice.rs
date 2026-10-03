//! Execution Spine Vertical Slice (PR 1 - Gate 1 & 2)
//!
//! Validates the full in-process execution spine:
//! TrustedKernel -> TaskRuntime (WorkflowPort) -> FakeProvider (ModelPort) -> Outcome.
//! Tests the complete lifecycle: start_run, checkpoint, and graceful cancellation.

use std::sync::Arc;
use custos_adapters::custos_adapter_provider_fake::FakeProvider;
use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::contracts::workflow::WorkflowPort;
use custos_domain::{
    ContractEvidence, EvidenceKind, RunStatus, StartRunCommand, TaskContract, TaskStatus,
};
use custos_persistence::SqliteTaskStore;
use custos_runtime::workflow::TaskRuntime;

#[tokio::test]
async fn test_execution_spine_start_checkpoint_and_cancel_lifecycle() {
    // 1. Bootstrap in-memory persistence and ports
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));
    let model = Arc::new(FakeProvider::new("fake-model"));

    // 2. Instantiate TaskRuntime wired with Kernel and Model ports
    let runtime: Arc<dyn WorkflowPort> = Arc::new(TaskRuntime::with_ports(kernel.clone(), model.clone()));

    // 3. Create Task via KernelPort
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

    assert_eq!(task.status, TaskStatus::Draft);

    // 4. Start Run via WorkflowPort
    let start_cmd = StartRunCommand::new(&task.id, "orchestrator")
        .with_workflow_revision("wf_rev_001");
    let run_handle = runtime
        .start_run(start_cmd)
        .await
        .expect("start_run must succeed");

    assert_eq!(run_handle.task_id, task.id);
    assert_eq!(run_handle.status, RunStatus::Active);

    // 5. Verify Kernel state transitioned to Running
    let running_task = kernel
        .get_task(&task.id)
        .await
        .expect("query task")
        .expect("task exists");
    assert_eq!(running_task.status, TaskStatus::Running);

    // 6. Checkpoint via WorkflowPort
    let checkpoint_packet = runtime
        .checkpoint(&run_handle.run_id)
        .await
        .expect("checkpoint should succeed");
    assert_eq!(checkpoint_packet.task_id, task.id);
    assert!(!checkpoint_packet.integrity_hash.is_empty());

    // 7. Request graceful cancellation
    let cancel_receipt = runtime
        .request_cancel(&run_handle.run_id, "User requested test halt")
        .await
        .expect("cancel should succeed");
    assert_eq!(cancel_receipt.run_id, run_handle.run_id);
    assert_eq!(cancel_receipt.uncertain_effects_count, 0);

    // 8. Verify Task state in Kernel transitioned to Cancelled
    let cancelled_task = kernel
        .get_task(&task.id)
        .await
        .expect("query task")
        .expect("task exists");
    assert_eq!(cancelled_task.status, TaskStatus::Cancelled);
}
