-- Migration 0024: Scoped Browser, Remote Fleet, and Headless Automation
CREATE TABLE IF NOT EXISTS browser_sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    workspace_id TEXT,
    active_tab_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS browser_tabs (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    last_snapshot_json TEXT,
    console_logs_json TEXT,
    network_requests_json TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS remote_fleet_nodes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    user TEXT NOT NULL,
    auth_method_json TEXT NOT NULL,
    status TEXT NOT NULL,
    labels_json TEXT NOT NULL,
    last_ping_ms INTEGER,
    os_info TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS remote_fleet_executions (
    execution_id TEXT PRIMARY KEY,
    host_id TEXT NOT NULL,
    command TEXT NOT NULL,
    exit_code INTEGER,
    stdout TEXT NOT NULL,
    stderr TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    verified INTEGER NOT NULL DEFAULT 1,
    timestamp INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS headless_automation_jobs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    spec_json TEXT NOT NULL,
    trigger_json TEXT NOT NULL,
    status TEXT NOT NULL,
    exit_code INTEGER,
    output_log TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER
);
