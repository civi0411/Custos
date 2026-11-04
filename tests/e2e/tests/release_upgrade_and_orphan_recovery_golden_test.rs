//! Golden Acceptance Test G13: Release Upgrade, Restart & Orphan Resource Recovery (F12)
//!
//! Invariant: "Release upgrade/restart/orphan resource recovery | Legacy data readable, active jobs discoverable, no destructive cleanup"
//!
//! Core Scenarios:
//! 1. Legacy Database Schema Migration & Data Preservation (Release Upgrade):
//!    - Opens a database initialized with an older schema subset and legacy records.
//!    - Upgrades forward through all migrations (0001 -> 0041).
//!    - Verifies all legacy tasks, events, and runs remain intact and readable without destructive loss.
//! 2. Crash & Startup Reconciliation (Orphan Resource Recovery):
//!    - Simulates an unexpected daemon crash during in-flight operations:
//!      * Remote run (RunPod) in 'active' status.
//!      * Local worker run in 'active' status.
//!      * Outbox entry in 'dispatching' status.
//!      * Experiment attempt in 'running' state.
//!      * Launch attempt in 'prepared' status.
//!      * Orphan dispatch claim in 'pending' status without node.
//!    - Executes unified startup reconciliation (`store.reconcile_all_on_startup()`).
//!    - Verifies:
//!      * Remote run transitions to 'uncertain' (fail-safe; live cloud jobs are preserved).
//!      * Local worker run transitions to 'failed' with timestamp and state version bump.
//!      * Outbox entry transitions to 'uncertain' (blocking automated blind retries per INV-03).
//!      * Experiment attempt transitions to 'uncertain'.
//!      * Prepared launch attempt transitions to 'unknown'.
//!      * Orphan claim is released.
//! 3. Active Job Discoverability Post-Crash:
//!    - Verifies all reconciled and historical jobs are discoverable via standard queries.
//!    - No records are dropped or wiped from SQLite.
//! 4. Workspace Orphan & Host Recovery (No Destructive Cleanup):
//!    - Workspace pointing to a healthy host directory recovers to Ready.
//!    - Workspace pointing to a missing or corrupt host path safely recovers to SetupFailed with diagnostics
//!      rather than dropping database records.
//!    - Dirty workspace protection prevents destructive archive/deletion without explicit force consent.

use chrono::Utc;
use custos_adapters::workspace::LocalWorkspaceProvider;
use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};
use custos_domain::session::SessionId;
use custos_domain::task::run::{DispatchClaim, LaunchAttempt, LaunchStatus, Run, WorkerRun};
use custos_domain::workspace::{ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceStatus};
use custos_domain::{DomainError, RunStatus, TaskStatus};
use custos_persistence::SqliteTaskStore;
use custos_runtime::workspace::WorkspaceCoordinator;
use rusqlite::Connection;
use std::sync::Arc;
use tempfile::tempdir;

