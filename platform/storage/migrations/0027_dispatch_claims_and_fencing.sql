-- Migration 0027: Durable Dispatch Claims and Launch Fencing
--
-- Persists atomic dispatch claims, fences concurrent workers,
-- and records launch attempts with launch uncertainty tracking.

CREATE TABLE IF NOT EXISTS dispatch_claims (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    node_id TEXT,
    assignee TEXT NOT NULL,
    depth INTEGER NOT NULL,
    status TEXT NOT NULL,
    claimed_at TEXT NOT NULL,
    dispatched_at TEXT,
    released_at TEXT,
    worker_run_id TEXT,
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_active_dispatch_claims
    ON dispatch_claims (task_id, COALESCE(node_id, ''))
    WHERE status IN ('pending', 'dispatched');

CREATE INDEX IF NOT EXISTS idx_dispatch_claims_task_id ON dispatch_claims(task_id);
CREATE INDEX IF NOT EXISTS idx_dispatch_claims_status ON dispatch_claims(status);

CREATE TABLE IF NOT EXISTS launch_attempts (
    id TEXT PRIMARY KEY,
    worker_run_id TEXT NOT NULL,
    requested_mode TEXT NOT NULL,
    actual_mode TEXT,
    harness_id TEXT,
    status TEXT NOT NULL,
    detail TEXT,
    prepared_at TEXT NOT NULL,
    resolved_at TEXT,
    FOREIGN KEY(worker_run_id) REFERENCES worker_runs(id)
);

CREATE INDEX IF NOT EXISTS idx_launch_attempts_worker_run_id ON launch_attempts(worker_run_id);
