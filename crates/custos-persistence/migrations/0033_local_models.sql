-- Local Model Folder Configuration
CREATE TABLE IF NOT EXISTS local_model_folders (
    id TEXT PRIMARY KEY,
    folder_path TEXT NOT NULL UNIQUE,
    label TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    auto_scan INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Local Discovered GGUF Models Cache
CREATE TABLE IF NOT EXISTS local_model_cache (
    file_path TEXT PRIMARY KEY,
    folder_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    file_mtime INTEGER NOT NULL,
    architecture TEXT NOT NULL,
    model_name TEXT NOT NULL,
    context_length INTEGER NOT NULL,
    quantization TEXT NOT NULL,
    chat_template TEXT,
    total_memory_required INTEGER NOT NULL,
    compatibility TEXT NOT NULL,
    scanned_at INTEGER NOT NULL,
    FOREIGN KEY(folder_id) REFERENCES local_model_folders(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_local_model_folder ON local_model_cache(folder_id);
