-- Migration 0010: Harden Outbox & Effect Attempts (Gate 3 Hardening)
--
-- 1. Enforces uniqueness on idempotency_key for effect_attempts.
-- 2. Indexes node_attempt_id for fast attempt correlation.
-- 3. Composite index on outbox_entries(status, created_at) for efficient recovery sweeps.

DROP INDEX IF EXISTS idx_effects_idempotency;
CREATE UNIQUE INDEX IF NOT EXISTS uq_effects_idempotency ON effect_attempts(idempotency_key);

CREATE INDEX IF NOT EXISTS idx_effects_node_attempt ON effect_attempts(node_attempt_id);

CREATE INDEX IF NOT EXISTS idx_outbox_status_created ON outbox_entries(status, created_at);
