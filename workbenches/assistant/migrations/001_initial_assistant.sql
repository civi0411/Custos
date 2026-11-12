-- Initial scoped schema for Assistant Workbench
-- Rule: Workbench owns its domain tables prefixed with mod_<name>_*

CREATE TABLE IF NOT EXISTS mod_assistant_drafts (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    recipient TEXT,
    subject TEXT,
    body_markdown TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

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
