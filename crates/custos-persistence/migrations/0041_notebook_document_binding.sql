-- Migration 0041: Notebook Document & Kernel Binding Migration (SADE F06)
-- Adds notebook_id and cell_version to notebook_cells
-- Adds notebook_id and kernel_id to notebook_kernel_sessions

ALTER TABLE notebook_cells ADD COLUMN notebook_id TEXT;
ALTER TABLE notebook_cells ADD COLUMN cell_version INTEGER DEFAULT 1;

CREATE INDEX IF NOT EXISTS idx_notebook_cells_scoped ON notebook_cells(session_id, notebook_id, cell_index);

ALTER TABLE notebook_kernel_sessions ADD COLUMN notebook_id TEXT;
ALTER TABLE notebook_kernel_sessions ADD COLUMN kernel_id TEXT;
