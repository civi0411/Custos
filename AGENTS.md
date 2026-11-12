# CUSTOS — MASTER AGENT PROTOCOL

> **Single Source of Truth (SSOT)** for all AI Assistants and IDE-embedded agents (Cursor, Claude Code, Antigravity, Codex, Copilot, Windsurf, etc).
> 
> **Read this file completely before touching any file in this repository.**
> Violation of any rule in this document constitutes an agent error — not a design choice.
> 
> **Architecture Authority:** [`Custos.md`](Custos.md) (Must be strictly followed for all design decisions).
> **Master Codebase Catalog:** [`docs/development/codebase-architecture.md`](docs/development/codebase-architecture.md) (Physical file-by-file mapping & AST index).
> **Desktop UI Protocol:** [`.agents/rules/desktop_ui_architecture_standards.md`](.agents/rules/desktop_ui_architecture_standards.md) (Strict React 19, Zero-Mock, 3-Tier Hierarchy, Quality Gates).

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

## RULE 1 — Architecture Boundaries: Platform Control Plane + Vertical Workbenches + Reusable Capabilities

The system is evolving according to the modular architecture refactor plan ([`docs/development/modular-workbench-refactor-plan.md`](docs/development/modular-workbench-refactor-plan.md)). There is no fixed 11-crate ceiling during this transformation. Worktree scoping applies where physically required (Coding), without imposing git worktrees on Assistant or Research corpora. All agents must respect the following inviolable invariants:

1. **Zero-I/O Domain Invariant:** Domain code has 0 I/O dependencies (no tokio, no rusqlite, no reqwest, no std::fs).
2. **Single-Writer Persistence Invariant:** SQLite WAL persistence remains single-writer; workers never open raw DB connections directly.
3. **Strict Authority & Completion Gates:** Actions require valid ExecutionPermits; completion gates reject unverified or foreign-task evidence (F01).
4. **Zero-Mock & Data Truthfulness:** Market datasets and scientific claims must reflect genuine sources or explicitly labeled simulation/demo environments, never fabricated or spoofed (F02).
5. **Boundary Isolation:** Platform owns the control plane (Task/Session/Run, authority, scheduling, canonical storage). Vertical Workbenches (Assistant, Coding, Research, Quant) own specialized domain semantics, schemas, and UI views. Capabilities (workspace, terminal, notebook, compute, inference) serve multiple workbenches on-demand.

Driver source descriptors currently live beside compiled modules in `platform/adapters/src/`; do not recreate uncompiled copies under a root capability scaffold. Tests and development automation live in `tests/desktop/` and `xtask/scripts/`.

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

- Before writing or editing any code, the agent MUST read [`docs/development/codebase-architecture.md`](docs/development/codebase-architecture.md) to locate the target crate or package, verify the file's architectural role, check existing structs/traits, and confirm the owner.
- Agents must never guess file locations or create stray folders (e.g., `scratch/`, `templates/`, `services/`). All code strictly resides in the canonical modular structure: `platform/`, `apps/`, `sdk/`, `workbenches/`, `schemas/`, `tests/`, `ui/`, or `xtask/`.

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
