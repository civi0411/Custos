//! Real Daemon Process-Level Integration & Crash Recovery Slice
//!
//! Reorganized into clear thematic test modules:
//! 1. `task_lifecycle_and_invariants`: Local API task creation, epoch transitions, direct advance rejection, and proof-closure completion.
//! 2. `interactive_session_and_journal`: Session creation, dialogue journaling, task attachment, and steering injection.
//! 3. `process_crash_recovery_resilience`: Process-level SIGKILL/restart against SQLite, state durability, and post-recovery operations.

use custos_core_domain::{SessionStatus, Task, TaskStatus};
use custos_daemon::custos_local_api;
use custos_domain as custos_core_domain;
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
        let candidate = current.join(if cfg!(windows) { "custos-daemon.exe" } else { "custos-daemon" });
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from("target/debug/custos-daemon")
}

fn spawn_daemon_client(db_path: &str) -> LocalApiClient {
    let daemon_bin = resolve_daemon_binary();
    assert!(daemon_bin.exists(), "custos-daemon binary must exist at {:?}", daemon_bin);
    let transport = ProcessTransport::spawn(daemon_bin.to_str().unwrap(), Some(db_path))
        .expect("Failed to launch custos-daemon process");
    LocalApiClient::new(Box::new(transport))
}

// =========================================================================
// THEME 1: Task Lifecycle & Proof-Closure Invariants
// =========================================================================
mod task_lifecycle_and_invariants {
    use super::*;

    #[tokio::test]
    async fn test_task_lifecycle_and_proof_closure_enforcement() {
        let temp_dir = std::env::temp_dir().join(format!("custos_daemon_task_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("daemon_task.db");

        let client = spawn_daemon_client(db_path.to_str().unwrap());

        // 1. Create Task via Local API
        let task: Task = client
            .create_task("p1_create", "Daemon Process E2E Task", None, Some(serde_json::json!({"e2e": true})))
            .await
            .expect("CreateTask must succeed");
        assert_eq!(task.status, TaskStatus::Draft);
        assert_eq!(task.epoch, 0);

        // 2. Advance to Queued -> Running
        let queued = client.advance_task("p1_queue", &task.id, TaskStatus::Queued).await.unwrap();
        assert_eq!(queued.status, TaskStatus::Queued);
        assert_eq!(queued.epoch, 1);

        let running = client.advance_task("p1_run", &task.id, TaskStatus::Running).await.unwrap();
        assert_eq!(running.status, TaskStatus::Running);
        assert_eq!(running.epoch, 2);

        // 3. Invariant: Advancing directly to Succeeded must be REJECTED
        let bad_advance = client.advance_task("p1_bad", &task.id, TaskStatus::Succeeded).await;
        assert!(bad_advance.is_err(), "Direct advance to Succeeded must be rejected");
        assert!(bad_advance.unwrap_err().contains("Cannot advance directly to Succeeded"));

        // 4. Complete Task via v1.tasks.complete
        let completed = client.complete_task("p1_comp", &task.id, Some("Task payload verified".into())).await.unwrap();
        assert_eq!(completed.status, TaskStatus::Succeeded);
        assert_eq!(completed.epoch, 3);

        let fetched = client.get_task("p1_get", &task.id).await.unwrap();
        assert_eq!(fetched.status, TaskStatus::Succeeded);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

// =========================================================================
// THEME 2: Interactive Session, Journaling & Task Steering
// =========================================================================
mod interactive_session_and_journal {
    use super::*;

    #[tokio::test]
    async fn test_interactive_session_and_steering_journal() {
        let temp_dir = std::env::temp_dir().join(format!("custos_daemon_sess_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("daemon_sess.db");

        let client = spawn_daemon_client(db_path.to_str().unwrap());

        let task = client.create_task("p2_t", "Steer Target Task", None, None).await.unwrap();
        let session = client.create_session("p2_s", Some("assisted")).await.unwrap();
        assert_eq!(session.status, SessionStatus::Active);
        let session_id = session.id.0.clone();

        client.append_session_message("m1", &session_id, "user", "I want to refactor the database layer").await.unwrap();
        client.append_session_message("m2", &session_id, "assistant", "Sure, I can help analyze that").await.unwrap();
        client.attach_session("att", &session_id, &task.id).await.unwrap();

        let steer_res = client.steer_session("st", &session_id, &task.id, "Prioritize SQLite WAL checkpoints").await.unwrap();
        assert_eq!(steer_res.get("accepted").and_then(|v| v.as_bool()), Some(true));

        let journal = client.get_session_journal("j", &session_id).await.unwrap();
        assert_eq!(journal.len(), 3);
        assert_eq!(journal[0].entry_type, "user_message");
        assert_eq!(journal[1].entry_type, "assistant_message");
        assert!(journal[2].entry_data.contains("[STEER TASK"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

// =========================================================================
// THEME 3: Process Crash Recovery & Multi-Instance Durability
// =========================================================================
mod process_crash_recovery_resilience {
    use super::*;

    #[tokio::test]
    async fn test_daemon_process_kill_and_restart_recovery() {
        let temp_dir = std::env::temp_dir().join(format!("custos_daemon_crash_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("daemon_crash.db");
        let db_path_str = db_path.to_str().unwrap();

        let task_id: String;
        let session_id_str: String;

        // --- Process Instance 1 ---
        {
            let client = spawn_daemon_client(db_path_str);
            let task = client.create_task("p1_t", "Persistent Task", None, None).await.unwrap();
            task_id = task.id.clone();

            client.advance_task("p1_q", &task_id, TaskStatus::Queued).await.unwrap();
            client.advance_task("p1_r", &task_id, TaskStatus::Running).await.unwrap();
            client.complete_task("p1_c", &task_id, Some("Closed".into())).await.unwrap();

            let session = client.create_session("p1_s", Some("assisted")).await.unwrap();
            session_id_str = session.id.0.clone();
            client.append_session_message("p1_m", &session_id_str, "user", "Message before reboot").await.unwrap();
            client.attach_session("p1_att", &session_id_str, &task_id).await.unwrap();
            // Daemon 1 terminates here when dropped
        }

        // --- Process Instance 2 (Reboot) ---
        {
            let client2 = spawn_daemon_client(db_path_str);

            // 1. Recover Task
            let recovered_task = client2.get_task("p2_t", &task_id).await.unwrap();
            assert_eq!(recovered_task.id, task_id);
            assert_eq!(recovered_task.status, TaskStatus::Succeeded);
            assert_eq!(recovered_task.epoch, 3);

            // 2. Recover Session & Journal
            let session_list = client2.list_sessions("p2_sl").await.unwrap();
            assert_eq!(session_list.len(), 1);
            let journal = client2.get_session_journal("p2_j", &session_id_str).await.unwrap();
            assert_eq!(journal.len(), 1);
            assert_eq!(journal[0].entry_data, "Message before reboot");

            // 3. Continue Session Operations Post-Crash
            client2.append_session_message("p2_m2", &session_id_str, "user", "Message after reboot").await.unwrap();
            let journal_after = client2.get_session_journal("p2_ja", &session_id_str).await.unwrap();
            assert_eq!(journal_after.len(), 2);

            // 4. Create New Task Post-Recovery
            let new_task = client2.create_task("p2_new", "Post-Recovery Task", None, None).await.unwrap();
            assert_eq!(new_task.title, "Post-Recovery Task");

            let all_tasks = client2.list_tasks("p2_all").await.unwrap();
            assert_eq!(all_tasks.len(), 2);
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
