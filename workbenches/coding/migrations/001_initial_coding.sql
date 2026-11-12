-- Initial scoped schema for Coding Workbench
-- Rule: Workbench owns its domain tables prefixed with mod_<name>_*

CREATE TABLE IF NOT EXISTS mod_coding_worktrees (
    id TEXT PRIMARY KEY,
    repo_path TEXT NOT NULL,
    branch TEXT NOT NULL,
    worktree_path TEXT NOT NULL,
    is_dirty INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE IF NOT EXISTS mod_coding_reviews (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    patch_summary TEXT NOT NULL,
    passed INTEGER NOT NULL DEFAULT 0,
    report_markdown TEXT NOT NULL,
    reviewed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
