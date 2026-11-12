CREATE TABLE IF NOT EXISTS outbox_entries (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    permit_id TEXT NOT NULL,
    argument_digest TEXT NOT NULL,
    idempotency_key TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    receipt_json TEXT,
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE TABLE IF NOT EXISTS effect_attempts (
    id TEXT PRIMARY KEY,
    node_attempt_id TEXT NOT NULL,
    permit_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    receipt_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_outbox_status ON outbox_entries(status);
CREATE INDEX IF NOT EXISTS idx_outbox_task_id ON outbox_entries(task_id);
CREATE INDEX IF NOT EXISTS idx_effects_idempotency ON effect_attempts(idempotency_key);
