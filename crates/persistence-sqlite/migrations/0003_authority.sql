CREATE TABLE IF NOT EXISTS grants (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    capability TEXT NOT NULL,
    resource_pattern TEXT NOT NULL,
    max_risk TEXT NOT NULL,
    expires_at TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE TABLE IF NOT EXISTS permits (
    id TEXT PRIMARY KEY,
    grant_id TEXT NOT NULL,
    action_type TEXT NOT NULL,
    resource TEXT NOT NULL,
    allowed_operations TEXT NOT NULL DEFAULT '[]',
    single_use BOOLEAN NOT NULL DEFAULT 1,
    used BOOLEAN NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(grant_id) REFERENCES grants(id)
);

CREATE TABLE IF NOT EXISTS approval_requests (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    action_type TEXT NOT NULL,
    risk_class TEXT NOT NULL,
    justification TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    decided_at TEXT,
    decision_by TEXT,
    reason TEXT,
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    action_type TEXT NOT NULL,
    resource TEXT NOT NULL,
    risk_class TEXT NOT NULL,
    allowed BOOLEAN NOT NULL,
    decision_reason TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    prev_entry_hash TEXT NOT NULL,
    entry_hash TEXT NOT NULL
);
