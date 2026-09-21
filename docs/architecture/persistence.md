# Persistence Architecture & Storage

> **Status:** Canonical Baseline v4.0  
> **Source:** Part V (§21) Canonical Specification

Custos adheres strictly to the **Local-First & Durable by Design** principle: all state data, event histories, and generated artifacts are durably persisted on the local workstation with ACID transaction guarantees and instant post-crash recovery.

---

## 1. Storage Pillars

1. **SQLite (Core Metadata & Event Store):** Embedded SQLite configured with **Write-Ahead Logging (`WAL`)** mode, safe lock timeouts (`busy_timeout = 5000ms`), and normalized foreign key constraints. Guarantees transactional atomicity and high concurrent read performance.
2. **Content-Addressable Storage (CAS):** Large payloads (file diffs, binary blobs, outcome bundles, test execution logs) are stored outside SQLite in the dedicated CAS directory (`.custos/cas/`), indexed directly by their `SHA-256` content hash.
3. **Transactional Outbox Pattern:** External side effects (dispatching webhooks, broadcasting events to client listeners) are staged in the `outbox_events` table within the same atomic database transaction as the domain event, ensuring *At-Least-Once Delivery*.

---

## 2. Core Schema DDL

```sql
-- Enable WAL mode and foreign key constraints
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

-- 1. Tasks Table
CREATE TABLE IF NOT EXISTS tasks (
    task_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    intent TEXT NOT NULL,
    status TEXT NOT NULL,
    contract_json TEXT NOT NULL,
    budget_tokens_limit INTEGER NOT NULL,
    budget_usd_limit REAL NOT NULL,
    tokens_consumed INTEGER DEFAULT 0,
    cost_usd_consumed REAL DEFAULT 0.0,
    worktree_path TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 2. Immutable Event Store
CREATE TABLE IF NOT EXISTS task_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    sequence_no INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    UNIQUE(task_id, sequence_no)
);

-- 3. Runs Table
CREATE TABLE IF NOT EXISTS runs (
    run_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    run_number INTEGER NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    UNIQUE(task_id, run_number)
);

-- 4. Execution Steps Table
CREATE TABLE IF NOT EXISTS steps (
    step_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(run_id) ON DELETE CASCADE,
    step_number INTEGER NOT NULL,
    role_name TEXT NOT NULL,
    status TEXT NOT NULL,
    action_type TEXT NOT NULL,
    action_payload_hash TEXT,
    receipt_id TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(run_id, step_number)
);

-- 5. Decision Ledger Table
CREATE TABLE IF NOT EXISTS decision_ledger (
    decision_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    step_id TEXT REFERENCES steps(step_id),
    question_id TEXT NOT NULL,
    decision_outcome TEXT NOT NULL,
    confidence_score REAL NOT NULL,
    rationale TEXT NOT NULL,
    decided_by TEXT NOT NULL, -- 'system_one' or 'human_principal'
    created_at TEXT NOT NULL
);

-- 6. Outbox Events Table
CREATE TABLE IF NOT EXISTS outbox_events (
    outbox_id INTEGER PRIMARY KEY AUTOINCREMENT,
    destination TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT DEFAULT 'PENDING',
    created_at TEXT NOT NULL,
    sent_at TEXT
);
```

---

## 3. Backup & Disaster Recovery Strategy

- **Online Hot Backups:** Custos utilizes the SQLite Backup API to snapshot the active database periodically into `.custos/backup/custos_snapshot.db` without locking read or write threads.
- **CAS Integrity:** The Content-Addressable Storage directory is append-only and can be synchronized, archived, or restored using standard file synchronization utilities (`rsync`, `rclone`).
