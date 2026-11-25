-- Command Ledger v2: state tracking (pending | completed), scope isolation, and integer millisecond expiration.
ALTER TABLE command_ledger ADD COLUMN state TEXT NOT NULL DEFAULT 'completed';
ALTER TABLE command_ledger ADD COLUMN scope TEXT;
ALTER TABLE command_ledger ADD COLUMN expires_at_ms INTEGER;

CREATE INDEX IF NOT EXISTS idx_command_ledger_state ON command_ledger (state);
CREATE INDEX IF NOT EXISTS idx_command_ledger_expires_ms ON command_ledger (expires_at_ms);
