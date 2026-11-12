-- Migration 0013: Workflow Revisions (RFC 004 Orchestration Intelligence)
--
-- Persists compiled workflow revisions containing DAG nodes, dependencies, and obligations.

CREATE TABLE IF NOT EXISTS workflow_revisions (
    revision_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    proposal_id TEXT NOT NULL,
    revision_number INTEGER NOT NULL,
    nodes_json TEXT NOT NULL,
    dependencies_json TEXT NOT NULL,
    obligations_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_workflow_revisions_task_id ON workflow_revisions(task_id);
CREATE INDEX IF NOT EXISTS idx_workflow_revisions_proposal_id ON workflow_revisions(proposal_id);
