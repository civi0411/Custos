CREATE TABLE IF NOT EXISTS research_synthesis_proposals (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    claims_json TEXT NOT NULL DEFAULT '[]',
    recipes_json TEXT NOT NULL DEFAULT '[]',
    artifact_paths_json TEXT NOT NULL DEFAULT '[]',
    workspace_id TEXT,
    target_branch TEXT,
    caveats_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'draft',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_research_synthesis_status ON research_synthesis_proposals (status);
CREATE INDEX IF NOT EXISTS idx_research_synthesis_created ON research_synthesis_proposals (created_at DESC);
