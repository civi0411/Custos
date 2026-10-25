# Source Code Engineering Standards & Safety Protocol
## Coding Standards for AI Coding Assistants

> **Scope:** All AI Coding Assistants (Cursor, Claude Code, Antigravity, Codex, Copilot, Windsurf) contributing to the Custos repository.

---

### 1. Rust Engineering Standards

1. **Zero `.unwrap()` or `.expect()` in Production Code:**
   - Any fallible operation must return `Result<T, E>`.
   - `E` must be a strongly typed error enum defined at the crate level via `thiserror`.
2. **Zero Silent Panics:**
   - Prohibit `panic!()`, `unreachable!()`, or `unimplemented!()` in production execution paths.
   - Use `todo!()` only when explicitly instructed by human operators to mark deliberate stubs.
3. **Mandatory Structured Tracing:**
   - Emit spans or logs for significant operations using `tracing::info!` or `tracing::debug!`.
   - Always attach contextual identifiers (`task_id`, `run_id`, `session_id`).
4. **Cancellation Awareness:**
   - Long-running async routines must observe a `CancellationToken` via `tokio::select!`.
5. **Transactional State Mutations:**
   - State mutations persisting to SQLite must utilize the Transactional Outbox pattern or emit domain events. Never perform un-audited direct mutations.

---

### 2. Inviolable 11 Canonical Crate Boundaries

1. **`crates/custos-domain`:**
   - Absolute Zero-I/O invariant.
   - Prohibited dependencies: `tokio`, `rusqlite`, `reqwest`, `std::fs`, or external model vendor SDKs.
2. **`crates/custos-core`:**
   - Coordination kernel and state engine.
   - Depends exclusively on `custos-domain`. External SDKs or persistence drivers are strictly forbidden.
3. **`crates/custos-persistence`:**
   - SQLite WAL storage implementation.
   - Must operate via a single-writer connection pool to eliminate write contention and WAL starvation.
4. **`crates/custos-bridge`:**
   - Session-to-Task promotion and client IPC routing.
   - STRICTLY PROHIBITED from importing `custos-persistence` directly. Storage queries must route through `custos-core::ports::KernelPort`.
5. **`crates/custos-daemon`:**
   - Sole Composition Root.
   - The only crate permitted to bind concrete adapters, databases, and network transports to launch the daemon.

---

### 3. Formatting & Documentation Standards (Zero-Emoji Policy)

1. **Zero Decorative Emojis:**
   - Do not use decorative emojis in markdown documentation, tables, code comments, or commit messages.
2. **Evergreen Documentation:**
   - Do not embed ephemeral dates or temporary version tags (e.g. "v2", "v3.1") in document titles.
3. **Conventional Commits:**
   - Standard format: `<type>(<scope>): <concise technical English summary, max 72 chars>`.

---

### 4. Git Safety Protocol

1. Never run `git reset --hard` without explicit, 2-step written confirmation from a human.
2. Never execute `git push --force`.
3. Always inspect `git status --short` before editing to preserve active working changes.
4. Never auto-commit or auto-push without unambiguous human instruction.

---

### 5. Commit Standards, PR Decomposition & Attribution

> **Objective:** Ensure team members (Vĩ, Vinh, Nhật Trường) receive proper attribution, streak credits, and GitHub Achievements (Pair Extraordinaire, Pull Shark).

1. **Mandatory Co-authored-by Trailers:**
   - Every commit must include trailing `Co-authored-by` headers for collaborators:
     - `Co-authored-by: Vinh <phuvi301@gmail.com>`
     - `Co-authored-by: NhatTruong1905 <tn696199@gmail.com>`
     *(or `Co-authored-by: civi0411 <cuuvi985@gmail.com>` when authored by Vinh or Trường)*.
   - Exactly one blank line must separate the commit message body from the trailers.
2. **Hybrid Rotation Strategy:**
   - Rotate `--author` when generating PR batches to maintain balanced contribution graphs across the team.
3. **Modular Thematic PR Decomposition:**
   - Decompose features into single-responsibility, atomic PRs:
     - Batch 1: Domain schemas and provider contracts.
     - Batch 2: Adapters and wire protocols.
     - Batch 3: Runtime coordination and cognitive loop.
     - Batch 4: Gateways, MCP integrations, and documentation updates.
   - Validate with `cargo check --workspace` before pushing.
