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

### 2. Topic-Driven Monorepo Architecture & Inward Boundaries

1. **`platform/contracts` (`custos-contracts`):**
   - Ring 0: Pure domain entities and coordination contracts.
   - Absolute Zero-I/O invariant (INV-01).
   - Prohibited dependencies: `tokio`, `rusqlite`, `reqwest`, `std::fs`, or external model vendor SDKs. Pure mathematical state representations, domain entities, event definitions, and abstract coordination ports.
2. **`platform/storage` (`custos-storage`):**
   - Ring 1: SQLite WAL storage and dynamic partition engine.
   - Transactional Outbox pattern, state persistence, and content-addressed storage (CAS).
   - Must operate via a single-writer connection pool with dedicated multi-reader pool to eliminate write contention and WAL starvation.
3. **`platform/integrations` (`custos-integrations`):**
   - Ring 1: External model providers, credentials, and capability bridges.
   - ModelPort router, provider codecs (OpenAI, Anthropic, Gemini, Groq, DeepSeek, Ollama), local llama.cpp inference, pricing catalogs.
   - External adapters: MCP federation, A2A card, PTY terminal, OS sandboxing (Seatbelt/Bubblewrap), Git worktrees, remote GPU compute relays.
4. **`platform/engine` (`custos-engine`):**
   - Ring 2: Governed execution, cognitive orchestration, and verification.
   - Task execution engine, S1 heuristics, S2 replanner, OI Pareto engine, FastPathFilter, T0-T6 topologies, Completion Gate, and workbench pack registration.
5. **`workbenches/*` (`custos-workbench-*`):**
   - Specialized vertical domain packs (`assistant`, `coding`, `quant`, `research`). Symmetrical 5-part layout (`manifest/`, `migrations/`, `recipes/`, `schemas/`, `src/`).
6. **`apps/*`:**
   - `apps/daemon` (`custos-daemon`): Sole server composition root.
   - `apps/cli` (`custos-app-cli`): Pure-Rust headless client over `custos-sdk`.
   - `apps/desktop/native`: Thin Tauri 2 native desktop host.
7. **`sdk/*`:**
   - `sdk/protocol` (`custos-protocol`): Wire protocol, envelopes, and DTO schemas.
   - `sdk/client-rust` (`custos-sdk`): Strongly typed Rust client SDK over Local API transport.

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

> **Objective:** Ensure consistent team collaboration attribution and clean atomic PR decomposition according to industry standards.

1. **Standard Co-authored-by Attribution:**
   - When multiple developers or agents collaborate on a commit, append standard git trailers following the Git trailer specification:
     `Co-authored-by: Contributor Name <contributor@example.com>`
   - Exactly one blank line must separate the commit message body from trailer blocks.
   - Never embed private credentials or personal email addresses directly in public repository documentation.
2. **Contribution Attribution:**
   - Maintain clear and balanced author attribution across collaborative workstreams.
3. **Modular Thematic PR Decomposition:**
   - Decompose features into single-responsibility, atomic PRs:
     - Batch 1: Contracts and domain schema specifications (`platform/contracts`, `sdk/protocol`).
     - Batch 2: Storage and external integration adapters (`platform/storage`, `platform/integrations`).
     - Batch 3: Engine coordination, runtime loops, and workbenches (`platform/engine`, `workbenches/*`).
     - Batch 4: Applications, gateways, and documentation updates (`apps/*`, `docs/*`).
   - Validate with `cargo check --workspace` before pushing.
