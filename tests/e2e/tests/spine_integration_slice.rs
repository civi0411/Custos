//! Spine Integration Slice (PR 3.2 - Gates 1, 2, 3, 4 Verification)
//!
//! Reorganized into clear thematic test modules:
//! 1. `mediated_spine_execution`: End-to-end mediated execution, transactional outbox, and effect ledger persistence.
//! 2. `gate3_idempotency_conflict`: Strict rejection of duplicate idempotency keys with DomainError::Conflict.
//! 3. `crash_reconciliation_and_cancellation`: InFlight to Uncertain startup recovery and graceful run cancellation.

use async_trait::async_trait;
use custos_adapters::sandbox::SovereignDeveloperAdapter;
use custos_core::contracts::harness::{AgentRuntimePort, HarnessProfile, ToolMediationLevel};
use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::contracts::storage::{EffectLedgerPort, OutboxPort, OutboxStatus, RunPort};
use custos_core::contracts::workflow::WorkflowPort;
use custos_domain::{
    new_id, ActionIntent, Assurance, ContextPack, ContractEvidence, DomainError, EffectAttempt,
    EffectStatus, EvidenceKind, RiskLevel, RunHandle, RunStatus, StartRunCommand, Task,
    TaskContract, TaskStatus, WorkerRun,
};
use custos_persistence::SqliteTaskStore;
use custos_runtime::workflow::TaskRuntime;
use std::sync::Arc;

struct TestGovernedHarness;

#[async_trait]
impl AgentRuntimePort for TestGovernedHarness {
    fn harness_id(&self) -> &str {
        "test-governed"
    }

    fn profile(&self) -> HarnessProfile {
        HarnessProfile::new("test-governed", ToolMediationLevel::CustosMediated)
            .with_supports_cancel(true)
    }

    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        _context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        let mut intent = ActionIntent::new(
            new_id("act"),
            "bash".into(),
            "local_shell".into(),
            serde_json::json!({ "command": "echo 'custos-spine-verified'" }),
            RiskLevel::Low,
        );
        intent.task_id = Some(worker_run.task_id.clone());
        intent.idempotency_key = Some("idemp-spine-001".into());
        intent.assurance = Assurance::CustosMediated;
        Ok(vec![intent])
    }
}

async fn setup_spine_fixture() -> (
    Arc<SqliteTaskStore>,
    Arc<dyn KernelPort>,
    Arc<dyn WorkflowPort>,
    Task,
) {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));
    let harness: Arc<dyn AgentRuntimePort> = Arc::new(TestGovernedHarness);
    let sandbox = Arc::new(SovereignDeveloperAdapter::new(std::env::temp_dir()));

    let runtime: Arc<dyn WorkflowPort> = Arc::new(
        TaskRuntime::new()
            .with_kernel(kernel.clone())
            .with_harness(harness)
            .with_sandbox(sandbox)
            .with_outbox(store.clone())
            .with_effect_ledger(store.clone())
            .with_run_store(store.clone()),
    );

    let contract = TaskContract {
        pack_id: "governance".into(),
        name: "spine_audit".into(),
        description: "Audit mediated spine lifecycle".into(),
        required_capabilities: vec!["shell".into()],
        evidence_requirements: vec![ContractEvidence {
            kind: EvidenceKind::FileAnchor,
            required: true,
        }],
    };
    let task = kernel
        .create_task_with_contract("Verify Spine Integrity".into(), contract)
        .await
        .expect("create task");

    (store, kernel, runtime, task)
}

// =========================================================================
// THEME 1: Mediated Spine Execution & Durable Persistence
// =========================================================================
mod mediated_spine_execution {
    use super::*;

