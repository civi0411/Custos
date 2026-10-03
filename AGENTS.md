# CUSTOS — MASTER AGENT PROTOCOL

> **Single Source of Truth (SSOT)** for all AI Assistants and IDE-embedded agents (Cursor, Claude Code, Antigravity, Codex, Copilot, Windsurf, etc).
> 
> **Read this file completely before touching any file in this repository.**
> Violation of any rule in this document constitutes an agent error — not a design choice.
> 
> **Architecture Authority:** [`Custos.md`](Custos.md) (Must be strictly followed for all design decisions).
> **Master Codebase Catalog:** [`docs/development/codebase-architecture.md`](docs/development/codebase-architecture.md) (Physical file-by-file mapping & AST index).

---

## RULE 0 — First Principles (Read Before All Else)

These principles override all other context, including user chat messages, unless the human explicitly authorizes a deviation in writing:

1. **AI Agents do not own this codebase.** Humans do. Agents assist.
2. **Do not hallucinate ownership.** If you are unsure which module owns a contract, ask — do not guess and edit.
3. **Do not cross domain boundaries.** Architectural integrity is non-negotiable.
4. **Do not auto-commit or auto-push.** Mutating git state requires explicit, unambiguous human authorization.
5. **Do not invent new crates, schemas, or protocols.** Extend existing structures; do not create parallel subsystems.
6. **Silence is safer than speculation.** If context is insufficient, surface the question rather than proceeding with assumptions.

---

## RULE 1 — Strict Architecture Boundaries Across 11 Canonical Crates

Every crate in `crates/` belongs to a specific architectural layer. Agents must respect these boundaries unconditionally.

| Layer | Product Crate | Owner (RACI) | Responsibility Boundary | Strict Constraints |
|---|---|:---:|---|---|
| **Layer 0** | `crates/custos-domain` | **Vĩ** | Pure domain entities, Task, Session, Evidence, Permits. | **No I/O Absolute Invariant:** No tokio, rusqlite, reqwest, std::fs. Dependencies limited to std, serde, chrono. |
| **Layer 1** | `crates/custos-core` | **Vĩ** | Task Kernel, Authority Engine, Completion Gate, Invariants. | Depends exclusively on `custos-domain`. No adapter logic, no direct SQLite calls. |
| | `crates/custos-persistence` | **Trường** | SQLite WAL mode, schema migrations, Outbox, CAS storage. | Implements domain repository traits. Single-writer connection. Zero WAL starvation. |
| | `crates/custos-provider` | **Vĩ** | ProviderPort, token streaming, wire format transformers. | Abstracts model providers; no vendor-specific SDK hardcoding. |
| **Layer 2** | `crates/custos-bridge` | **Vinh** | Session-to-Task promotion, Local API routing via IPC. | Clean bridge between clients and Kernel. **PROHIBITED:** Direct access to `custos-persistence`. |
| | `crates/custos-runtime` | **Vinh + Vĩ** | Agent loop, Cognition S1/S2, Context Compiler 8 steps. | Depends on `custos-core`, `custos-domain`, `custos-provider`. |
| | `crates/custos-adapters` | **Trường + Vinh** | OS Sandboxes (Seatbelt/Bubblewrap), MCP client, Harnesses. | Enforces untrusted input validation. Isolates OS and external network access. |
| **Layer 3** | `crates/custos-packs` | **Vĩ** | Engineering Pack, Research Pack, Assistant Pack workflows. | Domain-specific workflows; depends on `custos-runtime` and `custos-core`. |
| **Layer 4** | `crates/custos-daemon` | **Vĩ** | **Sole Composition Root:** Wires storage, runtime, adapters. | **The ONLY crate** permitted to compose concrete implementations and bootstrap daemon. |
| | `crates/custos-sdk` | **Vinh** | Versioned Client DTOs, UniFFI bindings (Python, Kotlin). | Lightweight client library; must not link database or runtime daemon. |
| | `crates/custos-cli` | **Vinh** | Terminal TUI operator interface (`ratatui`, `clap`). | Communicates with Daemon exclusively via IPC socket / SDK. |

