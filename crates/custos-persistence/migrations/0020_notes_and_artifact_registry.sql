-- Migration 0020: Personal Notes, Scratchpads, and Lineage Index
--
-- Adds SQLite persistence for versioned notes and scratchpads with cryptographic hashes.

CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    version INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    session_id TEXT,
    task_id TEXT,
    tags_json TEXT NOT NULL DEFAULT '[]',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notes_session ON notes(session_id);
CREATE INDEX IF NOT EXISTS idx_notes_task ON notes(task_id);
CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated_at);

CREATE TABLE IF NOT EXISTS note_versions (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_note_versions_note ON note_versions(note_id, version);
