-- Persisted MCP Server Configurations (Zero-Secret Control Plane)

CREATE TABLE IF NOT EXISTS mcp_servers (
    id TEXT PRIMARY KEY,
    prefix TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    transport_kind TEXT NOT NULL CHECK (transport_kind IN ('stdio', 'http')),
    command TEXT,
    args_json TEXT NOT NULL DEFAULT '[]',
    env_json TEXT NOT NULL DEFAULT '{}',
    cwd TEXT,
    url TEXT,
    headers_json TEXT NOT NULL DEFAULT '{}',
    auth_secret_ref TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled
    ON mcp_servers (enabled, prefix);

