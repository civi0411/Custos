-- migrations/0043_package_inventory.sql

CREATE TABLE IF NOT EXISTS package_inventory (
    component_id    TEXT NOT NULL,
    version         TEXT NOT NULL,
    channel         TEXT NOT NULL DEFAULT 'stable',
    state           TEXT NOT NULL,       -- 'builtin' | 'installed' | 'pending_install' | 'pending_remove' | 'failed'
    platform        TEXT,
    artifact_digest TEXT,
    installed_path  TEXT,
    activation_intent TEXT,
    enabled_in_profile TEXT,
    installed_at    TEXT,
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (component_id, version, platform)
);

CREATE TABLE IF NOT EXISTS package_activation_events (
    id              TEXT PRIMARY KEY,
    component_id    TEXT NOT NULL,
    version         TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    command_id      TEXT,
    detail_json     TEXT,
    occurred_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_package_activation_events_component
    ON package_activation_events(component_id, occurred_at DESC);

-- Seed base builtin
INSERT OR IGNORE INTO package_inventory
    (component_id, version, channel, state, platform, updated_at)
VALUES
    ('assistant', 'builtin', 'stable', 'builtin', 'any', strftime('%Y-%m-%dT%H:%M:%SZ','now'));

