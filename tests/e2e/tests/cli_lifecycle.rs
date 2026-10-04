//! End-to-End Task Lifecycle Test
//!
//! Reorganized into clear thematic test modules:
//! 1. `task_lifecycle_and_spans`: Task creation, status advancement, execution span recording, continuation packet persistence.
//! 2. `persistence_restart_and_invariants`: Database reconnection integrity, continuation verification, terminal immutability, and optimistic concurrency.

use custos_core::{AdvanceTask, CompleteTask, CreateTask, TaskService, TaskStore};
use custos_domain::{ContinuationPacket, Span, SpanState, TaskStatus};
use custos_persistence::SqliteTaskStore;
use std::sync::Arc;

// =========================================================================
// THEME 1: Task Lifecycle & Execution Spans
// =========================================================================
mod task_lifecycle_and_spans {
    use super::*;

    #[tokio::test]
    async fn test_task_advancement_spans_and_continuation_packets() {
        let temp_dir = std::env::temp_dir().join(format!("custos_e2e_life_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("lifecycle.db");

        let store = Arc::new(SqliteTaskStore::new(db_path.to_str().unwrap()).expect("Must open db"));
        let service = TaskService::new(store.clone());

        // Create Task
        let (task, _) = service
            .execute_create(CreateTask {
                title: "Lifecycle Task".to_string(),
                metadata: Some(serde_json::json!({"env": "test"})),
                contract: None,
            })
            .await
            .expect("Create must succeed");
        assert_eq!(task.status, TaskStatus::Draft);
        assert_eq!(task.epoch, 0);

        // Advance to Queued -> Running
        let (task, _) = service
            .execute_advance(AdvanceTask {
                task_id: task.id.clone(),
                next_status: TaskStatus::Queued,
                expected_epoch: 0,
                rationale: Some("Enqueueing".into()),
            })
            .await
            .unwrap();
        assert_eq!(task.status, TaskStatus::Queued);

        let (task, _) = service
            .execute_advance(AdvanceTask {
                task_id: task.id.clone(),
                next_status: TaskStatus::Running,
                expected_epoch: 1,
                rationale: Some("Running".into()),
            })
            .await
            .unwrap();
        assert_eq!(task.status, TaskStatus::Running);

        // Save Span and Continuation
        let span = Span::new(
            format!("span_1_{}", task.id),
            task.id.clone(),
            1,
            "provider-fake".into(),
            "model-test".into(),
            "sha256:input".into(),
        );
        store.save_span(&span).await.unwrap();

        let continuation = ContinuationPacket::create(
            task.id.clone(),
            1,
            2,
            "provider-fake".into(),
            "model-test".into(),
            "Completed span 1".into(),
            serde_json::json!({"step": 1}),
        )
        .unwrap();
        store.save_continuation(&continuation).await.unwrap();

        // Complete Task
        let (completed, _) = service
            .execute_complete(CompleteTask {
                task_id: task.id.clone(),
                summary: "Done".into(),
                expected_epoch: 2,
                evidence_claims: vec![],
            })
            .await
            .unwrap();
        assert_eq!(completed.status, TaskStatus::Succeeded);
        assert_eq!(completed.epoch, 3);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

// =========================================================================
// THEME 2: Persistence Restart & Concurrency Invariants
// =========================================================================
mod persistence_restart_and_invariants {
    use super::*;

    #[tokio::test]
    async fn test_persistence_integrity_after_reopen_and_epoch_invariants() {
        let temp_dir = std::env::temp_dir().join(format!("custos_e2e_reopen_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("reopen.db");
        let db_path_str = db_path.to_str().unwrap();

        let task_id: String;

        // Phase 1: Create, execute, and complete task
        {
            let store = Arc::new(SqliteTaskStore::new(db_path_str).expect("Must open db"));
            let service = TaskService::new(store.clone());

            let (task, _) = service
                .execute_create(CreateTask {
                    title: "Reopen Task".to_string(),
                    metadata: Some(serde_json::json!({"env": "test", "priority": "high"})),
                    contract: None,
                })
                .await
                .unwrap();
            task_id = task.id.clone();

            let (task, _) = service
                .execute_advance(AdvanceTask {
                    task_id: task.id.clone(),
                    next_status: TaskStatus::Queued,
                    expected_epoch: 0,
                    rationale: None,
                })
                .await
                .unwrap();

            let (task, _) = service
                .execute_advance(AdvanceTask {
                    task_id: task.id.clone(),
                    next_status: TaskStatus::Running,
                    expected_epoch: 1,
                    rationale: None,
                })
                .await
                .unwrap();

            let span = Span::new(
                format!("span_1_{}", task.id),
                task.id.clone(),
                1,
                "provider-fake".into(),
                "model-test".into(),
                "sha256:input1".into(),
            );
            store.save_span(&span).await.unwrap();

            let continuation = ContinuationPacket::create(
                task.id.clone(),
                1,
                2,
                "provider-fake".into(),
                "model-test".into(),
                "Summary".into(),
                serde_json::json!({"done": true}),
            )
            .unwrap();
            store.save_continuation(&continuation).await.unwrap();

            service
                .execute_complete(CompleteTask {
                    task_id: task.id.clone(),
                    summary: "Done".into(),
                    expected_epoch: 2,
                    evidence_claims: vec![],
                })
                .await
                .unwrap();
        }

        // Phase 2: Reopen SQLite file in new connection & verify state & invariants
        {
            let store2 = Arc::new(SqliteTaskStore::new(db_path_str).expect("Must re-open db"));
            let service2 = TaskService::new(store2.clone());

            let retrieved = service2.get_task(&task_id).await.unwrap().expect("Task must exist");
            assert_eq!(retrieved.id, task_id);
            assert_eq!(retrieved.status, TaskStatus::Succeeded);
            assert_eq!(retrieved.epoch, 3);
            assert_eq!(retrieved.metadata["priority"], "high");

            let spans = store2.list_spans(&task_id).await.unwrap();
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].state, SpanState::Started);

            let cont = store2.get_latest_continuation(&task_id).await.unwrap().unwrap();
            assert_eq!(cont.task_id, task_id);
            assert!(cont.verify().is_ok(), "Integrity hash verification must pass");

            // Invariant: cannot advance terminal task
            let advance_err = service2
                .execute_advance(AdvanceTask {
                    task_id: task_id.clone(),
                    next_status: TaskStatus::Running,
                    expected_epoch: 3,
                    rationale: None,
                })
                .await;
            assert!(advance_err.is_err(), "Terminal task cannot be advanced");

            // Optimistic concurrency: wrong epoch rejected
            let stale_err = service2
                .execute_complete(CompleteTask {
                    task_id: task_id.clone(),
                    summary: "Repeat".into(),
                    expected_epoch: 0,
                    evidence_claims: vec![],
                })
                .await;
            assert!(stale_err.is_err(), "Stale epoch must be rejected");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
