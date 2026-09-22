# Truong's Workspace — Core Platform & Security Lead

> **Role:** Core Platform & Security Lead  
> **Core Direction:** Builds the trusted runtime, durable Task Kernel, persistence, capability security, execution isolation, APIs, recovery, and release infrastructure.  
> **Language & Invariants:** 100% Rust & SQL. Zero unwrap/expect in production code. Systems & Data First: Treat all decisions from the AI runtime strictly as untrusted JSON payloads requiring validation, durable SQLite persistence, and OS sandbox verification before command execution.

---

## 1. Core Focus Areas

1. **Durable Persistence (SQLite WAL):**
   - SQLite WAL mode, single-writer multi-reader discipline, atomic step transactions.
   - Idempotent schema migrations for `tasks`, `spans`, `domain_events`, and `outbox`.
   - Typed CRUD operations consuming domain models from `crates/core-domain`.
2. **Local API & Daemon Runtime (`custosd` & `custos-cli`):**
   - Developer CLI using `clap` (`custos run`, `custos status`, `custos pause`, `custos resume`).
   - Axum local IPC HTTP daemon server with structured error envelopes.
3. **Capability Gateway & OS Sandboxing:**
   - OS-level process containment: macOS Seatbelt and Linux Bubblewrap (`bwrap`).
   - Ephemeral Git worktree lifecycle management.
   - Enforce single-use `ExecutionPermit` validation and receipt generation before/after command execution.
4. **Authority Engine & Security Policy:**
   - Evaluates risk tiers (Low/Med allow, High human approval, Critical deny).
   - Enforces exact-payload cryptographic hashes before granting execution permits.
5. **Crash Recovery & Supervised Process Execution:**
   - Outbox pattern dispatcher and lease heartbeat monitoring.
   - Resilient recovery of uncommitted or pending tasks across daemon restarts.

---

## 2. Code Ownership & Repository Layout

```text
crates/
├── persistence-sqlite/         # SQLite WAL connection pool, migrations, and repositories
├── capability-gateway/         # OS-level sandboxing (Seatbelt/bwrap) and permit gates
├── authority-engine/           # Security policy evaluation and approval workflows
├── task-kernel/                # Core Task state machine, CQRS engine, and event store
├── local-api/                  # Axum IPC daemon HTTP endpoints and JSON-RPC
├── evidence-engine/            # Objective citation, hash, and diff verifiers
└── artifact-store/             # Content-addressed filesystem artifact storage

apps/
├── custos-cli/                 # Command-line interface binary (`clap`)
└── custosd/                    # Background daemon supervisor binary

tests/
├── e2e/                        # End-to-end integration and crash recovery test suites
└── contract/                   # Schema compatibility and persistence contract tests
```

---

## 3. Workspace Purpose & Directory Structure

This space (`dev_docs/truong/`) serves two distinct purposes:
1. **Domain Architecture (`notes/`)**: Localized technical specifications, database schema designs, I/O benchmarks, and sandbox configurations.
2. **Development Reports (`reports/`)**: Chronological record of daily sprint progress and sync logs committed directly to branch `truong`.

---

## 4. Standard Daily Report Template (`reports/YYYY-MM-DD.md`)

When committing daily progress, write your report to `reports/YYYY-MM-DD.md` in this directory, commit directly to the `truong` branch alongside your code, and open a Pull Request to `dev`:

```markdown
# Truong Progress Report — YYYY-MM-DD

## 1. Accomplished Today
- [x] Task description (Crate: `persistence-sqlite`)
- [x] Task description (Crate: `custos-cli`)

## 2. Tests & Verification
- Unit tests added: `cargo test -p custos_persistence_sqlite`
- Verification receipt / evidence: All test cases passed.

## 3. In-Flight Work & Next Steps
- Currently implementing: Outbox dispatcher loop.
- Tomorrow: Connect CLI task submission to Axum daemon.

## 4. Blockers & Questions for Vi / Vinh
- Question for Vi regarding `TaskStatus` serialization format in JSON events.
- Question for Vinh regarding worker coordination event projection schema.
```
