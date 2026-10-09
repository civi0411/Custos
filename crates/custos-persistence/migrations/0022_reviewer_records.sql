CREATE TABLE IF NOT EXISTS reviewer_records (
    id TEXT PRIMARY KEY,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    reviewer TEXT NOT NULL,
    method TEXT NOT NULL,
    status TEXT NOT NULL,
    evidence_summary TEXT NOT NULL,
    evidence_digest TEXT,
    findings_json TEXT NOT NULL DEFAULT '[]',
    is_fresh INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_reviewer_records_target ON reviewer_records (target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_reviewer_records_status ON reviewer_records (status);
