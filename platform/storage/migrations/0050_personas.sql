-- Sovereign Personas, Action Queue Triage, and Multi-Persona Rooms

CREATE TABLE IF NOT EXISTS personas (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    default_workbench TEXT NOT NULL,
    memory_scope TEXT NOT NULL UNIQUE,
    allowed_triggers_json TEXT NOT NULL,
    acts_for_policy_json TEXT NOT NULL,
    sovereign_instructions TEXT NOT NULL DEFAULT '',
    look_around_job_id TEXT,
    max_up_next_items INTEGER NOT NULL DEFAULT 5,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS persona_items (
    id TEXT PRIMARY KEY,
    persona_id TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
    bucket TEXT NOT NULL CHECK (bucket IN ('needs_you', 'up_next', 'handled')),
    kind TEXT NOT NULL CHECK (kind IN ('permit_approval', 'hook_permission', 'memory_proposal', 'look_around_finding', 'task_result')),
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    ask_questions_json TEXT NOT NULL DEFAULT '[]',
    source_kind TEXT,
    source_id TEXT,
    task_id TEXT,
    run_id TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_persona_items_bucket
    ON persona_items (bucket, priority DESC, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_persona_items_persona_bucket
    ON persona_items (persona_id, bucket, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_persona_items_source
    ON persona_items (source_kind, source_id);

CREATE TABLE IF NOT EXISTS persona_rooms (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    member_persona_ids_json TEXT NOT NULL,
    default_workbench TEXT,
    active_session_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS persona_room_messages (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL REFERENCES persona_rooms(id) ON DELETE CASCADE,
    sender TEXT NOT NULL,
    sender_kind TEXT NOT NULL CHECK (sender_kind IN ('owner', 'persona', 'system')),
    target_persona_id TEXT,
    root_request_id TEXT,
    content TEXT NOT NULL,
    task_id TEXT,
    run_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_persona_room_messages_room
    ON persona_room_messages (room_id, created_at ASC);