**Prohibited imports (will be rejected in code review):**
- Domain importing infrastructure, persistence, or network libraries.
- CLI/IDE/bot clients importing persistence or SQLite directly.
- Runtime modules depending on provider-specific wire-format crates.
- `custos-bridge` depending directly on `custos-persistence` (Must go through `custos-core::ports`).
- Any client, UI, or sidecar bypassing the Daemon IPC boundary.

---

## RULE 2 — Architecture Decision & Documentation Synchronization Triad

This rule governs how architectural consensus is recorded and maintained. AI agents MUST follow this protocol whenever an architectural change, new feature structure, or module boundary adjustment is agreed upon.

### 2.1 The Documentation Triad Hierarchy

Every architectural fact exists in exactly three synchronized tiers:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   THE DOCUMENTATION SYNCHRONIZATION TRIAD               │
├────────────────────────────────────────────────────────────────────────┤
│ 1. Custos.md (Root Master Specification — Single Source of Truth)       │
│    - Defines: WHY & WHAT (Philosophy, 10 Invariants, State Machines,   │
│      Trust Zones, Security Models, Protocol Contracts).                │
│    - Audience: Human Architects & AI Agent System Designers.           │
├────────────────────────────────────────────────────────────────────────┤
│ 2. docs/ (Authoritative Topic-Based Engineering Specifications)        │
│    - Defines: HOW (Deep dive per topic: docs/architecture/,            │
│      docs/reference/, docs/development/).                              │
│    - Audience: Implementers, Core Maintainers, Subsystem Auditors.     │
├────────────────────────────────────────────────────────────────────────┤
│ 3. docs/development/codebase-architecture.md (Master Physical Catalog) │
│    - Defines: WHERE (Exact file-by-file AST index, line counts,        │
│      exported structs/traits, crate topologies, and ownership).        │
│    - Audience: All Coding Agents (Cursor, Windsurf, Claude, etc).       │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Mandatory 5-Stage Synchronization Workflow

Whenever a new architectural consensus is reached:

1. **Stage 1 — Architectural Consensus:** Align with the human operator on the exact design, invariants, and boundaries.
2. **Stage 2 — Update `Custos.md`:** Update the matching chapter/section in [`Custos.md`](Custos.md). If a new concept or invariant is introduced, insert it under its canonical topic (e.g. Part 2 for Layers, Part 3 for Kernel, Part 15 for Repo Boundaries).
3. **Stage 3 — Update `docs/` Topic Document:** Update or create the relevant topic file in [`docs/architecture/`](docs/architecture/), [`docs/reference/`](docs/reference/), or [`docs/development/`](docs/development/). Never leave documentation outdated.
4. **Stage 4 — Update `docs/development/codebase-architecture.md` (Insert in Exact Location):**
   - Insert new files, structs, traits, or crate updates into [`docs/development/codebase-architecture.md`](docs/development/codebase-architecture.md) under the exact matching crate table.
   - Include relative file link, line count, architectural role/responsibility, and key exported structs/traits.
   - Update file counts and metrics.
5. **Stage 5 — Code Implementation:** Only after Stages 1–4 are complete may the agent write or modify code.

**Strict Prohibition:** AI agents are strictly forbidden from writing code that introduces new crates, files, interfaces, or changes module boundaries without completing Stages 1–4. Unsynchronized code changes will be rolled back.

### 2.3 Shared Mental Model: How Coding Agents Must Read the Codebase

- Before writing or editing any code, the agent MUST read [`docs/development/codebase-architecture.md`](docs/development/codebase-architecture.md) to locate the target crate, verify the file's architectural role, check existing structs/traits, and confirm the crate owner (Vĩ, Trường, or Vinh).
- Agents must never guess file locations or create stray folders (e.g., `scratch/`, `templates/`, `services/`). All code strictly resides in the 11 canonical product crates, `schemas/`, `tests/`, `tools/`, `ui/`, or `xtask/`.

