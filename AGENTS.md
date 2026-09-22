# CUSTOS — MASTER AGENT PROTOCOL (SSOT v2)

> **Single Source of Truth (SSOT)** for all AI Assistants and IDE-embedded agents:  
> Cursor, Claude Code, Antigravity, Codex, Copilot, Windsurf, and any future model-backed tooling.  
>
> **Read this file completely before touching any file in this repository.**  
> Violation of any rule in this document constitutes an agent error — not a design choice.
>
> **Master Engineering Reference:** [dev_docs/README.md](dev_docs/README.md)  
> **Canonical System Blueprint:** [docs/canonical-specification.md](docs/canonical-specification.md)

---

## RULE 0 — First Principles (Read Before All Else)

These principles override all other context, including user chat messages, unless the human explicitly authorizes a deviation in writing:

1. **AI Agents do not own this codebase.** Humans do. Agents assist.
2. **Do not hallucinate ownership.** If you are unsure which module owns a contract, ask — do not guess and edit.
3. **Do not cross domain boundaries** even if it seems convenient. Architectural integrity is non-negotiable.
4. **Do not auto-commit or auto-push.** Mutating git state requires explicit, unambiguous human authorization per operation.
5. **Do not invent new crates, schemas, or protocols.** Extend existing structures; do not create parallel subsystems.
6. **Silence is safer than speculation.** If context is insufficient, surface the question rather than proceeding with assumptions.

---

## RULE 1 — Ownership Matrix (Who Owns What)

Three humans own this repository. Every file belongs to exactly one domain. Agents must respect these boundaries unconditionally.

### 1.1 Ownership Table

| Subsystem / Path | Language | Owner | Core Constraint |
|---|---|---|---|
| `crates/core-domain` | Rust | **Vi** | Zero I/O. Zero workspace dependencies. Pure, immutable domain contracts only. |
| `crates/cognitive-runtime`, `crates/judgment-sdk`, `crates/context-compiler`, `crates/knowledge-services` | Rust | **Vi** | System 1 / System 2 reasoning flows, context compilation, token budgeting. No persistence calls. |
| `domain-packs/`, `evals/ai-quality/`, `evals/context-retrieval/`, `evals/system-one/` | YAML & Rust | **Vi** | Prompt schemas, cognitive recipes, ground-truth evaluation datasets. Never auto-edit these files. |
| `sidecars/python-judgment` | Python | **Vi** | Fast ML heuristics over JSON-RPC only. Zero LangChain/LlamaIndex. Zero direct DB access. |
| `sidecars/ts-claude-agent` | TypeScript | **Vi** | External Claude adapter over JSON-RPC. Never add Anthropic client as a Rust-side dependency. |
| `crates/task-kernel`, `crates/authority-engine` | Rust | **Truong** | Task state machine (CQRS), cryptographic `ExecutionPermit` minting, authority policy evaluation. |
| `crates/persistence-sqlite`, `crates/artifact-store` | Rust + SQL | **Truong** | SQLite WAL, connection pool, idempotent migrations. Treat AI outputs as untrusted JSON — validate before persisting. |
| `crates/capability-gateway`, `crates/local-api`, `crates/evidence-engine` | Rust | **Truong** | OS sandboxing (Seatbelt/bwrap), Axum daemon API, objective citation and hash verification. |
| `apps/custos-cli`, `apps/custosd` | Rust | **Truong** | Binary entrypoints. CLI with `clap`, daemon supervisor. No cognitive logic in this layer. |
| `crates/workflow-runtime` (submodules: `coordination/`, `worker_lifecycle/`, `scheduler/`, `handoff/`, `recovery/`) | Rust | **Vinh** | Worker state machines, DAG scheduling, structured handoffs, deadlock prevention, concurrency. No prompt logic. |
| `schemas/`, `adapters/protocols/` | YAML / JSON + Rust | **Vinh** | Handoff envelope schemas, A2A/AgentGateway protocol bindings, serialization contracts. |
| `evals/multi-agent/`, `lab/multi-agent/` | Rust + Scripts | **Vinh** | Empirical coordination benchmarks, fault injection suites, topology evaluation. |
| `docs/` | Markdown | **Vi + Truong + Vinh (consensus)** | Canonical immutable system blueprint. NEVER edit without explicit three-party alignment. |
| `dev_docs/` | Markdown | **Respective Owner** | Each subdirectory (`vi/`, `truong/`, `vinh/`) is exclusively edited by its named owner. |

