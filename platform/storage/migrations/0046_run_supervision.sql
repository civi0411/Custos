-- Migration 0046: Agent Run Execution Supervision Tables
-- Dedicated telemetry, step waterfall spans, structured logs, error grouping, and watchdog findings.
-- Deliberately separated from Task DAG spans (`spans`) and zero-coupling from specific workbenches.

CREATE TABLE IF NOT EXISTS supervision_runs (
    run_id TEXT PRIMARY KEY,
    workbench_id TEXT,
    agent TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'success', 'error')),
    started_at_ms INTEGER NOT NULL,
    ended_at_ms INTEGER,
    duration_ms INTEGER,
    tokens INTEGER,
    cost_usd REAL,
    usage_measurement_json TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_supervision_runs_workbench_started
    ON supervision_runs(workbench_id, started_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_supervision_runs_status_started
    ON supervision_runs(status, started_at_ms DESC);

CREATE TABLE IF NOT EXISTS run_step_spans (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    worker_run_id TEXT,
    parent_id TEXT,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'success', 'error')),
    started_at_ms INTEGER NOT NULL,
    ended_at_ms INTEGER,
    duration_ms INTEGER,
    metadata_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_run_step_spans_run_started
    ON run_step_spans(run_id, started_at_ms ASC);

CREATE INDEX IF NOT EXISTS idx_run_step_spans_branch_started
    ON run_step_spans(run_id, worker_run_id, parent_id, started_at_ms ASC);

CREATE TABLE IF NOT EXISTS supervision_logs (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    worker_run_id TEXT,
    span_id TEXT,
    timestamp_ms INTEGER NOT NULL,
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    metadata_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_supervision_logs_run_ts
    ON supervision_logs(run_id, timestamp_ms DESC);

CREATE TABLE IF NOT EXISTS supervision_errors (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    worker_run_id TEXT,
    span_id TEXT,
    error_type TEXT NOT NULL,
    message TEXT NOT NULL,
    stack TEXT,
    agent TEXT NOT NULL,
    signature_hash TEXT NOT NULL,
    timestamp_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_supervision_errors_run_ts
    ON supervision_errors(run_id, timestamp_ms DESC);

CREATE INDEX IF NOT EXISTS idx_supervision_errors_sig_ts
    ON supervision_errors(signature_hash, timestamp_ms DESC);

CREATE TABLE IF NOT EXISTS supervision_findings (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    worker_run_id TEXT,
    kind TEXT NOT NULL,
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    message TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    resolved_at_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_supervision_findings_run_created
    ON supervision_findings(run_id, created_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_supervision_findings_severity_created
    ON supervision_findings(severity, created_at_ms DESC);

