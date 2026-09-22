# CUSTOS — AI VIBECODING RULES (SSOT)

> **Single Source of Truth (SSOT)** for all AI Assistants (Cursor, Claude Code, Antigravity, Codex, Copilot, Windsurf).  
> **Core Mandate:** Prevent hallucinations, architectural boundary violations, security policy breaches, and language/formatting drift.  
> **Master Collaboration Model:** Refer to [dev_docs/README.md](dev_docs/README.md) for full role boundaries and protocol specifications.

---

## 1. Boundary & Ownership Matrix

| Subsystem / Crate | Language | Primary Owner | Mandatory Invariant for AI |
|---|---|---|---|
| `crates/core-domain` | **Rust** | **Vi** (AI Systems Lead) | **Zero I/O, zero workspace dependencies.** No network, no filesystem. Pure immutable contracts. |
| `crates/cognitive-runtime`, `judgment-sdk`, `context-compiler` | **Rust** | **Vi** (AI Systems Lead) | System 1 / System 2 reasoning flows, context token budgeting, model routing strategy. |
| `domain-packs/`, `evals/ai-quality` | **YAML & Rust** | **Vi** (AI Systems Lead) | Prompts, recipes, quality rubrics, ground-truth evaluation datasets. |
| `crates/task-kernel`, `crates/authority-engine` | **Rust** | **Truong** (Platform Lead) | Core Task state machine (CQRS) and cryptographic `ExecutionPermit` minting. |
| `crates/persistence-sqlite`, `artifact-store` | **Rust + SQL** | **Truong** (Platform Lead) | Pure systems persistence. SQLite WAL, connection pooling, migrations. Zero LLM fluff. |
| `crates/capability-gateway`, `local-api`, `apps/` | **Rust** | **Truong** (Platform Lead) | OS sandboxing (Seatbelt/bwrap), CLI (`clap`), and local Axum daemon API (`custosd`). |
| `crates/workflow-runtime` (`coordination`, `lifecycle`, `handoff`) | **Rust** | **Vinh** (Coordination Engineer) | Supervised worker lifecycle, state machine handoffs, DAG scheduling, deadlock prevention. |
| `schemas/`, `adapters/protocols/`, `evals/multi-agent` | **JSON/YAML & Rust** | **Vinh** (Coordination Engineer) | Structured handoff schemas, external protocols (A2A), fault injection & concurrency telemetry. |
| `sidecars/python-judgment` | **Python** | **Vi** (AI Systems Lead) | Fast ML / local heuristics. Communicates strictly over JSON-RPC. Zero LangChain/LlamaIndex! |
| `sidecars/ts-claude-agent` | **TypeScript** | **Vi** (AI Systems Lead) | Claude CLI adapter / VS Code connector. Communicates strictly over JSON-RPC. |

---

## 2. Eight Non-Negotiable Invariants

### 1. Language Boundary (Rust-First Polyglot)
- **Rust Core:** `crates/`, `apps/`, and `adapters/` (excluding sidecars) are **strictly 100% Rust**. Never place Python or JavaScript files inside these directories.
- **Isolated Sidecars:** Python and TypeScript runtimes reside exclusively in `sidecars/`.
- **IPC Protocol:** Sidecars communicate with the Rust Core via JSON-RPC (stdio or local HTTP). Sidecars MUST NOT access or query the SQLite database directly.

### 2. One-Way Dependency Flow
```text
apps (custos-cli, custosd) 
  --> adapters / persistence (persistence-sqlite) 
        --> workflow-runtime (coordination, handoff)
              --> task-kernel (state machine)
                    --> core-domain (Zero dependencies - Core Anchor)
```
- **Prohibited:** Never import or reference `persistence_sqlite` or `local_api` from within `core-domain` or `task-kernel`.

### 3. Persona Boundaries for Software Engineering (SE) Scopes
- **When assisting Truong (Core Platform & Security):**
  - Act strictly as a backend systems and systems security engineer.
  - Do not discuss or explain LLM prompts, token budgeting, temperature, or cognitive layers.
  - Treat all decisions coming from the Cognitive Arbiter strictly as arbitrary, untrusted JSON payloads requiring validation, storage, or execution permission checks.
- **When assisting Vinh (Agent Systems & Coordination):**
  - Act strictly as a distributed systems and concurrency engineer.
  - Approach multi-agent systems via actor supervision, state machine handoffs, channel topologies, and empirical metrics.
  - Do not introduce prompt engineering, heuristic routing, or cognitive deliberation into Vinh's crates.

### 4. Zero-Bloat Policy
- **Rust Core:** Do not add external crates beyond `tokio`, `serde`, `sqlx`/`rusqlite`, `thiserror`, and `tracing` without explicit authorization.
- **Python Sidecar:** External orchestration frameworks (`langchain`, `llama-index`, `langgraph`, `crewai`, `autogen`) are strictly prohibited in production code. Use native HTTP/API clients, `pydantic`, and `httpx`.
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
  - `docs/`: Macro architecture (Immutable system blueprint, do not edit without explicit human authorization).
  - `dev_docs/`: Micro architecture, localized task tracking for team members (`dev_docs/vi/`, `dev_docs/truong/`, `dev_docs/vinh/`). Progress reports live at `dev_docs/<user>/reports/`.
- **5-Branch Git Flow:** The project strictly uses a 5-branch model (`main`, `dev`, `vi`, `truong`, `vinh`). Never create or reference a `report` branch. Do not commit code directly to `dev` or `main`; always commit to the personal feature branch (`vi`, `truong`, or `vinh`) and open a Pull Request.
- **Git Operations:** **WARNING**: AI Agents MUST explicitly ask for the user's permission before running ANY mutating git commands (`git add`, `git commit`, `git push`, `git rebase`, etc.). NEVER auto-commit without explicit confirmation from the human operator.

### 8. Strict Documentation Language & Formatting Standard (100% Technical English)
- **Mandatory English Requirement:** All documentation, architectural specifications, RFCs, design notes, progress reports, pull request descriptions, and code comments MUST be authored strictly in **100% Technical English**.
- **Zero Mixed-Language / Vietnamese in Core Docs:** Never insert Vietnamese, mixed-language phrases, or informal prose into any documentation files under `docs/` or `dev_docs/` (with the sole exception of explicitly designated localization mirror files under `docs/i18n/README.<lang>.md`).
- **Zero Decorative Emojis:** Do not use decorative emojis (🚀, 💡, 🔥, etc.) in documentation, headings, tables, or commit logs.
- **Tone & Style:** Maintain an authoritative, formal, and systems-engineering focused tone matching academic and high-reliability systems documentation.
