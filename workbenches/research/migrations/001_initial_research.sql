-- Initial scoped schema for Research Workbench
-- Rule: Workbench owns its domain tables prefixed with mod_<name>_*

CREATE TABLE IF NOT EXISTS mod_research_sources (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    authors_json TEXT,
    abstract_text TEXT,
    content_hash TEXT,
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE IF NOT EXISTS mod_research_claims (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    statement TEXT NOT NULL,
    grounding_level TEXT NOT NULL,
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY(source_id) REFERENCES mod_research_sources(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS mod_research_notes (
    id TEXT PRIMARY KEY,
    task_id TEXT,
    title TEXT NOT NULL,
    content_markdown TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
