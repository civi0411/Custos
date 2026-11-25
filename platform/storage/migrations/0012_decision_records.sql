-- Migration 0012: Decision Records Ledger (RFC 003 Orchestration Intelligence Audit)
--
-- Persists immutable OI decision records capturing the DecisionSnapshot,
-- the StrategyProposal (chosen topology and alternatives), and decision latency overhead.

CREATE TABLE IF NOT EXISTS decision_records (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    proposal_json TEXT NOT NULL,
    overhead_ms INTEGER NOT NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_decision_records_task_id ON decision_records(task_id);
CREATE INDEX IF NOT EXISTS idx_decision_records_created_at ON decision_records(created_at);
