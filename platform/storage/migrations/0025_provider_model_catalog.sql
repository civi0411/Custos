-- Migration 0025: Provider Model Catalog & Probed Models Cache
--
-- Persists discovered provider models, context windows, default effort,
-- fast mode support, and pricing metadata.

CREATE TABLE IF NOT EXISTS provider_model_catalog (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    label TEXT NOT NULL,
    description TEXT,
    context_window INTEGER,
    default_effort TEXT,
    supports_fast_mode INTEGER NOT NULL DEFAULT 0,
    pricing_json TEXT,
    fetched_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_model_catalog_provider ON provider_model_catalog(provider_id);