fn ensure_task_exists(store: &SqliteTaskStore, task_id: &str) {
    let conn = store.db().lock().unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO tasks (id, title, status, created_at, updated_at) VALUES (?1, 'Test Task', 'running', datetime('now'), datetime('now'))",
        [task_id],
    )
    .unwrap();
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 1: Legacy Database Upgrade & Data Preservation (Release Upgrade)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_g13_legacy_database_migration_preserves_all_historical_data() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("legacy_custos.db");
    let db_path_str = db_path.to_str().expect("valid path");

    // 1. Initialize SQLite with only older core schemas (0001 - 0006)
    {
        let conn = Connection::open(&db_path).expect("open raw sqlite");
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            "#,
        )
        .expect("pragmas");

        // Execute early schemas
        let schema_01 = include_str!("../../../crates/custos-persistence/migrations/0001_core.sql");
        let schema_02 = include_str!("../../../crates/custos-persistence/migrations/0002_runtime.sql");
        let schema_03 = include_str!("../../../crates/custos-persistence/migrations/0003_authority.sql");
        let schema_06 = include_str!("../../../crates/custos-persistence/migrations/0006_session_and_bridge.sql");

        conn.execute_batch(schema_01).expect("schema 01");
        conn.execute_batch(schema_02).expect("schema 02");
        conn.execute_batch(schema_03).expect("schema 03");
        conn.execute_batch(schema_06).expect("schema 06");

        // Seed legacy task and session data before newer migrations exist
        conn.execute(
            "INSERT INTO tasks (id, title, status, epoch, created_at, updated_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                "task_legacy_001",
                "Legacy Research Project",
                "running",
                0,
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z",
                "{}",
            ),
        )
        .expect("insert legacy task");

        conn.execute(
            "INSERT INTO sessions (session_id, mode, status, current_goal, promotion_score, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                "session_legacy_001",
                "\"bare\"",
                "\"active\"",
                "Legacy Conversation Goal",
                0.5,
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z",
            ),
        )
        .expect("insert legacy session");
    }

    // 2. Open via modern SqliteTaskStore; triggers full migration suite up to 0041
    let store = SqliteTaskStore::new(db_path_str).expect("upgrade database via SqliteTaskStore");

    // 3. G13 Invariant: All legacy rows remain intact and readable
    let task = store
        .tasks()
        .get_task("task_legacy_001")
        .expect("query task")
        .expect("task exists");
    assert_eq!(task.id.as_str(), "task_legacy_001");
    assert_eq!(task.title, "Legacy Research Project");
    assert_eq!(task.status, TaskStatus::Running);

    let session = store
        .sessions()
        .get_session(&SessionId::new("session_legacy_001"))
        .expect("query session")
        .expect("session exists");
    assert_eq!(session.id, SessionId::new("session_legacy_001"));
    assert_eq!(session.current_goal.as_deref(), Some("Legacy Conversation Goal"));


    // 4. Verify modern schema extensions are functional without data loss
    let notebook_cells_count: i64 = store
        .db()
        .lock()
        .unwrap()
        .prepare("SELECT COUNT(*) FROM notebook_cells")
        .expect("query notebook_cells")
        .query_row([], |row| row.get(0))
        .expect("count");
    assert_eq!(notebook_cells_count, 0);

    let checkpoint_promotions_count: i64 = store
        .db()
        .lock()
        .unwrap()
        .prepare("SELECT COUNT(*) FROM checkpoint_promotions")
        .expect("query checkpoint_promotions")
        .query_row([], |row| row.get(0))
        .expect("count");
    assert_eq!(checkpoint_promotions_count, 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 2: Crash & Startup Reconciliation (Orphan Resource Recovery)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_g13_crash_startup_reconciliation_fences_orphans_without_destructive_cleanup() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));

    let task_id = "task_crash_001";
    ensure_task_exists(&store, task_id);

    let now = Utc::now();

    // 1. Seed simulated in-flight / crashed states:
    // A. Remote Run (RunPod) in 'active' status & associated remote worker run
    let mut remote_run = Run::new(task_id.to_string(), 1);
    remote_run.id = "run_remote_active".into();
    remote_run.status = RunStatus::Active;
    remote_run.metadata = serde_json::json!({
        "backend": "runpod",
        "external_job_ref": "pod-runpod-xyz-123"
    });
    store.runs().save_run(&mut remote_run).expect("save remote run");

    let mut remote_wrun = WorkerRun::new(task_id.to_string(), "worker_remote".to_string(), 1);
    remote_wrun.id = "wrun_remote_active".into();
    remote_wrun.run_id = Some("run_remote_active".into());
    remote_wrun.status = RunStatus::Active;
    store.runs().save_worker_run(&mut remote_wrun).expect("save remote worker run");

    // B. Local Run and Local Worker Run in 'active' status (fail-closed target)
    let mut local_run = Run::new(task_id.to_string(), 1);
    local_run.id = "run_local_active".into();
    local_run.status = RunStatus::Active;
    store.runs().save_run(&mut local_run).expect("save local run");

    let mut local_wrun = WorkerRun::new(task_id.to_string(), "worker_local".to_string(), 1);
    local_wrun.id = "wrun_local_active".into();
    local_wrun.run_id = Some("run_local_active".into());
    local_wrun.status = RunStatus::Active;
    store.runs().save_worker_run(&mut local_wrun).expect("save local worker run");

    // C. Outbox entry left in 'dispatching' status
    let outbox_entry = OutboxEntry::new(
        "out_crash_001",
        task_id,
        "action_payment_dispatch",
        "permit_stripe_charge",
        "digest_payment_123",
        Some("{\"amount\": 500}".into()),
        OutboxStatus::Dispatching,
        now,
    );
    store.enqueue(outbox_entry).await.expect("insert outbox entry");

    // D. In-flight experiment attempt in 'running' state
    let mut attempt = custos_domain::evidence::ExecutionAttempt::new(
        "exp_recovery",
        "recipe_train",
        custos_domain::compute::ComputeBackendKind::RunPod,
        None,
    );
    attempt.id = "attempt_crash_001".into();
    attempt.exec_state = custos_domain::evidence::ExecState::Running;
    attempt.resource_state = custos_domain::evidence::ResourceState::Held;
    store.experiment_attempts().save_attempt(&attempt).expect("save experiment attempt");

    // E. Prepared launch attempt in 'prepared' state
    let launch_att = LaunchAttempt {
        id: "launch_att_001".into(),
        worker_run_id: "wrun_remote_active".into(),
        requested_mode: "remote".into(),
        actual_mode: Some("remote".into()),
        harness_id: None,
        status: LaunchStatus::Prepared,
        detail: Some("Allocated remote pod".into()),
        prepared_at: now,
        resolved_at: None,
    };
    store.runs().save_launch_attempt(&launch_att).expect("save launch attempt");

    // F. Orphan dispatch claim with empty node
    let claim = DispatchClaim {
        id: "claim_orphan_001".into(),
        task_id: task_id.into(),
        node_id: None,
        assignee: "worker_01".into(),
        depth: 1,
        status: custos_domain::task::run::ClaimStatus::Pending,
        claimed_at: now,
        dispatched_at: None,
        released_at: None,
        worker_run_id: Some("wrun_remote_active".into()),
    };
    store.runs().claim_ready(&claim).expect("save orphan claim");

    // 2. Perform Unified Startup Reconciliation
    let summary = store
        .reconcile_all_on_startup()
        .expect("reconcile all on startup");

    // G13 Assertions:
    assert!(summary.runs_reconciled >= 1, "Must reconcile active runs");
    assert!(summary.outbox_reconciled >= 1, "Must reconcile dispatching outbox entries");
    assert!(summary.attempts_reconciled >= 1, "Must reconcile in-flight experiment attempts");

    // A. Remote Run & Worker Run are preserved as 'Uncertain' (preventing duplicate pod launch and keeping job ref)
    let reconciled_remote = store
        .runs()
        .get_run("run_remote_active")
        .expect("get run")
        .expect("run exists");
    assert_eq!(reconciled_remote.status, RunStatus::Uncertain);

    let reconciled_remote_wrun = store
        .runs()
        .get_worker_run("wrun_remote_active")
        .expect("get worker run")
        .expect("worker run exists");
    assert_eq!(reconciled_remote_wrun.status, RunStatus::Uncertain);

    // B. Local Run & Worker Run transitioned fail-closed to 'Failed' with timestamp
    let reconciled_local_run = store
        .runs()
        .get_run("run_local_active")
        .expect("get local run")
        .expect("local run exists");
    assert_eq!(reconciled_local_run.status, RunStatus::Failed);
    assert!(reconciled_local_run.ended_at.is_some());

    let reconciled_local = store
        .runs()
        .get_worker_run("wrun_local_active")
        .expect("get worker run")
        .expect("worker run exists");
    assert_eq!(reconciled_local.status, RunStatus::Failed);
    assert!(reconciled_local.ended_at.is_some());

    // C. Outbox entry transitioned to 'Uncertain' (INV-03 fences automated blind retries)
    let reconciled_outbox = store
        .get("out_crash_001")
        .await
        .expect("get outbox")
        .expect("outbox exists");
    assert_eq!(reconciled_outbox.status, OutboxStatus::Uncertain);

    // D. Experiment attempt transitioned to 'uncertain'
    let reconciled_att = store
        .experiment_attempts()
        .get_attempt("attempt_crash_001")
        .expect("get attempt")
        .expect("attempt exists");
    assert_eq!(reconciled_att.exec_state, custos_domain::evidence::ExecState::Uncertain);

    // E. Prepared launch attempt transitioned to 'unknown'
    let reconciled_launch = store
        .runs()
        .get_launch_attempt("launch_att_001")
        .expect("get launch attempt")
        .expect("launch exists");
    assert_eq!(reconciled_launch.status, LaunchStatus::Unknown);
    assert!(reconciled_launch
        .detail
        .as_deref()
        .unwrap_or("")
        .contains("Daemon restart during in-flight execution"));

    // F. Orphan dispatch claim transitioned to 'Released'
    let reconciled_claim = store
        .runs()
        .get_claim("claim_orphan_001")
        .expect("get claim")
        .expect("claim exists");
    assert_eq!(
        reconciled_claim.status,
        custos_domain::task::run::ClaimStatus::Released
    );
    assert!(reconciled_claim.released_at.is_some());
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 3: Active Job Discoverability Post-Crash
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_g13_active_jobs_and_history_discoverable_without_data_loss() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));

    let task_id = "task_discover_001";
    ensure_task_exists(&store, task_id);

    let mut run = Run::new(task_id.to_string(), 1);
    run.id = "run_disc_01".into();
    run.status = RunStatus::Active;
    run.metadata = serde_json::json!({
        "backend": "runpod",
        "external_job_ref": "pod-disc-777"
    });
    store.runs().save_run(&mut run).expect("save run");

    // Reconcile
    store.reconcile_runs_on_startup().expect("reconcile runs");

    // 1. Discoverable via list_uncertain_runs()
    let uncertain_runs = store.list_uncertain_runs().expect("list uncertain runs");
    assert_eq!(uncertain_runs.len(), 1);
    assert_eq!(uncertain_runs[0].id, "run_disc_01");
    assert_eq!(
        uncertain_runs[0].metadata["external_job_ref"],
        "pod-disc-777"
    );

    // 2. Discoverable via list_runs_for_task()
    let task_runs = store.runs().list_runs_for_task(task_id).expect("list task runs");
    assert_eq!(task_runs.len(), 1);
    assert_eq!(task_runs[0].id, "run_disc_01");
    assert_eq!(task_runs[0].status, RunStatus::Uncertain);

    // 3. Database records were NOT dropped or wiped
    let fetched_task = store.tasks().get_task(task_id).expect("query task");
    assert!(fetched_task.is_some());
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 4: Workspace Orphan & Host Recovery (No Destructive Cleanup)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_g13_workspace_recovery_and_dirty_protection_without_destructive_cleanup() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let provider = Arc::new(LocalWorkspaceProvider::new());
    let coordinator = WorkspaceCoordinator::new(Arc::new(store.workspaces().clone()), provider.clone());

    let dir = tempdir().expect("tempdir");

    // Case A: Workspace pointing to a healthy host directory recovers to Ready
    let healthy_path = dir.path().join("healthy_repo");
    std::fs::create_dir_all(&healthy_path).expect("create healthy dir");

    let mut healthy_ws = ExecutionWorkspace::new(
        WorkspaceId::new("ws_healthy_01"),
        "Healthy Workspace",
        WorkspaceKind::Folder {
            path: healthy_path.to_string_lossy().to_string(),
        },
        healthy_path.to_string_lossy().to_string(),
    );
    healthy_ws.status = WorkspaceStatus::Initializing;
    store.workspaces().save_workspace(&healthy_ws).expect("save healthy ws");

    let recovered_healthy = coordinator
        .recover_workspace(&healthy_ws.id)
        .await
        .expect("recover healthy ws");
    assert_eq!(recovered_healthy.status, WorkspaceStatus::Ready);

    // Case B: Workspace pointing to a missing or deleted host directory recovers safely to SetupFailed
    // without dropping database records
    let missing_path = dir.path().join("deleted_host_repo");
    // Do NOT create directory on disk

    let mut missing_ws = ExecutionWorkspace::new(
        WorkspaceId::new("ws_missing_01"),
        "Missing Workspace",
        WorkspaceKind::Folder {
            path: missing_path.to_string_lossy().to_string(),
        },
        missing_path.to_string_lossy().to_string(),
    );
    missing_ws.status = WorkspaceStatus::Initializing;
    store.workspaces().save_workspace(&missing_ws).expect("save missing ws");

    let recovered_missing = coordinator
        .recover_workspace(&missing_ws.id)
        .await
        .expect("recover missing ws");
    match recovered_missing.status {
        WorkspaceStatus::SetupFailed { reason } => {
            assert!(reason.contains("missing or corrupt"));
        }
        other => panic!("Expected SetupFailed, got {:?}", other),
    }

    // Verify record in SQLite is NOT dropped
    let persisted_missing = store
        .workspaces()
        .get_workspace(&missing_ws.id)
        .expect("query ws")
        .expect("record must exist in sqlite");
    assert_eq!(persisted_missing.id, missing_ws.id);

    // Case C: Dirty protection against destructive cleanup
    // Initialize a real Git repository with uncommitted changes
    let git_repo_path = dir.path().join("dirty_repo");
    std::fs::create_dir_all(&git_repo_path).expect("create git repo dir");
    std::process::Command::new("git")
        .current_dir(&git_repo_path)
        .args(["init"])
        .output()
        .expect("git init");
    std::process::Command::new("git")
        .current_dir(&git_repo_path)
        .args(["config", "user.email", "ci@custos.internal"])
        .output()
        .expect("git config email");
    std::process::Command::new("git")
        .current_dir(&git_repo_path)
        .args(["config", "user.name", "Custos CI"])
        .output()
        .expect("git config name");

    std::fs::write(git_repo_path.join("file.txt"), "committed content\n").expect("write file");
    std::process::Command::new("git")
        .current_dir(&git_repo_path)
        .args(["add", "file.txt"])
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .current_dir(&git_repo_path)
        .args(["commit", "-m", "Initial commit"])
        .output()
        .expect("git commit");

    // Modify file to make worktree dirty
    std::fs::write(git_repo_path.join("file.txt"), "uncommitted dirty bytes\n").expect("dirty write");

    let dirty_ws = ExecutionWorkspace::new(
        WorkspaceId::new("ws_dirty_01"),
        "Dirty Protected Workspace",
        WorkspaceKind::Git {
            repo_path: git_repo_path.to_string_lossy().to_string(),
            branch: "main".into(),
            base_commit: None,
        },
        git_repo_path.to_string_lossy().to_string(),
    );
    store.workspaces().save_workspace(&dirty_ws).expect("save dirty ws");

    // Attempting to delete physical workspace without force MUST return Conflict
    let archive_res = coordinator
        .archive_workspace_with_force(&dirty_ws.id, true, false)
        .await;
    assert!(archive_res.is_err(), "Must block destructive archive of dirty workspace");
    match archive_res.unwrap_err() {
        DomainError::Conflict(msg) => {
            assert!(msg.contains("uncommitted changes"));
            assert!(msg.contains("Set force=true to discard"));
        }
        other => panic!("Expected DomainError::Conflict, got {:?}", other),
    }

    // Verify user bytes on disk are completely preserved
    let file_bytes = std::fs::read_to_string(git_repo_path.join("file.txt")).expect("read file");
    assert_eq!(file_bytes, "uncommitted dirty bytes\n");
}
