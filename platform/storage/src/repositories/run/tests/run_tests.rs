use custos_domain::{
    ClaimStatus, DispatchClaim, DomainError, LaunchAttempt, LaunchStatus, Run, RunStatus, Task,
    WorkerRun,
};

use crate::connection::DbConnection;
use crate::migrations::run_migrations;
use crate::repositories::run::RunRepository;

#[test]
fn run_and_worker_updates_reject_stale_writers_and_restart_revival() {
    let db = DbConnection::open_in_memory().unwrap();
    {
        let conn = db.lock().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
        conn.execute("INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES ('cas_task', 'CAS', 'running', 'now', 'now')", []).unwrap();
    }
    let repo = RunRepository::new(db);
    let mut run = Run::new("cas_task".into(), 1);
    run.transition(RunStatus::Active).unwrap();
    repo.save_run(&mut run).unwrap();
    assert_eq!(run.state_version, 1);
    let mut stale = repo.get_run(&run.id).unwrap().unwrap();
    run.metadata["output"] = serde_json::json!("new");
    repo.save_run(&mut run).unwrap();
    assert_eq!(run.state_version, 2);
    stale.attempt = 2;
    assert!(matches!(
        repo.save_run(&mut stale),
        Err(DomainError::Conflict(_))
    ));
    assert_eq!(stale.state_version, 1);
    assert_eq!(
        repo.get_run(&run.id).unwrap().unwrap().metadata["output"],
        "new"
    );

    let mut worker = WorkerRun::new("cas_task".into(), "actor".into(), 3).with_run_id(&run.id);
    worker.transition(RunStatus::Active).unwrap();
    repo.save_worker_run(&mut worker).unwrap();
    let mut stale_worker = repo.get_worker_run(&worker.id).unwrap().unwrap();
    worker.record_attempt().unwrap();
    repo.save_worker_run(&mut worker).unwrap();
    assert!(matches!(
        repo.save_worker_run(&mut stale_worker),
        Err(DomainError::Conflict(_))
    ));

    repo.reconcile_runs_on_startup().unwrap();
    assert!(matches!(
        repo.save_run(&mut run),
        Err(DomainError::Conflict(_))
    ));
    assert!(matches!(
        repo.save_worker_run(&mut worker),
        Err(DomainError::Conflict(_))
    ));
    let mut terminal = repo.get_run(&run.id).unwrap().unwrap();
    terminal.status = RunStatus::Active;
    assert!(matches!(
        repo.save_run(&mut terminal),
        Err(DomainError::Conflict(_))
    ));
    let mut forged = Run::new("cas_task".into(), 1);
    forged.state_version = 1;
    assert!(matches!(
        repo.save_run(&mut forged),
        Err(DomainError::Conflict(_))
    ));
}

#[test]
fn test_durable_dispatch_claim_and_fencing() {
    let db = DbConnection::open_in_memory().unwrap();
    let task = Task::new("task_fencing_test_1".to_string(), "Test Task".to_string());
    {
        let c = db.lock().unwrap();
        run_migrations(&c).unwrap();
        c.execute(
            "INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES (?1, ?2, 'running', 'now', 'now')",
            rusqlite::params![task.id, task.title],
        ).unwrap();
    }

    let repo = RunRepository::new(db);

    // 1. First claim succeeds
    let claim1 = DispatchClaim::new(&task.id, "worker_1", 1);
    repo.claim_ready(&claim1).expect("First claim must succeed");

    let loaded = repo.get_claim(&claim1.id).unwrap().unwrap();
    assert_eq!(loaded.assignee, "worker_1");
    assert_eq!(loaded.status, ClaimStatus::Pending);

    // Active claim lookup matches
    let active = repo.get_active_claim(&task.id, None).unwrap().unwrap();
    assert_eq!(active.id, claim1.id);

    // 2. Second claim on same task while first is active fails with Conflict
    let claim2 = DispatchClaim::new(&task.id, "worker_2", 1);
    let err = repo
        .claim_ready(&claim2)
        .expect_err("Second claim must conflict");
    assert!(matches!(err, DomainError::Conflict(_)));

    // 3. Releasing first claim allows second claim to succeed
    let mut released_claim1 = claim1.clone();
    released_claim1.release().unwrap();
    repo.update_claim(&released_claim1).unwrap();

    repo.claim_ready(&claim2)
        .expect("Claim must succeed after release");
    let active2 = repo.get_active_claim(&task.id, None).unwrap().unwrap();
    assert_eq!(active2.assignee, "worker_2");

    // 4. CAS: Attempting to mark a released claim as dispatched fails with Conflict
    let mut stale_dispatch = released_claim1.clone();
    stale_dispatch.status = ClaimStatus::Dispatched;
    stale_dispatch.worker_run_id = Some("wrun_stale".to_string());
    let cas_err = repo
        .update_claim(&stale_dispatch)
        .expect_err("Stale dispatch must fail with CAS conflict");
    assert!(matches!(cas_err, DomainError::Conflict(_)));

    // 5. Idempotent release succeeds
    repo.update_claim(&released_claim1)
        .expect("Idempotent release must succeed");
}

