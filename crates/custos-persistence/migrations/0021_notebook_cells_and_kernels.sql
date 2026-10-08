-- Migration 0021: Notebook Cells, Executions, and Epoch Persistence
--
-- Tracks authorized kernel sessions, cell code, outputs, and epoch increments.

CREATE TABLE IF NOT EXISTS notebook_cells (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    cell_type TEXT NOT NULL,
    source TEXT NOT NULL,
    cell_index INTEGER NOT NULL,
    execution_count INTEGER,
    status TEXT NOT NULL,
    stdout TEXT,
    stderr TEXT,
    output_image TEXT,
    wall_ms INTEGER,
    epoch INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notebook_cells_session ON notebook_cells(session_id, cell_index);

CREATE TABLE IF NOT EXISTS notebook_kernel_sessions (
    session_id TEXT PRIMARY KEY,
    epoch INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL,
    python_version TEXT NOT NULL,
    execution_counter INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