    #[tokio::test]
    async fn test_spine_end_to_end_mediated_execution() {
        let (store, kernel, runtime, task) = setup_spine_fixture().await;
        assert_eq!(task.status, TaskStatus::Draft);

        let start_cmd =
            StartRunCommand::new(&task.id, "governor").with_workflow_revision("spine_v1");
        let run_handle: RunHandle = runtime
            .start_run(start_cmd)
            .await
            .expect("start_run must succeed");

        assert_eq!(run_handle.task_id, task.id);
        assert_eq!(run_handle.status, RunStatus::Active);

        // Verify Task state in Kernel transitioned to Running
        let running_task = kernel
            .get_task(&task.id)
            .await
            .unwrap()
            .expect("task exists");
        assert_eq!(running_task.status, TaskStatus::Running);

        // Verify Durable Run in SQLite
        let persisted_run = store
            .get_run(&run_handle.run_id)
            .await
            .unwrap()
            .expect("run exists");
        assert_eq!(persisted_run.status, RunStatus::Active);
        assert_eq!(persisted_run.workflow_revision.as_deref(), Some("spine_v1"));

        // Verify Transactional Outbox
        let receipted = store
            .outbox()
            .list_by_status(OutboxStatus::Receipted)
            .await
            .unwrap();
        assert_eq!(receipted.len(), 1);
        let entry = &receipted[0];
        assert_eq!(entry.task_id, task.id);
        assert_eq!(entry.idempotency_key.as_deref(), Some("idemp-spine-001"));

        let receipt = entry.receipt.as_ref().unwrap();
        assert_eq!(receipt.assurance, Assurance::CustosMediated);
        assert!(receipt.output_digest.starts_with("blake3:"));

        // Verify Effect Ledger
        let effect = store
            .get_effect_by_idempotency_key("idemp-spine-001")
            .await
            .unwrap()
            .expect("effect exists");
        assert_eq!(effect.status, EffectStatus::Succeeded);
    }
}

// =========================================================================
// THEME 2: Gate 3 Idempotency Conflict Enforcement
// =========================================================================
mod gate3_idempotency_conflict {
    use super::*;

    #[tokio::test]
    async fn test_gate3_duplicate_idempotency_key_conflict() {
        let (store, _, runtime, task) = setup_spine_fixture().await;

        let start_cmd =
            StartRunCommand::new(&task.id, "governor").with_workflow_revision("spine_v1");
        runtime.start_run(start_cmd).await.expect("start_run");

        let worker_runs = store.list_worker_runs_for_task(&task.id).await.unwrap();
        assert_eq!(worker_runs.len(), 1);

        // Attempt duplicate idempotency key insertion
        let dup_effect =
            EffectAttempt::new(worker_runs[0].id.clone(), "permit_dummy", "idemp-spine-001");
        let conflict_result = store.record_effect(&dup_effect).await;
        assert!(
            matches!(conflict_result, Err(DomainError::Conflict(_))),
            "Duplicate idempotency key must trigger DomainError::Conflict"
        );
    }
}

// =========================================================================
// THEME 3: Crash Reconciliation & Graceful Cancellation
// =========================================================================
mod crash_reconciliation_and_cancellation {
    use super::*;

    #[tokio::test]
    async fn test_reconciliation_and_graceful_cancellation() {
        let (store, kernel, runtime, task) = setup_spine_fixture().await;

        let start_cmd =
            StartRunCommand::new(&task.id, "governor").with_workflow_revision("spine_v1");
        let run_handle = runtime.start_run(start_cmd).await.expect("start_run");

        // Simulate interrupted in-flight effect
        let in_flight = EffectAttempt::new(
            "node_interrupt",
            "permit_interrupt",
            "idemp-interrupted-002",
        );
        store.record_effect(&in_flight).await.unwrap();
        store
            .update_effect_status(&in_flight.id, EffectStatus::InFlight, None)
            .await
            .unwrap();

        // Reconcile on startup converts InFlight -> Uncertain (INV-03)
        let count = store.reconcile_on_startup().await.unwrap();
        assert_eq!(count, 1);

        let reconciled = store
            .get_effect_by_idempotency_key("idemp-interrupted-002")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reconciled.status, EffectStatus::Uncertain);

        // Request Graceful Cancellation
        let cancel_receipt = runtime
            .request_cancel(&run_handle.run_id, "Audit finished")
            .await
            .unwrap();
        assert_eq!(cancel_receipt.run_id, run_handle.run_id);
        assert_eq!(cancel_receipt.reason, "Audit finished");

        let cancelled_task = kernel.get_task(&task.id).await.unwrap().unwrap();
        assert_eq!(cancelled_task.status, TaskStatus::Cancelled);

        let cancelled_run = store.get_run(&run_handle.run_id).await.unwrap().unwrap();
        assert_eq!(cancelled_run.status, RunStatus::Cancelled);
    }
}
