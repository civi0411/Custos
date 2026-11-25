-- Migration 0015: Replan Records (RFC 004 §4 / RFC 005)
--
-- Persists replan briefs and trigger events when assumptions fail or replanning is requested.

CREATE TABLE IF NOT EXISTS replan_records (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    trigger TEXT NOT NULL,
    failed_node_id TEXT,
    reason TEXT NOT NULL,
    brief_json TEXT NOT NULL,
    new_proposal_id TEXT NOT NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_replan_records_task_id ON replan_records(task_id);
CREATE INDEX IF NOT EXISTS idx_replan_records_created_at ON replan_records(created_at);