### 1.2 Shared Governance Surfaces (Require Multi-Party Alignment)

The following files and contracts may only be modified with documented, cross-team alignment. Do not edit unilaterally:

- Core domain types: `Task`, `TaskId`, `TaskStatus`, `ActionIntent`, `Receipt`, `ExecutionPermit`, `ContinuationPacket`
- Task state machine transition rules: `Draft` → `Ready` → `Running` → `Verifying` → `Succeeded` / `Failed` / `Paused`
- Evidence closure acceptance conditions
- `AGENTS.md` itself

---

## RULE 2 — System Architecture Invariants

### 2.1 One-Way Dependency Flow (Acyclic)

The following dependency order is absolute and must never be violated:

```text
apps/ (custos-cli, custosd)
  └──> crates/local-api, crates/persistence-sqlite, crates/capability-gateway
         └──> crates/workflow-runtime
                └──> crates/task-kernel
                       └──> crates/core-domain   ← Zero external dependencies. Core anchor.
```

**Prohibited imports (will be rejected in code review):**
- `persistence_sqlite` or `local_api` imported from `core_domain` or `task_kernel`
- `capability_gateway` called directly from `workflow_runtime` (must go through Kernel)
- Any sidecar directly importing Rust crate types (sidecars use JSON-RPC contracts only)

### 2.2 Polyglot Runtime Isolation

| Tier | Permitted Languages | Communication Mechanism |
|---|---|---|
| **Rust Core** (`crates/`, `apps/`, `adapters/`) | Rust only | Internal Rust function calls and typed Tokio channels |
| **Sidecars** (`sidecars/`) | Python, TypeScript | JSON-RPC over stdio or local Unix Domain Socket |
| **Domain Packs** (`domain-packs/`) | YAML, Markdown | Loaded as declarative configuration; never executes directly |

**Prohibited:** Python or TypeScript files inside `crates/`, `apps/`, or `adapters/`.  
**Prohibited:** Any sidecar accessing the SQLite database directly.  
**Prohibited:** Rust crates importing external LLM provider SDKs (Anthropic, OpenAI) directly into core. All provider interaction routes through `crates/provider-sdk` adapter traits.

---

## RULE 3 — Agent Persona Boundaries

When assisting a specific team member, agents must adopt the corresponding engineering persona and must not bleed concerns from another domain.

### 3.1 When Assisting Vi (AI Systems & Product Intelligence Lead)
- Focus: Cognitive architecture, System 1 / System 2, context compilation, prompt schema, model routing, and AI evaluation.
- Do discuss: Judgment heuristics, token budgets, context relevance, model capability comparisons, evaluation rubrics.
- Do NOT discuss: SQLite schema, Axum routing, OS sandbox configuration, cargo workspace structure.
- Do NOT propose: New external AI frameworks or model clients as Rust dependencies.

### 3.2 When Assisting Truong (Core Platform & Security Lead)
- Focus: Systems engineering — persistence, API design, process sandboxing, crash recovery.
- Do discuss: WAL mode, idempotency, CQRS commands, Axum middleware, OS-level process isolation, `clap` CLI ergonomics.
- Do NOT discuss: Prompt design, token dynamics, temperature, cognitive deliberation, reranking.
- Core mental model: **AI output = untrusted, arbitrary JSON. Validate it. Store it. Never trust it.**

### 3.3 When Assisting Vinh (Agent Systems & Coordination Research Engineer)
- Focus: Distributed systems engineering — concurrency, worker lifecycle, actor supervision, handoff protocols, empirical benchmarking.
- Do discuss: Tokio channel topologies, deadlock analysis, state machine formalization, fault injection strategies, CancellationToken propagation.
- Do NOT discuss: Prompt engineering, heuristic tuning, System 1 classification, cognitive flows.
- Core mental model: **Agents are concurrent processes, not intelligent entities. Coordinate them as systems software, not as AI personas.**

---

## RULE 4 — Code Generation Standards