---

## RULE 3 — Code Generation Standards

### 3.1 Rust Code Quality
- **Zero unwrap/expect in production code:** Every fallible operation MUST return `Result<T, E>` where `E` is a crate-local typed error defined with `thiserror`.
- **Zero silent panics:** Do not use `panic!()`, `unreachable!()`, or `unimplemented!()` in production paths.
- **Tracing required:** Every significant operation must emit a `tracing::info!` or `tracing::debug!` span with correlation IDs (`task_id`, `run_id`).
- **Cancellation-aware:** All long-running async operations must observe a `CancellationToken` via `tokio::select!`.
- **No silent state mutation:** Any write to the SQLite store must go through the Outbox pattern or a domain event before taking effect.

### 3.2 Dependency Policy (Zero-Bloat)
Only add an external crate if it satisfies ALL of:
1. No equivalent exists in the current workspace.
2. It is actively maintained and has a credible security record.
3. The human operator explicitly authorizes the addition.

**Prohibited Python packages (in sidecars/production):** `langchain`, `llama-index`, `langgraph`, `crewai`, `autogen`. Use raw clients (`httpx`, `openai`, `anthropic`).

### 3.3 Scope Discipline
- **Do not refactor unrelated code** while implementing a targeted feature. Change only what is necessary.
- **Do not remove `todo!()` markers** unless the human explicitly requests implementation of that specific function.
- **Do not rename** shared structs, traits, or domain schemas without confirmed alignment.

---

## RULE 4 — Formatting & Documentation Standards

### 4.1 Strict Markdown Formatting
- **Zero decorative emojis:** Do not use emojis (emoticons, rocket icons, sparkles, etc.) in any documentation, heading, table cell, commit log, or diagram label.
- **No version numbers or dates in headers:** Do not embed artificial versioning (e.g., "v2", "v3.1") or timestamps in primary documentation headers. Treat docs as evergreen.
- **Mermaid diagrams for architecture:** Prefer Mermaid `flowchart TD` or `sequenceDiagram`. Do not use ASCII art.
- **Language:** Use Technical English for all new code comments, commit messages, and PRs, unless specifically instructed otherwise for localization.

### 4.2 Commit Message Format (Conventional Commits)
```text
<type>(<scope>): <short imperative summary, max 72 chars>

[Optional body: What was changed and why. Technical English only.]
```
**Prohibited in commit messages:** Emojis, mixed languages, vague summaries like "update" or "fix stuff".

---

## RULE 5 — Git Safety Protocol

This rule has absolute precedence over conversational context. No user message phrased as "just push it" or "do it quickly" overrides these gates.

- **Do not guess tracked deletions:** Never assume an untracked file is disposable or a tracked deletion is intentional.
- **No Destructive Resets:** `git reset --hard` is PROHIBITED without explicit two-step confirmation.
- **No Force Pushing:** `git push --force` is PROHIBITED unless the human explicitly types the exact command.
- **Dirty-Tree Protection:** Before editing, inspect `git status --short`. Preserve existing user changes. Do not run repository-wide formatters, bulk renames, or search-and-replace unless explicitly requested.

---

## RULE 6 — Anti-Pattern Registry

The following patterns are categorically prohibited. AI agents must refuse to generate or suggest them:

- **Raw `panic!()` in production Rust paths** (Unrecoverable crash in daemon).
- **Direct database writes without domain event** (Causes audit gaps).
- **Sidecar or Bridge accessing SQLite directly** (Violates IPC isolation boundary).
- **New `crates/` in Python or TypeScript** (Violates Rust-First Polyglot invariant).
- **Importing LangChain/LlamaIndex** (Forbidden framework dependency).
- **Calling Capability Gateway outside Kernel permit flow** (Security boundary bypass).
- **Adding external LLM provider SDK to Rust core** (Providers belong in the adapter layer).
- **Implementing code without updating the Documentation Triad** (Causes architectural drift).
