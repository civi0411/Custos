-- Provider secret bytes live in the OS vault. These tables contain metadata only.
CREATE TABLE IF NOT EXISTS provider_credential_vault (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    vault_id TEXT NOT NULL,
    cleanup_pending INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS provider_credentials (
    provider_id TEXT PRIMARY KEY,
    metadata_json TEXT NOT NULL,
    generation TEXT NOT NULL,
    parts INTEGER NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('active','deleting'))
);
CREATE TABLE IF NOT EXISTS provider_credential_operations (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    operation_json TEXT NOT NULL
);