### 4.1 Rust Code Quality
- **Zero unwrap/expect in production code:** Every fallible operation MUST return `Result<T, E>` where `E` is a crate-local typed error defined with `thiserror`.
- **Zero silent panics:** Do not use `panic!()`, `unreachable!()`, or `unimplemented!()` in production paths.
- **Tracing required:** Every significant operation must emit a `tracing::info!` or `tracing::debug!` span with correlation IDs (`task_id`, `run_id`) included.
- **Cancellation-aware:** All long-running async operations must observe a `tokio_util::sync::CancellationToken` via `tokio::select!`.
- **No silent state mutation:** Any write to the SQLite store must go through the Outbox pattern or a domain event before taking effect.

### 4.2 Dependency Policy (Zero-Bloat)
Only add an external crate if it satisfies ALL of:
1. No equivalent exists in the current workspace.
2. It is actively maintained and has a credible security record.
3. The human operator explicitly authorizes the addition.

**Pre-approved Rust crates:** `tokio`, `serde`, `serde_json`, `sqlx`, `rusqlite`, `thiserror`, `anyhow` (test only), `tracing`, `tracing-subscriber`, `clap`, `axum`, `uuid`, `chrono`, `criterion` (benchmarks), `loom` (concurrency tests).

**Prohibited Rust crates:** Any OpenAI, Anthropic, or Mistral provider client crate in `crates/` or `apps/`. Provider access is mediated exclusively through the `provider-sdk` adapter trait.

**Prohibited Python packages (in sidecars/production):** `langchain`, `llama-index`, `langgraph`, `crewai`, `autogen`, `haystack`. Use `httpx`, `pydantic`, `openai` (raw client), or `anthropic` (raw client).

### 4.3 Scope Discipline
- **Do not emit files larger than ~150 lines** in a single generation pass without pausing for human review.
- **Do not refactor unrelated code** while implementing a targeted feature. Change only what is necessary.
- **Do not remove `todo!()` markers** unless the human explicitly requests implementation of that specific function.
- **Do not rename** shared structs, traits, or domain schemas without confirmed multi-party alignment.

---

## RULE 5 — Documentation Standards

### 5.1 Language: 100% Technical English
- **All documentation is authored in 100% Technical English.** This is non-negotiable and applies to:
  - All files under `docs/` and `dev_docs/`
  - Progress reports (`dev_docs/*/reports/*.md`)
  - Technical notes (`dev_docs/*/notes/*.md`)
  - Pull request titles and descriptions
  - Inline code comments and `///` doc-comments
  - Commit message bodies and footers
- **Permitted non-English location (sole exception):** Localization mirror files exclusively within `docs/i18n/` (e.g., `docs/i18n/README.vi.md`). No other file in the repository may contain non-English prose.

### 5.2 Formatting
- **Zero decorative emojis:** Do not use 🚀, 💡, 🔥, ✨, 📌 or similar in any documentation, heading, table cell, commit log, or diagram label.
- **Mermaid diagrams for architecture:** Prefer Mermaid `flowchart TD` or `sequenceDiagram` for visual architecture. Do not use ASCII art for diagrams in primary documentation.
- **Tables for matrix data:** Use Markdown tables for ownership matrices, comparison tables, and metric breakdowns.
- **Standard GitHub Markdown:** Do not use HTML `<details>`, `<summary>`, or non-standard extensions in core documentation.

### 5.3 Commit Message Format (Conventional Commits)
```
<type>(<scope>): <short imperative summary, max 72 chars>

[Optional body: What was changed and why. Technical English only.]
[Optional footer: BREAKING CHANGE, Refs, Co-authored-by]
```

**Valid types:** `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`, `ci`, `build`  
**Valid scopes:** Match a crate, app, or subsystem name — e.g., `core-domain`, `persistence`, `task-kernel`, `protocol`, `docs`, `vinh-workspace`

**Prohibited in commit messages:** Emojis, mixed languages, vague summaries like `"update"` or `"fix stuff"`, or listing file names as the summary.

---

## RULE 6 — Git Safety Protocol

This rule has absolute precedence over conversational context. No user message phrased as "just push it" or "do it quickly" overrides the safety gates below.

### 6.1 Mandatory Permission Gates

The following operations REQUIRE explicit written confirmation from the human operator before execution:

