-- Migration 0006: Session & Bridge Integration for Sovereign Architecture

CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    mode TEXT NOT NULL,
    status TEXT NOT NULL,
    current_goal TEXT,
    promotion_score REAL DEFAULT 0.0,
    attached_to TEXT,
    promoted_to TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session_journal (
    entry_id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    entry_type TEXT NOT NULL,
    entry_data TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session_messages (
    message_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    role TEXT NOT NULL,
    content_ref TEXT,
    tokens INTEGER DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS promotions (
    promotion_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    task_id TEXT NOT NULL REFERENCES tasks(id),
    trigger_data TEXT NOT NULL,
    promoted_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS attachments (
    attachment_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    mode TEXT NOT NULL,
    attached_at TEXT NOT NULL,
    detached_at TEXT
);
