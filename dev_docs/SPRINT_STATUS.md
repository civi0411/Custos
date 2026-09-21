# Sprint Status & Sync (Vi & Truong)

> This document is used by both team members to track active work items and record blockers.  
> Mark completed tasks with `[x]`.

---

## Sprint 1: Local Kernel & SQLite (Active Sprint)
**Primary Goal:** Build the foundation for durable task persistence in SQLite and support local CLI task creation and inspection. No external AI model calls required.

### Truong's Deliverables (Software Engineer)
- [ ] Initialize `rusqlite` or `sqlx` connection pool in `crates/persistence-sqlite`.
- [ ] Implement schema migrations for: `tasks`, `spans`, `events`.
- [ ] Implement CRUD APIs: `insert_task(task: &Task) -> Result<(), PersistenceError>` and `get_task(id: &TaskId) -> Result<Option<Task>, PersistenceError>`.
- [ ] Use `clap` in `apps/custos-cli` to handle `custos run "<task description>"`.
- [ ] Connect CLI command to SQLite persistence to verify the end-to-end local flow.

### Vi's Deliverables (AI Engineer)
- [ ] Complete core fields for `Task` struct and `TaskStatus` enum in `crates/core-domain`.
- [ ] Define the communication schema (JSON schema / Action payload) between Task Kernel and Cognitive Arbiter.
- [ ] Initialize directory scaffold for `sidecars/python-judgment` (configured with `pyproject.toml` / `uv`).
- [ ] Initialize directory scaffold for `sidecars/ts-claude-agent` (configured with `package.json` / `pnpm`).

---

## Notes & Blockers

### Truong (SE)
- *Notes / Blockers:* None currently.

### Vi (AI Engineer)
- *Notes / Blockers:* None currently.
