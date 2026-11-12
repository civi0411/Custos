-- migrations/0045_shared_capabilities.sql
-- Ring 2 Shared Capabilities: artifact lineage & annotations, versioned notes,
-- interactive notebook cells/kernels, and independent reviewer records.

CREATE TABLE IF NOT EXISTS artifact_lineage (
    id                  TEXT PRIMARY KEY,
    artifact_path       TEXT NOT NULL,
    version             INTEGER NOT NULL,
    content_hash        TEXT NOT NULL,
    produced_by_run_id  TEXT,
    parent_version_hash TEXT,
    timestamp           INTEGER NOT NULL,
    UNIQUE(artifact_path, version)
);
CREATE INDEX IF NOT EXISTS idx_artifact_lineage_path ON artifact_lineage(artifact_path);
CREATE INDEX IF NOT EXISTS idx_artifact_lineage_hash ON artifact_lineage(content_hash);

CREATE TABLE IF NOT EXISTS artifact_annotations (
    id                   TEXT PRIMARY KEY,
    artifact_path        TEXT NOT NULL,
    artifact_version     INTEGER NOT NULL,
    target_json          TEXT NOT NULL,
    note                 TEXT NOT NULL,
    actor                TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'pending',
    side_chat_session_id TEXT,
    submitted_turn_id    TEXT,
    created_at           INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_artifact_annotations_artifact ON artifact_annotations(artifact_path, artifact_version);
CREATE INDEX IF NOT EXISTS idx_artifact_annotations_status ON artifact_annotations(status);
CREATE INDEX IF NOT EXISTS idx_artifact_annotations_side_chat ON artifact_annotations(side_chat_session_id);

CREATE TABLE IF NOT EXISTS notes (
    id           TEXT PRIMARY KEY,
    title        TEXT NOT NULL,
    content      TEXT NOT NULL,
    version      INTEGER NOT NULL DEFAULT 1,
    content_hash TEXT NOT NULL,
    session_id   TEXT,
    task_id      TEXT,
    tags_json    TEXT NOT NULL DEFAULT '[]',
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_notes_session ON notes(session_id);
CREATE INDEX IF NOT EXISTS idx_notes_task ON notes(task_id);
CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated_at);

CREATE TABLE IF NOT EXISTS note_versions (
    id           TEXT PRIMARY KEY,
    note_id      TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    version      INTEGER NOT NULL,
    content      TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_note_versions_note ON note_versions(note_id, version);

CREATE TABLE IF NOT EXISTS notebook_cells (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    cell_type       TEXT NOT NULL,
    source          TEXT NOT NULL,
    cell_index      INTEGER NOT NULL,
    execution_count INTEGER,
    status          TEXT NOT NULL,
    stdout          TEXT,
    stderr          TEXT,
    output_image    TEXT,
    wall_ms         INTEGER,
    epoch           INTEGER NOT NULL DEFAULT 1,
    updated_at      INTEGER NOT NULL,
    notebook_id     TEXT,
    cell_version    INTEGER DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_notebook_cells_session ON notebook_cells(session_id, cell_index);
CREATE INDEX IF NOT EXISTS idx_notebook_cells_scoped ON notebook_cells(session_id, notebook_id, cell_index);

CREATE TABLE IF NOT EXISTS notebook_kernel_sessions (
    session_id        TEXT PRIMARY KEY,
    epoch             INTEGER NOT NULL DEFAULT 1,
    status            TEXT NOT NULL DEFAULT 'idle',
    python_version    TEXT NOT NULL,
    execution_counter INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL,
    updated_at        INTEGER NOT NULL,
    notebook_id       TEXT,
    kernel_id         TEXT
);

CREATE TABLE IF NOT EXISTS reviewer_records (
    id               TEXT PRIMARY KEY,
    target_type      TEXT NOT NULL,
    target_id        TEXT NOT NULL,
    reviewer         TEXT NOT NULL,
    method           TEXT NOT NULL,
    status           TEXT NOT NULL,
    evidence_summary TEXT NOT NULL DEFAULT '',
    evidence_digest  TEXT,
    findings_json    TEXT NOT NULL DEFAULT '[]',
    is_fresh         INTEGER NOT NULL DEFAULT 1,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_reviewer_records_target ON reviewer_records(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_reviewer_records_status ON reviewer_records(status);
