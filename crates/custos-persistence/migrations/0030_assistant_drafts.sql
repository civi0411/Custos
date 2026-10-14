-- Migration 0030: Assistant Drafts (RFC 003 §4)
-- Structured draft storage for personal productivity, email, and meeting coordination with payload locking.

CREATE TABLE IF NOT EXISTS assistant_drafts (
    id TEXT PRIMARY KEY,
    task_id TEXT,
    session_id TEXT,
    title TEXT NOT NULL,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    recipient TEXT,
    status TEXT NOT NULL,
    payload_lock TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_assistant_drafts_session ON assistant_drafts(session_id);
CREATE INDEX IF NOT EXISTS idx_assistant_drafts_task ON assistant_drafts(task_id);
CREATE INDEX IF NOT EXISTS idx_assistant_drafts_status ON assistant_drafts(status);
