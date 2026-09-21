# Custos Dev Docs — Collaboration & Responsibilities (Vi & Truong)

> This directory serves as the internal coordination hub between **Vi** (AI Engineer) and **Truong** (Software Engineer).  
> The canonical product architecture and specifications are maintained separately in [docs/](../docs/00-start-here.md).

---

## 1. Architectural Responsibilities & Boundaries

Custos is architected as a **Rust-First Polyglot**: Rust forms the immutable trusted computing base, while auxiliary runtimes (Python/TypeScript) execute strictly in isolated sidecars via JSON-RPC.

```mermaid
flowchart TD
    subgraph "Truong's Domain (Software Engineer - 100% Pure Rust & SQL)"
        DB[(SQLite Persistence)] <--> Kernel[Task Kernel & State Machine]
        Kernel <--> CLI[Custos CLI]
        Kernel <--> API[Axum Local API]
        Kernel <--> Sandbox[Capability Gateway & Sandbox]
    end

    subgraph "Vi's Domain (AI Engineer - Core Domain & Sidecars)"
        Arbiter[Cognitive Arbiter (Rust)]
        Arbiter <--> TS[Claude Agent Sidecar (TypeScript)]
        Arbiter <--> PY[Local ML / Heuristics (Python)]
        Arbiter <--> LLM[External Providers API]
    end
    
    Kernel ===|JSON-RPC / Core Domain Types| Arbiter
```

### Vi — AI Engineer (Core Architect)
- **Primary Modules:** `crates/core-domain`, `crates/cognitive-runtime`, `sidecars/python-judgment`, `sidecars/ts-claude-agent`.
- **Languages:** Rust (interfaces & contracts), Python & TypeScript (sidecars).
- **Core Responsibilities:**
  - Define Core Domain types and traits in `crates/core-domain` (Zero I/O, Single Source of Truth).
  - Design Cognitive Arbiter workflows (System 1: Fast Heuristics / System 2: Deep LLM Reasoning).
  - Implement and maintain isolated sidecars (Python for Local ML/scoring, TypeScript for Claude API/CLI adapters).
  - Prompt engineering, Context Compiler design, token budgeting.
- **Personal Workspace & Reports:** [dev_docs/vi/](./vi/README.md)

### Truong — Software Engineer (SE)
- **Primary Modules:** `crates/persistence-sqlite`, `crates/capability-gateway`, `crates/local-api`, `apps/custos-cli`, `apps/custosd`.
- **Languages:** 100% Rust & SQL.
- **Engineering Mindset:**
  - **Treat AI as a Black Box:** No requirement to learn or track LLM prompts, token dynamics, temperature, or cognitive layers.
  - **Systems & Data First:** Treat all decisions from the AI runtime strictly as arbitrary JSON payloads requiring validation, durable SQLite persistence, and OS sandbox verification before command execution.
- **Core Responsibilities:**
  - SQLite schema design, migrations, and resilient CRUD operations via `rusqlite` / `sqlx`.
  - CLI binary development with `clap` and local HTTP daemon endpoints with `axum`.
  - OS-level safety gates: Worktree isolation, Seatbelt (macOS) / Bubblewrap (Linux).
  - Observability infrastructure: Structured logging, OpenTelemetry tracing.
- **Personal Workspace & Reports:** [dev_docs/truong/](./truong/README.md)

---

## 2. Git Branching Strategy

To prevent merge conflicts, keep commit histories clean, and facilitate asynchronous reviews, the repository uses a dedicated 5-branch strategy:

| Branch | Purpose | Primary Operator | Invariants |
|---|---|---|---|
| `main` | **Production Release** | Both | Cleanest branch. Contains only thoroughly tested, production-ready code and finalized documentation. |
| `dev` | **Integration & System Test** | Both | Integration branch where `vi` and `truong` merge feature code for full workspace testing (`cargo test --workspace`). |
| `vi` | **Vi's Feature Branch** | Vi | AI domain traits, prompts, cognitive runtime, and sidecar implementations. |
| `truong` | **Truong's Feature Branch** | Truong | Backend systems, SQLite storage, CLI commands, and capability gateway sandboxing. |
| `report` | **Documentation & Status Sync** | Both | Dedicated branch to commit sprint reports, design notes, and reviews in `dev_docs/` **without polluting the `dev` code commit history**. |

### Daily Workflow:
1. **Feature Implementation:** Vi works on `vi`; Truong works on `truong`.
2. **Status & Documentation Sync:** Switch to `report`, write updates into `dev_docs/vi/` or `dev_docs/truong/`, commit, and push for peer review.
3. **Integration Merge:** Once a milestone is ready, open a PR / merge from `vi` or `truong` into `dev`. Run full regression tests on `dev`.
4. **Production Release:** Once `dev` passes all verification suites and proves stable, merge `dev` into `main`.

---

## 3. Code Hand-Off Protocol

1. **Step 1:** Vi defines Rust `struct` or `trait` models in `crates/core-domain` (e.g., `struct Task { pub id: TaskId, pub status: TaskStatus }`).
2. **Step 2:** Truong references these shared types to implement `INSERT INTO tasks...` persistence in `crates/persistence-sqlite` or exposes them via `crates/local-api`.
3. **Step 3:** Neither engineer renames shared structs, traits, or contract files without prior notification.

---

## 4. Current Sprint Tracking
Track active deliverables and checklists in [SPRINT_STATUS.md](./SPRINT_STATUS.md).
