# CUSTOS — AI VIBECODING RULES (SSOT)

> **Single Source of Truth (SSOT)** for all AI Assistants (Cursor, Claude Code, Antigravity, Codex, Copilot, Windsurf).  
> **Core Mandate:** Prevent hallucinations, architectural boundary violations, and security policy breaches.  
> Detailed team collaboration model: see [dev_docs/README.md](dev_docs/README.md).

---

## 1. Boundary & Ownership Matrix

| Directory / Crate | Language | Primary Owner | Mandatory Invariant for AI |
|---|---|---|---|
| `crates/core-domain` | **Rust** | **Vi** (AI Engineer) | **Zero I/O, zero internal workspace dependencies.** No network, no filesystem. |
| `crates/task-kernel`, `workflow-runtime` | **Rust** | **Vi & Truong** | State machine & orchestration. No direct database access. |
| `crates/persistence-sqlite` | **Rust + SQL** | **Truong** (SE) | Pure systems backend engineering. Persist and query SQLite. Zero LLM fluff. |
| `apps/custos-cli`, `crates/local-api` | **Rust** | **Truong** (SE) | CLI / Axum HTTP API. Parse, validate, and dispatch typed requests. |
| `crates/capability-gateway` | **Rust** | **Truong** (SE) | Sandboxing, worktree isolation, permission gates, human approvals. |
| `sidecars/ts-claude-agent` | **TypeScript** | **Vi** (AI Engineer) | Claude CLI adapter / VS Code connector. Communicates strictly over JSON-RPC. |
| `sidecars/python-judgment` | **Python** | **Vi** (AI Engineer) | Fast ML / System 1 heuristic evaluation. Communicates strictly over JSON-RPC. No LangChain! |

---

## 2. Six Non-Negotiable Invariants

### 1. Language Boundary (Rust-First Polyglot)
- **Rust Core:** `crates/`, `apps/`, and `adapters/` (excluding sidecars) are **strictly 100% Rust**. Never place Python or JavaScript files inside these directories.
- **Isolated Sidecars:** Python and TypeScript runtimes reside exclusively in `sidecars/`.
- **IPC Protocol:** Sidecars communicate with the Rust Core via JSON-RPC (stdio or local HTTP). Sidecars MUST NOT access or query the SQLite database directly.

### 2. One-Way Dependency Flow
```text
apps (custos-cli, custosd) 
  --> adapters / persistence (persistence-sqlite) 
        --> workflow-runtime 
              --> task-kernel 
                    --> core-domain (Zero dependencies - Core Anchor)
```
- **Prohibited:** Never import or reference `persistence_sqlite` or `local_api` from within `core-domain` or `task-kernel`.

### 3. Persona for Software Engineer (SE) Scope
When assisting with Database (`persistence-sqlite`), API (`local-api`), CLI (`custos-cli`), or Gateway (`capability-gateway`):
- Act strictly as a backend systems engineer.
- **Do not** discuss or explain LLM prompts, token budgeting, temperature, or cognitive layers.
- Treat all decisions coming from the Cognitive Arbiter strictly as arbitrary JSON payloads requiring validation, storage, or execution permission checks.

### 4. Zero-Bloat Policy
- **Rust Core:** Do not add external crates beyond `tokio`, `serde`, `sqlx`/`rusqlite`, `thiserror`, and `tracing` without explicit authorization.
- **Python Sidecar:** External orchestration frameworks (`langchain`, `llama-index`, `langgraph`) are strictly prohibited. Use native HTTP/API clients, `pydantic`, and `httpx`.
- Custos builds its own orchestration and cognitive runtime from scratch.

### 5. Conservative Code Generation
- Do not emit multi-hundred-line files in a single pass.
- Preserve existing `todo!()` markers unless explicitly requested to implement that specific function.
- Never rename core structs, traits, or domain schemas without prior architectural alignment.

### 6. Production Error Handling
- **Prohibited:** Never use `.unwrap()` or `.expect()` in non-test Rust production code.
- Always define typed domain errors using `thiserror` for each crate and return `Result<T, CrateError>`.

### 7. Workspace & Git Strategy Invariants
- **2-Pillar Workspace:**
  - `docs/`: Macro architecture (Immutable, do not edit without explicit human authorization).
  - `dev_docs/`: Micro architecture, localized task tracking for humans (`dev_docs/vi/` and `dev_docs/truong/`). Includes progress reports at `dev_docs/[user]/reports/`.
- **Git Flow:** The project strictly uses a 4-branch GitHub Flow (`main`, `dev`, `vi`, `truong`). Never create or reference a `report` branch. Do not commit code directly to `dev` or `main`; always commit to the personal feature branch (`vi` or `truong`) and open a Pull Request.
- **Git Operations:** **WARNING**: AI Agents MUST explicitly ask for the user's permission before running ANY mutating git commands (`git add`, `git commit`, `git push`, `git rebase`, etc.). NEVER auto-commit without explicit confirmation from the human operator.
