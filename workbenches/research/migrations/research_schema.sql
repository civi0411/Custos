-- Outbox messages table for transactionally committed research events
CREATE TABLE IF NOT EXISTS outbox_messages (
    id TEXT PRIMARY KEY,
    topic TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    published_at TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0
);

-- Migration 0017: Research Lineage, Sources, Cryptographic Passage Anchors, and Claims Matrix
--
-- Replaces unverified JSONL files and mtime sniffing with a deterministic SQLite Authority Ledger.

CREATE TABLE IF NOT EXISTS research_sources (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    title TEXT NOT NULL,
    doi TEXT,
    authors_json TEXT NOT NULL,
    year INTEGER,
    content_hash TEXT NOT NULL,
    local_path TEXT,
    verified INTEGER NOT NULL DEFAULT 0,
    abstract_text TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_research_sources_doi ON research_sources(doi);
CREATE INDEX IF NOT EXISTS idx_research_sources_hash ON research_sources(content_hash);

CREATE TABLE IF NOT EXISTS research_passage_anchors (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES research_sources(id) ON DELETE CASCADE,
    source_title TEXT,
    section_title TEXT,
    page_number INTEGER,
    start_offset INTEGER NOT NULL,
    end_offset INTEGER NOT NULL,
    exact_text TEXT NOT NULL,
    passage_hash TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_research_anchors_source ON research_passage_anchors(source_id);
CREATE INDEX IF NOT EXISTS idx_research_anchors_hash ON research_passage_anchors(passage_hash);

CREATE TABLE IF NOT EXISTS research_claims (
    id TEXT PRIMARY KEY,
    statement TEXT NOT NULL,
    level TEXT NOT NULL,
    confidence_score REAL NOT NULL DEFAULT 0.0,
    invariants_json TEXT NOT NULL,
    sealed_proof_uri TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_research_claims_level ON research_claims(level);

CREATE TABLE IF NOT EXISTS research_claim_evidence_links (
    id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL REFERENCES research_claims(id) ON DELETE CASCADE,
    passage_anchor_id TEXT NOT NULL REFERENCES research_passage_anchors(id) ON DELETE CASCADE,
    relation TEXT NOT NULL,
    rationale TEXT NOT NULL,
    verified_by TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_research_evidence_claim ON research_claim_evidence_links(claim_id);
CREATE INDEX IF NOT EXISTS idx_research_evidence_anchor ON research_claim_evidence_links(passage_anchor_id);

CREATE TABLE IF NOT EXISTS research_experiment_runs (
    run_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    command TEXT NOT NULL,
    cwd TEXT NOT NULL,
    status TEXT NOT NULL,
    wall_ms INTEGER NOT NULL,
    surface TEXT,
    reproducibility TEXT NOT NULL,
    input_merkle_root TEXT NOT NULL,
    output_merkle_root TEXT NOT NULL,
    env_snapshot_json TEXT NOT NULL,
    sade_permit_id TEXT,
    cas_log_uri TEXT,
    ts INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_research_runs_session ON research_experiment_runs(session_id);
CREATE INDEX IF NOT EXISTS idx_research_runs_status ON research_experiment_runs(status);
CREATE INDEX IF NOT EXISTS idx_research_runs_ts ON research_experiment_runs(ts);

-- Migration 0018: Research Recipes and Execution Records
--
-- Formalizes reproducible experiment definitions (Recipe) separated from
-- observed execution logs (ExecutionRecord).

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
    artifacts_json TEXT NOT NULL,
    record_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_research_exec_recipe ON research_execution_records(recipe_id);
CREATE INDEX IF NOT EXISTS idx_research_exec_session ON research_execution_records(session_id);
CREATE INDEX IF NOT EXISTS idx_research_exec_status ON research_execution_records(status);

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
