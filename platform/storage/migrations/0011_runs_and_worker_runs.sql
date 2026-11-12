-- Migration 0011: Runs and Worker Runs (Execution Spine Persistence)
--
-- Persists sovereign task execution attempts (Runs) and bounded worker executions (WorkerRuns).

CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    workflow_revision TEXT,
    status TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    current_span_num INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    metadata_json TEXT,
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE TABLE IF NOT EXISTS worker_runs (
    id TEXT PRIMARY KEY,
    run_id TEXT,
    task_id TEXT NOT NULL,
    worker_id TEXT NOT NULL,
    attempt_id INTEGER NOT NULL,
    max_attempts INTEGER NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    continuation_packet_ref TEXT,
    FOREIGN KEY(run_id) REFERENCES runs(id),
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_runs_task_id ON runs(task_id);
CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status);
CREATE INDEX IF NOT EXISTS idx_worker_runs_run_id ON worker_runs(run_id);
CREATE INDEX IF NOT EXISTS idx_worker_runs_task_id ON worker_runs(task_id);
