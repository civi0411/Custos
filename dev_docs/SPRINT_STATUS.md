# Sprint Status & Sync (Vi & Truong)

> **Sprint Cadence:** Sprint 1 — The Local Kernel & First Vertical Slice (`repo_explain`)  
> **Master Reference:** [dev_docs/README.md](./README.md)  
> **Tracking Branch:** All status updates and daily reports should be committed to the `report` branch.

---

## Sprint 1: Local Kernel & Vertical Slice (`repo_explain`)

**Sprint Goal:** Establish the durable task persistence foundation in SQLite and validate the end-to-end `repo_explain` vertical slice using `FakeProvider` (zero external API dependency, zero side-effect risk).

### 1. Truong's Deliverables (Platform & Software Engineering Lead)
- [ ] **SQLite Connection & Pool:** Initialize connection pool with WAL mode enabled in `crates/persistence-sqlite`.
- [ ] **Schema Migrations:** Write idempotent SQL migrations for `tasks`, `spans`, `domain_events`, and `outbox` tables.
- [ ] **Typed CRUD Operations:** Implement:
  - `insert_task(task: &Task) -> Result<(), PersistenceError>`
  - `get_task(id: &TaskId) -> Result<Option<Task>, PersistenceError>`
  - `append_event(event: &DomainEvent) -> Result<(), PersistenceError>`
- [ ] **CLI Commands:** Implement `custos run "<task description>"` and `custos status <task-id>` in `apps/custos-cli` using `clap`.
- [ ] **Local Daemon Skeleton:** Set up minimal `axum` HTTP server in `apps/custosd` / `crates/local-api` handling task creation requests.
- [ ] **Crash Test Harness:** Verify that a committed task survives daemon process restart (`kill -9` recovery test).

### 2. Vi's Deliverables (Product & AI/Research Lead)
- [ ] **Core Domain Models:** Define immutable domain structs and enums in `crates/core-domain`:
  - `Task`, `TaskId`, `TaskStatus`, `TaskContract`, `TaskBudget`
  - `DomainEvent`, `ActionIntent`, `Receipt`, `ExecutionPermit`, `ArtifactId`
- [ ] **ProviderPort & FakeProvider:**
  - Define `ProviderPort` trait in `crates/provider-sdk`.
  - Implement deterministic `FakeProvider` returning structured explanations with file/line citations for testing.
- [ ] **Citation Verifier:** Implement verifier logic in `crates/evidence-engine` checking that cited file paths and line ranges exist in the target repository snapshot.
- [ ] **Sidecar Scaffolding:**
  - Initialize directory scaffold for `sidecars/python-judgment` (configured with `pyproject.toml` and `uv`).
  - Initialize directory scaffold for `sidecars/ts-claude-agent` (configured with `package.json` and `pnpm`).

### 3. Joint Integration Checkpoint (Vi & Truong)
- [ ] **End-to-End `repo_explain` Execution:**
  - User runs: `custos run "Which module stores tasks and why can the UI not access the DB directly?"`
  - Request creates task -> Persists to SQLite -> Workspace Engine scans repository -> Context Compiler builds manifest -> FakeProvider returns cited answer -> Citation Verifier checks paths -> Artifact Store records result -> Task transitions to `Succeeded`.
- [ ] **Crash Verification:** Restart daemon and confirm task history, events, and evidence bundle are fully readable from SQLite.

---

## Active Blockers & Technical Notes

| Date | Owner | Type | Description | Resolution / Status |
|---|---|---|---|---|
| 2026-09-22 | Truong | Note | Selecting between `rusqlite` with `r2d2` vs `sqlx` for SQLite connection pooling. | In progress |
| 2026-09-22 | Vi | Note | Finalizing `TaskStatus` state transition enum values in `core-domain`. | In progress |

---

## Daily Sync Logs

- **Vi's Daily Reports:** See [dev_docs/vi/reports/](./vi/reports/)
- **Truong's Daily Reports:** See [dev_docs/truong/reports/](./truong/reports/)