#[test]
fn test_durable_launch_attempt_persistence() {
    let db = DbConnection::open_in_memory().unwrap();
    {
        let c = db.lock().unwrap();
        run_migrations(&c).unwrap();
        c.execute(
            "INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES ('task_launch_test', 'Launch Task', 'running', 'now', 'now')",
            [],
        ).unwrap();
        c.execute(
            "INSERT INTO runs (id, task_id, status, attempt, current_span_num, started_at) VALUES ('run_1', 'task_launch_test', 'active', 1, 1, 'now')",
            [],
        ).unwrap();
        c.execute(
            "INSERT INTO worker_runs (id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at) VALUES ('wrun_1', 'run_1', 'task_launch_test', 'worker_alpha', 1, 3, 'active', 'now')",
            [],
        ).unwrap();
    }

    let repo = RunRepository::new(db);
    let mut attempt = LaunchAttempt::new("wrun_1", "model").unwrap();
    attempt
        .mark_launched("model", Some("claude-sonnet".into()))
        .unwrap();

    repo.save_launch_attempt(&attempt).unwrap();

    let loaded = repo.get_launch_attempt(&attempt.id).unwrap().unwrap();
    assert_eq!(loaded.worker_run_id, "wrun_1");
    assert_eq!(loaded.actual_mode.as_deref(), Some("model"));
    assert_eq!(loaded.harness_id.as_deref(), Some("claude-sonnet"));
    assert_eq!(loaded.status, LaunchStatus::Launched);
}

#[test]
fn test_reconcile_runs_on_startup() {
    let db = DbConnection::open_in_memory().unwrap();
    {
        let c = db.lock().unwrap();
        run_migrations(&c).unwrap();
        c.execute(
            "INSERT INTO tasks (id, title, status, created_at, updated_at) VALUES ('task_rec_test', 'Rec Task', 'running', 'now', 'now')",
            [],
        ).unwrap();
        c.execute(
            "INSERT INTO runs (id, task_id, status, attempt, current_span_num, started_at) VALUES ('run_stale', 'task_rec_test', 'active', 1, 1, 'now')",
            [],
        ).unwrap();
        c.execute(
            "INSERT INTO worker_runs (id, run_id, task_id, worker_id, attempt_id, max_attempts, status, started_at) VALUES ('wrun_stale', 'run_stale', 'task_rec_test', 'worker_alpha', 1, 3, 'active', 'now')",
            [],
        ).unwrap();
        c.execute(
            "INSERT INTO dispatch_claims (id, task_id, node_id, assignee, depth, status, claimed_at) VALUES ('claim_stale', 'task_rec_test', '', 'worker_alpha', 1, 'dispatched', 'now')",
            [],
        ).unwrap();
        c.execute(
            "INSERT INTO dispatch_claims (id, task_id, node_id, assignee, depth, status, claimed_at) VALUES ('claim_remote', 'task_rec_test', 'node_ext_1', 'worker_beta', 1, 'dispatched', 'now')",
            [],
        ).unwrap();
    }

    let repo = RunRepository::new(db);
    let reconciled_count = repo
        .reconcile_runs_on_startup()
        .expect("Reconcile should succeed");
    assert_eq!(reconciled_count, 2);

    let wrun = repo.get_worker_run("wrun_stale").unwrap().unwrap();
    assert_eq!(wrun.status, RunStatus::Failed);

    let run = repo.get_run("run_stale").unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Failed);

    // Local in-process claim was released
    let claim = repo.get_claim("claim_stale").unwrap().unwrap();
    assert_eq!(claim.status, ClaimStatus::Released);

    // External node claim was preserved until probed
    let remote_claim = repo.get_claim("claim_remote").unwrap().unwrap();
    assert_eq!(remote_claim.status, ClaimStatus::Dispatched);

    // Explicit fencing transitions external claim to released
    repo.fence_claim("claim_remote").unwrap();
    let fenced_claim = repo.get_claim("claim_remote").unwrap().unwrap();
    assert_eq!(fenced_claim.status, ClaimStatus::Released);
}
