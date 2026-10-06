-- Migration 0016: Execution Workspaces (RFC 006 / OrCa Integration)
--
-- Persists isolated execution workspaces (Worktree, Folder, Remote SSH)
-- with lifecycle status, lineage, and domain metadata.

CREATE TABLE IF NOT EXISTS execution_workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind_type TEXT NOT NULL,
    kind_json TEXT NOT NULL,
    path TEXT NOT NULL,
    status TEXT NOT NULL,
    status_reason TEXT,
    lineage_json TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_execution_workspaces_status ON execution_workspaces(status);
CREATE INDEX IF NOT EXISTS idx_execution_workspaces_kind_type ON execution_workspaces(kind_type);
CREATE INDEX IF NOT EXISTS idx_execution_workspaces_created_at ON execution_workspaces(created_at);
