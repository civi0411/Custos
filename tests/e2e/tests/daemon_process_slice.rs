//! Real Daemon Process-Level Integration & Crash Recovery Slice
//!
//! Validates:
//! 1. CLI / client communicates with custos-daemon strictly over Local API JSONL (stdio).
//! 2. Single-writer invariant: custos-daemon alone opens SQLite and manages state transitions.
//! 3. Proof-closure invariant: advancing directly to Succeeded via v1.tasks.advance is rejected.
//! 4. Completion gate: v1.tasks.complete transitions task to Succeeded.
//! 5. Crash recovery: Daemon process killed mid-lifecycle, restarted against same SQLite DB,
//!    recovers intact Task state and continues accepting requests without corruption.

use custos_core_domain::{Task, TaskStatus};
use custos_local_api::{LocalApiClient, ProcessTransport};
use std::path::PathBuf;

fn resolve_daemon_binary() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_custos-daemon") {
        return PathBuf::from(p);
    }
    if let Ok(mut current) = std::env::current_exe() {
        current.pop();
        if current.file_name().and_then(|n| n.to_str()) == Some("deps") {
            current.pop();
        }
        let candidate = current.join(if cfg!(windows) {
            "custos-daemon.exe"
        } else {
            "custos-daemon"
        });
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from("target/debug/custos-daemon")
}

