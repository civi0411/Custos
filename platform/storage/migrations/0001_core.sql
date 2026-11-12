CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    epoch INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS spans (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    span_num INTEGER NOT NULL,
    state TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    input_digest TEXT NOT NULL,
    output_digest TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);

CREATE TABLE IF NOT EXISTS continuation_packets (
    task_id TEXT NOT NULL,
    from_span INTEGER NOT NULL,
    to_span INTEGER NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    task_summary TEXT NOT NULL,
    current_state TEXT NOT NULL,
    integrity_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY(task_id, to_span),
    FOREIGN KEY(task_id) REFERENCES tasks(id)
);
