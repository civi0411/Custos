-- Migration 0018: Research Recipes, Execution Records, and Artifact Annotations
--
-- Formalizes reproducible experiment definitions (Recipe) separated from
-- observed execution logs (ExecutionRecord) and persistent interactive annotations
-- (AnnotationRecord) with side-chat branching.

CREATE TABLE IF NOT EXISTS research_recipes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    command TEXT NOT NULL,
    environment_spec_json TEXT NOT NULL,
    inputs_json TEXT NOT NULL,
    outputs_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_research_recipes_name ON research_recipes(name);
CREATE INDEX IF NOT EXISTS idx_research_recipes_created ON research_recipes(created_at);

CREATE TABLE IF NOT EXISTS research_execution_records (
    id TEXT PRIMARY KEY,
    recipe_id TEXT NOT NULL REFERENCES research_recipes(id) ON DELETE CASCADE,
    session_id TEXT,
    status TEXT NOT NULL,
    exit_code INTEGER,
    stdout_cas_uri TEXT,
    stderr_cas_uri TEXT,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    artifacts_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_research_exec_recipe ON research_execution_records(recipe_id);
CREATE INDEX IF NOT EXISTS idx_research_exec_session ON research_execution_records(session_id);
CREATE INDEX IF NOT EXISTS idx_research_exec_status ON research_execution_records(status);

CREATE TABLE IF NOT EXISTS research_annotations (
    id TEXT PRIMARY KEY,
    artifact_path TEXT NOT NULL,
    artifact_version INTEGER NOT NULL,
    target_json TEXT NOT NULL,
    note TEXT NOT NULL,
    actor TEXT NOT NULL,
    status TEXT NOT NULL,
    side_chat_session_id TEXT,
    submitted_turn_id TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_research_annotations_artifact ON research_annotations(artifact_path, artifact_version);
CREATE INDEX IF NOT EXISTS idx_research_annotations_status ON research_annotations(status);
CREATE INDEX IF NOT EXISTS idx_research_annotations_side_chat ON research_annotations(side_chat_session_id);
