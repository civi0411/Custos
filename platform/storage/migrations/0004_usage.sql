-- 0004_usage.sql
-- Custos Native Token Reservation and Cost Accounting Ledger
-- Conforms to Super Plan Section 2.4 and PR-02 requirements

CREATE TABLE IF NOT EXISTS usage_ledger (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    worker_run_id TEXT NOT NULL,
    attempt_id INTEGER NOT NULL DEFAULT 1,
    model TEXT NOT NULL,
    reserved_tokens INTEGER NOT NULL DEFAULT 0,
    settled_tokens INTEGER NOT NULL DEFAULT 0,
    cost_microcents INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL CHECK(status IN ('reserved', 'settled', 'unknown')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_usage_ledger_task_id ON usage_ledger(task_id);
CREATE INDEX IF NOT EXISTS idx_usage_ledger_status ON usage_ledger(status);

CREATE TABLE IF NOT EXISTS token_reservations (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    worker_id TEXT NOT NULL,
    amount INTEGER NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_token_reservations_task ON token_reservations(task_id);
