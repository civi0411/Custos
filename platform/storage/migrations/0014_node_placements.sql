-- Migration 0014: Node Placements (RFC 004 §2C)
--
-- Persists role-to-backend placements, token budget slices, and workspace lease IDs per revision node.

CREATE TABLE IF NOT EXISTS node_placements (
    revision_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    role TEXT NOT NULL,
    backend_harness TEXT NOT NULL,
    budget_tokens_slice INTEGER NOT NULL,
    workspace_lease_id TEXT,
    PRIMARY KEY (revision_id, node_id),
    FOREIGN KEY(revision_id) REFERENCES workflow_revisions(revision_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_node_placements_revision_id ON node_placements(revision_id);
CREATE INDEX IF NOT EXISTS idx_node_placements_lease_id ON node_placements(workspace_lease_id);
