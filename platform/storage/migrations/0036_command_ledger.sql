CREATE TABLE IF NOT EXISTS command_ledger (
    command_id TEXT PRIMARY KEY,
    method TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    result_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_command_ledger_expires ON command_ledger (expires_at);
