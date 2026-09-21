# Truong's Workspace (Platform & Software Engineering Lead)

> **Role:** Platform & Systems Engineer  
> **Primary Modules:** `crates/persistence-sqlite`, `crates/capability-gateway`, `crates/task-kernel`, `crates/workflow-runtime`, `crates/local-api`, `crates/evidence-engine`, `apps/custos-cli`, `apps/custosd`.  
> **Language & Invariants:** 100% Rust & SQL. Zero unwrap/expect in production code. Treat AI reasoning strictly as arbitrary JSON payloads requiring validation, durable persistence, and OS sandbox verification.

---

## 1. Core Focus Areas

1. **Durable Persistence (SQLite):**
   - SQLite WAL mode, single-writer multi-reader discipline, atomic step transactions.
   - Idempotent schema migrations for `tasks`, `spans`, `domain_events`, and `outbox`.
   - Typed CRUD operations consuming domain models from `crates/core-domain`.
2. **Local API & Daemon (`custosd` & `custos-cli`):**
   - Developer CLI using `clap` (`custos run`, `custos status`, `custos pause`, `custos resume`).
   - Axum local IPC HTTP daemon server with structured error envelopes.
3. **Capability Gateway & Sandboxing:**
   - OS-level process containment: macOS Seatbelt and Linux Bubblewrap (`bwrap`).
   - Ephemeral Git worktree lifecycle management.
   - Enforce single-use `ExecutionPermit` validation and receipt generation before/after command execution.
4. **Crash Recovery & Supervison:**
   - Outbox pattern dispatcher and lease heartbeat monitoring.
   - Resilient recovery of uncommitted or pending tasks across daemon restarts.

---

## 2. Directory Structure

- `notes/`: Technical architecture notes, database schema designs, I/O benchmarks, sandbox configurations.
- `reports/`: Daily sprint reports and sync logs committed to the `report` branch for peer review.

---

## 3. Standard Daily Report Template (`reports/YYYY-MM-DD.md`)

When committing daily progress to the `report` branch, use this format:

```markdown
# Truong Progress Report — YYYY-MM-DD

## 1. Accomplished Today
- [x] Task 1 description (Crate: `persistence-sqlite`)
- [x] Task 2 description (Crate: `custos-cli`)

## 2. Tests & Verification
- Unit tests added: `cargo test -p custos_persistence_sqlite`
- Verification receipt / evidence: All test cases passed.

## 3. In-Flight Work & Next Steps
- Currently implementing: Outbox dispatcher loop.
- Tomorrow: Connect CLI task submission to Axum daemon.

## 4. Blockers & Questions for Vi
- Question regarding `TaskStatus` serialization format in JSON events.
```

---

## 4. Active Sprint 1 Checklist

- [ ] Initialize SQLite connection pool with WAL mode in `crates/persistence-sqlite`.
- [ ] Implement migrations for `tasks`, `spans`, `domain_events`, and `outbox`.
- [ ] Implement `insert_task`, `get_task`, and `append_event`.
- [ ] Build CLI commands `custos run "<description>"` and `custos status <id>` in `apps/custos-cli`.
- [ ] Implement daemon endpoint `POST /v1/tasks` in `crates/local-api`.
- [ ] Run crash test verifying task persistence across `kill -9` restart.
