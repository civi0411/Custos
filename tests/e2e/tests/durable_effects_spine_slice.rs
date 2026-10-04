//! Durable Effect Spine & Crash Recovery Slice (PR 2 - Gate 3)
//!
//! Reorganized into clear thematic test modules:
//! 1. `outbox_and_crash_reconciliation`: Durable outbox enqueuing, InFlight recording, startup crash reconciliation to Uncertain, and INV-03 enforcement.
//! 2. `receipt_resolution_and_terminal_idempotency`: Outbox receipting, effect resolution to Succeeded, and idempotency key audit.

use std::sync::Arc;
use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};
use custos_core::kernel::{CreateTask, TaskService};
use custos_domain::{
    new_id, ActionIntent, ActionLifecycleState, EffectAttempt, EffectStatus, ExecutionReceipt,
    ReceiptStatus, RiskLevel, Task,
};
use custos_persistence::SqliteTaskStore;

async fn setup_task(store: &Arc<SqliteTaskStore>) -> Task {
    let service = TaskService::new(store.clone());
    let (task, _) = service
        .execute_create(CreateTask {
            title: "Durable Effect Task".into(),
            metadata: None,
            contract: None,
        })
        .await
        .expect("create task");
    task
}

// =========================================================================
// THEME 1: Outbox InFlight Tracking & Crash Reconciliation to Uncertain
// =========================================================================
mod outbox_and_crash_reconciliation {
    use super::*;

    #[tokio::test]
    async fn test_outbox_inflight_and_crash_reconciliation_to_uncertain() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite"));
        let task = setup_task(&store).await;

        let intent = ActionIntent::new(
            new_id("act"),
            "workspace_write".into(),
            "src/output.txt".into(),
            serde_json::json!({ "content": "hello world" }),
            RiskLevel::Medium,
        )
        .with_task_id(&task.id)
        .with_idempotency_key("idemp_key_unique_001");

        let outbox_entry_id = new_id("ob");
        let permit_id = new_id("permit");

        // Enqueue Outbox entry in Pending state
        store
            .enqueue(OutboxEntry {
                id: outbox_entry_id.clone(),
                task_id: task.id.clone(),
                action_id: intent.id.clone(),
                permit_id: permit_id.clone(),
                argument_digest: intent.argument_digest(),
                idempotency_key: intent.idempotency_key.clone(),
                status: OutboxStatus::Pending,
                created_at: chrono::Utc::now(),
                receipt: None,
            })
            .await
            .expect("enqueue outbox entry");

        // Mark Outbox entry as Dispatching
        store.mark_dispatching(&outbox_entry_id).await.expect("mark dispatching");

        // Record EffectAttempt as InFlight
        let mut effect = EffectAttempt::new("node_att_01", &permit_id, "idemp_key_unique_001");
        effect.mark_in_flight().expect("transition to in_flight");
        store.outbox().record_effect(&effect).expect("record effect attempt");

        let recorded = store.outbox().get_effect_by_idempotency_key("idemp_key_unique_001").unwrap().unwrap();
        assert_eq!(recorded.status, EffectStatus::InFlight);

        // SIMULATE DAEMON CRASH & REBOOT
        let reconciled_count = store.outbox().reconcile_on_startup().expect("reconciliation succeeds");
        assert_eq!(reconciled_count, 2, "reconciles 1 outbox entry + 1 effect attempt");

        // Verify Outbox transitioned to Uncertain
        let uncertain_entries = store.list_by_status(OutboxStatus::Uncertain).await.unwrap();
        assert_eq!(uncertain_entries.len(), 1);
        assert_eq!(uncertain_entries[0].id, outbox_entry_id);

        // Verify EffectAttempt transitioned to Uncertain
        let reconciled_effect = store.outbox().get_effect_by_idempotency_key("idemp_key_unique_001").unwrap().unwrap();
        assert_eq!(reconciled_effect.status, EffectStatus::Uncertain);

        // Invariant 3: Uncertain actions must NEVER permit blind retry
        assert!(
            !ActionLifecycleState::Uncertain.allows_blind_retry(),
            "INV-03: blind retry of uncertain action is strictly forbidden"
        );
    }
}

// =========================================================================
// THEME 2: Receipt Resolution & Terminal Idempotency Verification
// =========================================================================
mod receipt_resolution_and_terminal_idempotency {
    use super::*;

    #[tokio::test]
    async fn test_receipt_resolution_and_terminal_state() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite"));
        let task = setup_task(&store).await;

        let outbox_entry_id = new_id("ob");
        let permit_id = new_id("permit");
        let action_id = new_id("act");

        store
            .enqueue(OutboxEntry {
                id: outbox_entry_id.clone(),
                task_id: task.id.clone(),
                action_id: action_id.clone(),
                permit_id: permit_id.clone(),
                argument_digest: "sha256:digest001".into(),
                idempotency_key: Some("idemp_key_res_001".into()),
                status: OutboxStatus::Dispatching,
                created_at: chrono::Utc::now(),
                receipt: None,
            })
            .await
            .expect("enqueue outbox");

        let mut effect = EffectAttempt::new("node_att_02", &permit_id, "idemp_key_res_001");
        effect.mark_in_flight().unwrap();
        store.outbox().record_effect(&effect).unwrap();

        // Simulate receipt arrival
        let receipt = ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id,
            action_id,
            status: ReceiptStatus::Success,
            output_digest: "sha256:durable_output".into(),
            output_data: Some(serde_json::json!({ "written": true })),
            error_message: None,
            duration_ms: Some(15),
            executed_at: chrono::Utc::now(),
            assurance: custos_domain::Assurance::CustosMediated,
        };

        store.mark_receipted(&outbox_entry_id, receipt.clone()).await.unwrap();
        store.outbox().update_effect_status(&effect.id, EffectStatus::Succeeded, Some(&receipt)).unwrap();

        let final_effect = store.outbox().get_effect_by_idempotency_key("idemp_key_res_001").unwrap().unwrap();
        assert_eq!(final_effect.status, EffectStatus::Succeeded);
        assert!(final_effect.receipt.is_some());
    }
}
