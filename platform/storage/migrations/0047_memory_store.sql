-- Persistent Long-Term Memory Store & Governance Proposals (SADE Port 6)

CREATE TABLE IF NOT EXISTS memory_facts (
    id TEXT PRIMARY KEY,
    tier TEXT NOT NULL CHECK (tier IN ('working', 'episodic', 'semantic', 'procedural', 'judgment')),
    project_id TEXT,
    task_id TEXT,
    subject TEXT NOT NULL,
    predicate TEXT NOT NULL,
    object TEXT NOT NULL,
    content TEXT NOT NULL,
    confidence REAL NOT NULL,
    valid_from TEXT NOT NULL,
    valid_to TEXT,
    recorded_at TEXT NOT NULL,
    source_run_id TEXT,
    evidence_refs_json TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_memory_facts_scope
    ON memory_facts (task_id, project_id, recorded_at DESC);

CREATE INDEX IF NOT EXISTS idx_memory_facts_temporal
    ON memory_facts (subject, predicate, valid_from DESC);

CREATE VIRTUAL TABLE IF NOT EXISTS memory_facts_fts USING fts5(
    id UNINDEXED,
    content,
    subject,
    predicate,
    object
);

CREATE TABLE IF NOT EXISTS memory_fact_proposals (
    proposal_id TEXT PRIMARY KEY,
    worker_run_id TEXT NOT NULL,
    subject TEXT NOT NULL,
    predicate TEXT NOT NULL,
    object TEXT NOT NULL,
    confidence REAL NOT NULL,
    evidence_refs_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL CHECK (status IN ('pending', 'accepted', 'rejected')),
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_memory_proposals_status
    ON memory_fact_proposals (status, created_at DESC);