#[tokio::test]
async fn test_daemon_process_lifecycle_and_crash_recovery() {
    let daemon_bin = resolve_daemon_binary();
    assert!(
        daemon_bin.exists(),
        "custos-daemon binary must exist at {:?}. Run `cargo build --bin custos-daemon` first.",
        daemon_bin
    );

    let temp_dir = std::env::temp_dir().join(format!("custos_daemon_e2e_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("daemon_test.db");
    let db_path_str = db_path.to_str().unwrap();

    let task_id: String;

    // --- PHASE 1: Launch Daemon Process 1 and exercise Task Lifecycle ---
    {
        let transport = ProcessTransport::spawn(daemon_bin.to_str().unwrap(), Some(db_path_str))
            .expect("Failed to launch custos-daemon process");
        let client = LocalApiClient::new(Box::new(transport));

        // 1. Create Task via Local API
        let task: Task = client
            .create_task(
                "p1_create",
                "Daemon Process E2E Task",
                None,
                Some(serde_json::json!({"e2e": true, "process": 1})),
            )
            .await
            .expect("CreateTask over daemon IPC must succeed");

        assert_eq!(task.title, "Daemon Process E2E Task");
        assert_eq!(task.status, TaskStatus::Draft);
        assert_eq!(task.epoch, 0);
        task_id = task.id.clone();

        // 2. Advance to Queued
        let queued_task = client
            .advance_task("p1_queue", &task_id, TaskStatus::Queued)
            .await
            .expect("Advance to Queued must succeed");
        assert_eq!(queued_task.status, TaskStatus::Queued);
        assert_eq!(queued_task.epoch, 1);

        // 3. Advance to Running
        let running_task = client
            .advance_task("p1_run", &task_id, TaskStatus::Running)
            .await
            .expect("Advance to Running must succeed");
        assert_eq!(running_task.status, TaskStatus::Running);
        assert_eq!(running_task.epoch, 2);

        // 4. Invariant Enforcement: Advancing directly to Succeeded must be REJECTED
        let bad_advance = client
            .advance_task("p1_bad_succeeded", &task_id, TaskStatus::Succeeded)
            .await;
        assert!(
            bad_advance.is_err(),
            "Advancing directly to Succeeded without proof closure must be rejected"
        );
        let err_msg = bad_advance.unwrap_err();
        assert!(
            err_msg.contains("Cannot advance directly to Succeeded"),
            "Error must specify proof closure requirement, got: {err_msg}"
        );

        // 5. Complete Task via v1.tasks.complete
        let completed_task = client
            .complete_task(
                "p1_complete",
                &task_id,
                Some("Task payload verified and successfully closed".into()),
            )
            .await
            .expect("CompleteTask over daemon IPC must succeed");
        assert_eq!(completed_task.status, TaskStatus::Succeeded);
        assert_eq!(completed_task.epoch, 3);

        // 6. Query Task via v1.tasks.get
        let fetched_task = client
            .get_task("p1_get", &task_id)
            .await
            .expect("GetTask over daemon IPC must succeed");
        assert_eq!(fetched_task.status, TaskStatus::Succeeded);
        assert_eq!(fetched_task.id, task_id);

        // 7. Exercise Interactive Session Lifecycle & Steering
        let session = client
            .create_session("p1_sess_create", Some("assisted"))
            .await
            .expect("CreateSession over daemon IPC must succeed");
        assert_eq!(session.status, custos_core_domain::SessionStatus::Active);
        let session_id_str = session.id.0.clone();

        client
            .append_session_message(
                "p1_msg1",
                &session_id_str,
                "user",
                "I want to refactor the database layer",
            )
            .await
            .expect("Append user message must succeed");

        client
            .append_session_message(
                "p1_msg2",
                &session_id_str,
                "assistant",
                "Sure, I can help analyze that",
            )
            .await
            .expect("Append assistant message must succeed");

        client
            .attach_session("p1_attach", &session_id_str, &task_id)
            .await
            .expect("Attach session must succeed");

        let steer_res = client
            .steer_session(
                "p1_steer",
                &session_id_str,
                &task_id,
                "Prioritize SQLite WAL checkpoints",
            )
            .await
            .expect("Steer task must succeed");
        assert_eq!(
            steer_res.get("accepted").and_then(|v| v.as_bool()),
            Some(true)
        );

        let journal = client
            .get_session_journal("p1_journal", &session_id_str)
            .await
            .expect("GetSessionJournal must succeed");
        assert_eq!(journal.len(), 3);
        assert_eq!(journal[0].entry_type, "user_message");
        assert_eq!(journal[1].entry_type, "assistant_message");
        assert_eq!(journal[2].entry_type, "user_message");
        assert!(journal[2].entry_data.contains("[STEER TASK"));

        // Daemon Process 1 terminates here when transport is dropped / killed
    }

    // --- PHASE 2: Kill / Restart Daemon Process 2 & Assert Persistence Recovery ---
    {
        // Boot a completely new daemon process instance pointing to the same SQLite file
        let transport = ProcessTransport::spawn(daemon_bin.to_str().unwrap(), Some(db_path_str))
            .expect("Failed to launch second custos-daemon process for recovery verification");
        let client = LocalApiClient::new(Box::new(transport));

        // 8. Verify Task state survived process death and was recovered
        let recovered_task = client
            .get_task("p2_get", &task_id)
            .await
            .expect("GetTask after daemon restart must succeed");
        assert_eq!(recovered_task.id, task_id);
        assert_eq!(recovered_task.title, "Daemon Process E2E Task");
        assert_eq!(recovered_task.status, TaskStatus::Succeeded);
        assert_eq!(recovered_task.epoch, 3);

        // 9. Verify Session state & Interaction Journal survived process death
        let session_list = client
            .list_sessions("p2_sess_list")
            .await
            .expect("ListSessions after daemon restart must succeed");
        assert_eq!(session_list.len(), 1);
        let recovered_session_id = session_list[0].id.0.clone();

        let recovered_session = client
            .get_session("p2_sess_get", &recovered_session_id)
            .await
            .expect("GetSession after daemon restart must succeed");
        assert_eq!(
            recovered_session.status,
            custos_core_domain::SessionStatus::Active
        );
        assert_eq!(
            recovered_session.attached_to.as_deref(),
            Some(task_id.as_str())
        );

        let recovered_journal = client
            .get_session_journal("p2_sess_journal", &recovered_session_id)
            .await
            .expect("GetSessionJournal after daemon restart must succeed");
        assert_eq!(recovered_journal.len(), 3);
        assert_eq!(
            recovered_journal[0].entry_data,
            "I want to refactor the database layer"
        );

        // 10. Concurrency Invariant: Session continues independently while Tasks run
        client
            .append_session_message(
                "p2_msg3",
                &recovered_session_id,
                "user",
                "How did the completed task turn out?",
            )
            .await
            .expect("Session continues accepting dialogue independently");

        let journal_after = client
            .get_session_journal("p2_journal_after", &recovered_session_id)
            .await
            .unwrap();
        assert_eq!(journal_after.len(), 4);

        // 11. Verify Task listing recovers all historical tasks
        let list = client
            .list_tasks("p2_list")
            .await
            .expect("ListTasks after daemon restart must succeed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, task_id);

        // 12. Verify new tasks can be created and operated post-recovery
        let task2 = client
            .create_task("p2_create2", "Post-Recovery Task", None, None)
            .await
            .expect("CreateTask after restart must succeed");
        assert_eq!(task2.title, "Post-Recovery Task");
        assert_eq!(task2.status, TaskStatus::Draft);

        let list_after = client
            .list_tasks("p2_list2")
            .await
            .expect("ListTasks must now show 2 tasks");
        assert_eq!(list_after.len(), 2);
    }

    // Clean up temporary database directory
    let _ = std::fs::remove_dir_all(&temp_dir);
}
