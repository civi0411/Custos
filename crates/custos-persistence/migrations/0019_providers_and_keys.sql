-- Migration 0019: Providers Configuration and Client Gateway API Keys
--
-- Persists configured AI providers, API tokens, and client gateway keys in SQLite.

CREATE TABLE IF NOT EXISTS providers_config (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    service_type TEXT NOT NULL,
    api_key_masked TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'configured',
    endpoint_url TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_providers_service ON providers_config(service_type);

CREATE TABLE IF NOT EXISTS client_api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    token TEXT NOT NULL,
    created_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_client_keys_revoked ON client_api_keys(revoked);
