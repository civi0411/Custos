-- Migration 0038: Experiment Attempts and Checkpoint Promotion Fencing (F10 / P3)

CREATE TABLE IF NOT EXISTS experiment_attempts (
    id TEXT PRIMARY KEY,
    experiment_id TEXT NOT NULL,
    recipe_id TEXT NOT NULL,
    backend TEXT NOT NULL,
    external_ref_json TEXT,
    exec_state TEXT NOT NULL,
    resource_state TEXT NOT NULL,
    parent_checkpoint_id TEXT,
    generation INTEGER NOT NULL DEFAULT 0,
    output_prefix TEXT NOT NULL,
    attempt_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_experiment_attempts_experiment_id ON experiment_attempts(experiment_id);
CREATE INDEX IF NOT EXISTS idx_experiment_attempts_exec_state ON experiment_attempts(exec_state);
CREATE INDEX IF NOT EXISTS idx_experiment_attempts_resource_state ON experiment_attempts(resource_state);

CREATE TABLE IF NOT EXISTS attempt_invocations (
    id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL,
    op TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    argv_fingerprint TEXT NOT NULL,
    outcome TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(attempt_id) REFERENCES experiment_attempts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_attempt_invocations_attempt_id ON attempt_invocations(attempt_id);

CREATE TABLE IF NOT EXISTS checkpoint_promotions (
    experiment_id TEXT NOT NULL,
    generation INTEGER NOT NULL,
    checkpoint_id TEXT NOT NULL,
    promoted_at INTEGER NOT NULL,
    PRIMARY KEY (experiment_id, generation)
);

CREATE INDEX IF NOT EXISTS idx_checkpoint_promotions_experiment ON checkpoint_promotions(experiment_id);
