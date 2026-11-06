-- migrations/0044_universal_resource_registry.sql

CREATE TABLE IF NOT EXISTS resource_registry (
    uri             TEXT PRIMARY KEY,
    resource_kind   TEXT NOT NULL,
    component_id    TEXT NOT NULL,
    mime_type       TEXT NOT NULL,
    sha256_digest   TEXT,
    size_bytes      INTEGER,
    physical_path   TEXT,
    provenance_json TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_resource_registry_component ON resource_registry(component_id);
CREATE INDEX IF NOT EXISTS idx_resource_registry_digest ON resource_registry(sha256_digest);
