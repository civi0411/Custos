# Truong's Workspace (Software Engineer)

> **Role:** Backend & Systems Engineer  
> **Modules Owned:** `crates/persistence-sqlite`, `crates/capability-gateway`, `crates/local-api`, `apps/custos-cli`, `apps/custosd`.

---

## Focus Areas
1. **Persistence (SQLite):**
   - Design `tasks`, `spans`, and `events` tables with WAL mode and crash resilience.
   - Implement typed Rust CRUD operations (`rusqlite` or `sqlx`) consuming domain structs defined by Vi in `crates/core-domain`.
2. **Local API & CLI:**
   - Develop `custos-cli` binary using `clap` (parsing user commands).
   - Develop `custosd` daemon and local HTTP service using `axum`.
3. **Capability Gateway & Sandbox:**
   - Safe execution: Worktree management, read/write restrictions, and network egress controls via Seatbelt (macOS) and Bubblewrap (Linux).

---

## Directory Organization
- `notes/`: DB migration designs, I/O benchmarks, CLI syntax notes, sandbox configurations.
- `reports/`: Personal progress reports by day/sprint for peer review on the `report` branch.

---

## Active Checklist
- [ ] Initialize SQLite connection pool with WAL mode in `crates/persistence-sqlite`.
- [ ] Write migrations for `tasks` table.
- [ ] Implement `insert_task` and `get_task`.
