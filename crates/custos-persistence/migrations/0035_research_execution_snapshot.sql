-- Preserve compute/checkpoint metadata while retaining indexed legacy projections.
ALTER TABLE research_execution_records ADD COLUMN record_json TEXT;
