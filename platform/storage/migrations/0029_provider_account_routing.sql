-- Per-service automatic account selection; no credential bytes.
CREATE TABLE IF NOT EXISTS provider_account_routing (
    service_type TEXT PRIMARY KEY,
    round_robin INTEGER NOT NULL DEFAULT 0 CHECK(round_robin IN (0,1)),
    account_order TEXT NOT NULL DEFAULT '[]',
    cursor INTEGER NOT NULL DEFAULT 0
);
