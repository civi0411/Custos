-- Sovereign Assistant Workbench Schema
-- Tables: assistant_drafts, mod_assistant_outbox

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

CREATE TABLE IF NOT EXISTS mod_assistant_outbox (
    id TEXT PRIMARY KEY,
    draft_id TEXT,
    action_type TEXT NOT NULL,
    target TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    delivery_status TEXT NOT NULL DEFAULT 'pending',
    sent_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

