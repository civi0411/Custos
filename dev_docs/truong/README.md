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

## 2. Workspace Purpose & Directory Structure

This space (`dev_docs/truong/`) serves two distinct purposes:
1. **Domain Architecture (`notes/`)**: The localized technical specs, database schemas, and architectural notes specific to the Backend and Platform modules.
2. **Development Reports (`reports/`)**: The chronological record of daily sprint progress, features developed, and syncs.

- `notes/`: Technical architecture notes, database schema designs, I/O benchmarks, sandbox configurations.
- `reports/`: Daily sprint reports and sync logs committed directly to the feature branch.

---

## 3. Standard Daily Report Template (`reports/YYYY-MM-DD.md`)

When committing daily progress, write your report to `reports/YYYY-MM-DD.md` in this directory, commit directly to the `truong` branch alongside your code, and open a Pull Request to `dev`:

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

## 4. Core Infrastructure & Runtime Engine (Completed)

- **Database Persistence:** Deployed `crates/persistence-sqlite` featuring a Connection Pool and WAL mode, ensuring atomic transactions. SQL Migrations are finalized.
- **Local API & CLI:** Developed `apps/custos-cli` integrated with `clap`, supporting the complete task lifecycle commands (`run`, `status`, `advance`, `cancel`). The local API core via `axum` is successfully wired.
- **Durability & State Machine:** Successfully passed Crash Testing (`kill -9` recovery). All state transitions are durably event-sourced and survive abrupt process termination without data loss.
- **Capability Gateway:** Established OS-level sandboxing (Seatbelt/bwrap) and strict authorization gates via `ExecutionPermit`.

---

## 5. Upcoming Workload (UX & Integrations)

- Integrate `indicatif` to display real-time progress bars and spinners in the CLI.
- Build an intuitive, Git syntax-highlighted Approval Prompt for safe Exact-Payload diff reviews.
- Configure and wire the VS Code Extension (TypeScript) to communicate with the Custos Daemon via JSON-RPC.
- Bootstrap the Desktop Webview to render rich Human Attention Packets.