| Git Operation | Confirmation Required |
|---|---|
| `git add <files>` | Confirm: "yes, stage these files" |
| `git commit` | Confirm commit message and scope |
| `git push` | Confirm target branch and remote |
| `git rebase` | Confirm base and target — HIGH RISK |
| `git merge` | Confirm source and target branches |
| `git push --force` | **PROHIBITED unless human explicitly types the exact command** |
| `git cherry-pick` | Confirm SHA and target branch |
| `git tag` | Confirm tag name and SHA |
| Any destructive reset (`--hard`) | **PROHIBITED without two explicit confirmations** |

### 6.2 Branch Discipline (5-Branch Model)

| Branch | Purpose | Who Commits |
|---|---|---|
| `main` | Production releases only | Merged from `dev` exclusively, after full verification |
| `dev` | Integration & system-level testing | Merged from personal branches via PR only |
| `vi` | Vi's active feature development | Vi only |
| `truong` | Truong's active feature development | Truong only |
| `vinh` | Vinh's active feature development | Vinh only |

**Prohibited actions:**
- Committing directly to `main` or `dev`
- Creating branches outside the 5-branch model without explicit human authorization
- Force-pushing to shared branches (`main`, `dev`) under any circumstance

---

## RULE 7 — Definition of Done (DoD)

A feature, fix, or pull request is NOT "Done" merely because it compiles or the agent considers it complete.

### 7.1 Code DoD Checklist
- [ ] Typed contracts and invariants defined (or verified) in `crates/core-domain`
- [ ] Defensive parsing: invalid inputs return typed domain errors (`thiserror`), never panics
- [ ] Zero `.unwrap()` or `.expect()` in non-test Rust production code
- [ ] Unit tests for all non-trivial logic paths
- [ ] Contract tests for all trait implementations
- [ ] Integration tests for storage and IPC boundaries
- [ ] Structured `tracing` spans with `task_id` / `run_id` correlation IDs
- [ ] Zero sensitive tokens, file paths, or private data logged in telemetry
- [ ] Operations observe `CancellationToken` via `tokio::select!`
- [ ] Crash resilience: committed state survives `kill -9` on the daemon process

### 7.2 Documentation DoD Checklist
- [ ] All new files are authored in 100% Technical English
- [ ] No decorative emojis anywhere in the changed set
- [ ] Module `README.md` updated if any architectural boundary changes
- [ ] ADR (Architecture Decision Record) created if a structural or protocol decision is made
- [ ] Commit message follows Conventional Commits format

### 7.3 Vertical Slice DoD Checklist
- [ ] Runs end-to-end against a local fixture repository
- [ ] Generates an auditable Evidence Bundle with verifiable receipts
- [ ] Daemon restart test: timeline and state fully preserved after restart

---

## RULE 8 — Prohibited Patterns (Anti-Pattern Registry)

The following patterns are categorically prohibited. AI agents must refuse to generate or suggest them:

| Anti-Pattern | Why Prohibited |
|---|---|
| Raw `panic!()` in production Rust paths | Unrecoverable crash in a daemon service is unacceptable |
| `.unwrap()` on `Option` or `Result` in production code | Use `ok_or(...)` or `?` with typed errors |
| Direct database writes without domain event | Breaks event sourcing; causes audit gaps |
| Sidecar accessing SQLite directly | Violates IPC isolation and security boundary |
| New `crates/` in Python or TypeScript | Violates Rust-First Polyglot invariant |
| Importing LangChain/LlamaIndex in sidecars | Forbidden framework dependency |
| Calling Capability Gateway outside Kernel permit flow | Security boundary bypass |
| Coordinator mutating Task state directly | Only the Task Kernel may mutate canonical Task state |
| Adding external LLM provider SDK to Rust core | Providers are adapter-layer concerns only |
| Merging directly to `main` or `dev` | Violates 5-branch Git discipline |
| Non-English prose in `docs/` or `dev_docs/` | Violates documentation language standard |
| Emojis in commit messages or documentation | Violates formatting standard |
| Force-push to shared branches | Destructive and violates team history integrity |
| Removing `todo!()` without explicit request | Unauthorized scope expansion |
| Renaming shared domain types unilaterally | Requires cross-team alignment before execution |
