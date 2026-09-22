//! End-to-End Task Lifecycle Test
//!
//! Validates the full execution lifecycle of Custos tasks using file-based SQLite:
//! - CreateTask (Draft)
//! - Advance to Queued -> Running
//! - Span recordings and ContinuationPacket persistence
//! - CompleteTask (Succeeded)
//! - Persistence integrity across process restarts (re-opening SQLite file)
//! - Optimistic concurrency rejection on epoch mismatch
//! - State invariant rejection on completed tasks

use custos_core_domain::{ContinuationPacket, Span, SpanState, TaskStatus};
use custos_persistence_sqlite::SqliteTaskStore;
use custos_task_kernel::{AdvanceTask, CompleteTask, CreateTask, TaskService, TaskStore};
use std::sync::Arc;

#[tokio::test]
async fn test_full_file_backed_lifecycle_and_restart() {
    let temp_dir = std::env::temp_dir().join(format!("custos_e2e_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("custos_test.db");
    let db_path_str = db_path.to_str().unwrap();

    let task_id = {
        // Step 1: Open fresh file-backed database
        let store = Arc::new(SqliteTaskStore::new(db_path_str).expect("Must open db"));
        let service = TaskService::new(store.clone());

        // Step 2: Create a task
        let (task, _) = service
            .execute_create(CreateTask {
                title: "E2E Automated Task".to_string(),
                metadata: Some(serde_json::json!({"env": "test", "priority": "high"})),
            })
            .await
            .expect("Create must succeed");

        assert_eq!(task.status, TaskStatus::Draft);
        assert_eq!(task.epoch, 0);

        // Step 3: Advance to Queued
        let (task, _) = service
            .execute_advance(AdvanceTask {
                task_id: task.id.clone(),
                next_status: TaskStatus::Queued,
                expected_epoch: 0,
                rationale: Some("Enqueueing task".to_string()),
            })
            .await
            .expect("Advance to Queued must succeed");

        assert_eq!(task.status, TaskStatus::Queued);
        assert_eq!(task.epoch, 1);

        // Step 4: Advance to Running
        let (task, _) = service
            .execute_advance(AdvanceTask {
                task_id: task.id.clone(),
                next_status: TaskStatus::Running,
                expected_epoch: 1,
                rationale: Some("Worker picked up task".to_string()),
            })
            .await
            .expect("Advance to Running must succeed");

        assert_eq!(task.status, TaskStatus::Running);
        assert_eq!(task.epoch, 2);

        // Step 5: Record execution Spans and Continuation Packet
        let span1 = Span::new(
            format!("span_1_{}", task.id),
            task.id.clone(),
            1,
            "provider-fake".to_string(),
            "model-test".to_string(),
            "sha256:input1".to_string(),
        );
        store
            .save_span(&span1)
            .await
            .expect("Save span 1 must succeed");

        let continuation = ContinuationPacket::create(
            task.id.clone(),
            1,
            2,
            "provider-fake".to_string(),
            "model-test".to_string(),
            "Completed span 1 processing".to_string(),
            serde_json::json!({"step": 1, "done": true}),
        )
        .expect("ContinuationPacket create must succeed");

        store
            .save_continuation(&continuation)
            .await
            .expect("Save continuation must succeed");

        // Step 6: Complete Task
        let (task, _) = service
            .execute_complete(CompleteTask {
                task_id: task.id.clone(),
                summary: "All steps succeeded".to_string(),
                expected_epoch: 2,
            })
            .await
            .expect("Complete task must succeed");

        assert_eq!(task.status, TaskStatus::Succeeded);
        assert_eq!(task.epoch, 3);

        task.id
    };

    // Step 7: Restart simulation — open a new connection to the same SQLite file
    {
        let store2 = Arc::new(SqliteTaskStore::new(db_path_str).expect("Must re-open db"));
        let service2 = TaskService::new(store2.clone());

        // Verify task state persisted intact
        let retrieved = service2
            .get_task(&task_id)
            .await
            .expect("Query must succeed")
            .expect("Task must exist in DB");

        assert_eq!(retrieved.id, task_id);
        assert_eq!(retrieved.title, "E2E Automated Task");
        assert_eq!(retrieved.status, TaskStatus::Succeeded);
        assert_eq!(retrieved.epoch, 3);
        assert_eq!(retrieved.metadata["env"], "test");

        // Verify spans persisted intact
        let spans = store2
            .list_spans(&task_id)
            .await
            .expect("List spans must succeed");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].span_num, 1);
        assert_eq!(spans[0].state, SpanState::Started);

        // Verify continuation packet persisted and has valid integrity hash
        let cont = store2
            .get_latest_continuation(&task_id)
            .await
            .expect("Get continuation must succeed")
            .expect("Continuation must exist");

        assert_eq!(cont.task_id, task_id);
        assert_eq!(cont.from_span, 1);
        assert_eq!(cont.to_span, 2);
        assert!(
            cont.verify().is_ok(),
            "Integrity hash verification must pass"
        );

        // Step 8: Invariant check — cannot mutate completed task
        let mutate_result = service2
            .execute_advance(AdvanceTask {
                task_id: task_id.clone(),
                next_status: TaskStatus::Running,
                expected_epoch: 3,
                rationale: None,
            })
            .await;
        assert!(mutate_result.is_err(), "Terminal task cannot be advanced");

        // Step 9: Optimistic concurrency check — wrong epoch rejected
        let stale_result = service2
            .execute_complete(CompleteTask {
                task_id: task_id.clone(),
                summary: "Repeat".to_string(),
                expected_epoch: 0,
            })
            .await;
        assert!(stale_result.is_err(), "Stale epoch must be rejected");
    }

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);
}
