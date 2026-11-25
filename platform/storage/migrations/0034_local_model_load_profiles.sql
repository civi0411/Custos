CREATE TABLE IF NOT EXISTS local_model_load_profiles (
    model_path TEXT PRIMARY KEY,
    revision INTEGER NOT NULL,
    context_size INTEGER NOT NULL,
    gpu_mode TEXT NOT NULL CHECK(gpu_mode IN ('auto', 'max', 'off', 'manual')),
    gpu_layers INTEGER,
    eval_batch_size INTEGER,
    cpu_threads INTEGER,
    flash_attention INTEGER,
    mmap INTEGER NOT NULL DEFAULT 1,
    mlock INTEGER NOT NULL DEFAULT 0,
    idle_timeout_seconds INTEGER,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(model_path) REFERENCES local_model_cache(file_path) ON DELETE CASCADE
);
