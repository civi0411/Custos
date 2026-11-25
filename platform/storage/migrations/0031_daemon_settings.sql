-- Migration 0031: Daemon Settings Persistence
--
-- Stores explicit daemon settings. Runtime policy settings must be introduced
-- with a consumer that enforces them; this migration does not seed fake quotas.

CREATE TABLE IF NOT EXISTS daemon_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

