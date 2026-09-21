# Custos Dev Docs — Engineering Collaboration & Architecture Protocol

> **Audience:** Vi (Product & AI/Research Lead) and Truong (Platform & Software Engineering Lead).  
> **Master Architecture:** Refer to [docs/canonical-specification.md](../docs/canonical-specification.md) for the authoritative 39-section system blueprint.  
> **Rule Enforcement:** All contributors and AI assistants must strictly obey [AGENTS.md](../AGENTS.md).

---

## 1. Role Division & Ownership Matrix

Custos is architected as a **Rust-First Polyglot**: Rust forms the immutable, trusted computing base (Kernel, Persistence, Capability Gateway, Daemon), while auxiliary runtimes (Python for fast ML heuristics, TypeScript for external agent adapters) reside exclusively in isolated sidecars communicating via JSON-RPC.

```mermaid
flowchart TD
    subgraph Truong["Truong's Domain (Software Engineer — 100% Rust & SQL)"]
        DB[(SQLite Persistence)] <--> Kernel[Task Kernel & State Machine]
        Kernel <--> CLI[Custos CLI]
        Kernel <--> API[Axum Local API / Daemon]
        Kernel <--> Sandbox[Capability Gateway & OS Sandboxes]
        Kernel <--> Verifier[Evidence & Verifier Runners]
    end

    subgraph Vi["Vi's Domain (AI Engineer — Core Domain & Sidecars)"]
        Arbiter[Cognitive Arbiter (Rust)]
        Arbiter <--> TS[Claude Agent Sidecar (TypeScript)]
        Arbiter <--> PY[Local ML / Heuristics (Python)]
        Arbiter <--> LLM[External Providers API]
    end

    subgraph Shared["Shared Governance (Vi & Truong)"]
        Contracts[Core Domain Types & Ports]
        StateMachine[State Machine & Transition Rules]
        EvidenceClosure[Acceptance Conditions & Evidence Bundles]
    end

    Kernel ===|JSON-RPC / Core Domain Types| Arbiter
    Contracts -.-> Truong
    Contracts -.-> Vi
```

### Vi — Product & AI/Research Lead
- **Primary Modules:** `crates/core-domain`, `crates/cognitive-runtime`, `crates/deliberation-contracts`, `crates/judgment-contracts`, `crates/context-compiler`, `sidecars/python-judgment`, `sidecars/ts-claude-agent`.
- **Languages:** Rust (interfaces & contracts), Python (fast heuristics), TypeScript (agent sidecars).
- **Core Responsibilities:**
  - Define Core Domain types and traits in `crates/core-domain` (Zero I/O, Single Source of Truth).
  - Design the Cognitive Arbiter workflows (System 1: Fast Heuristics / System 2: Deep LLM Reasoning).
  - Implement and maintain isolated sidecars (`python-judgment` for scoring/reranking, `ts-claude-agent` for Claude CLI/API adapters).
  - Prompt engineering, Context Compiler recipes, token budgeting, and AI evaluation datasets.
- **Personal Workspace & Reports:** [dev_docs/vi/](./vi/README.md)

### Truong — Platform & Software Engineering Lead (SE)
- **Primary Modules:** `crates/persistence-sqlite`, `crates/capability-gateway`, `crates/task-kernel`, `crates/workflow-runtime`, `crates/local-api`, `crates/evidence-engine`, `apps/custos-cli`, `apps/custosd`.
- **Languages:** 100% Rust & SQL.
- **Engineering Mindset (AI as a Black Box):**
  - **No AI Prompt Fluff:** No requirement to learn or track LLM prompts, token dynamics, temperature, or cognitive layers.
  - **Systems & Data First:** Treat all decisions from the AI runtime strictly as arbitrary JSON payloads requiring validation, durable SQLite persistence, and OS sandbox verification before command execution.
- **Core Responsibilities:**
  - SQLite schema design, WAL mode, migrations, and resilient CRUD operations via `rusqlite` / `sqlx`.
  - CLI binary development with `clap` and local HTTP daemon endpoints with `axum`.
  - OS-level safety gates: Git worktree isolation, Seatbelt (macOS) / Bubblewrap (Linux).
  - Observability infrastructure: Structured logging, OpenTelemetry tracing, process supervision.
- **Personal Workspace & Reports:** [dev_docs/truong/](./truong/README.md)

### Shared Ownership
Vi and Truong meet at the contract and verification boundaries. Neither works in isolation:
1. **Core Domain Contracts:** Shared types (`Task`, `TaskId`, `ActionIntent`, `Receipt`, `ExecutionPermit`, `ContinuationPacket`).
2. **State Machine Semantics:** Transition rules (`Draft` -> `Ready` -> `Running` -> `Verifying` -> `Succeeded` / `Failed` / `Paused`).
3. **Evidence Closure:** Defining exact completion verification rules before a task can transition to `Succeeded`.
4. **Security Boundaries:** Enforcing least privilege, prompt injection defense, and exact-payload human approvals.

---

## 2. Dedicated 5-Branch Git Workflow

To prevent merge conflicts, keep commit histories clean, and enable asynchronous reviews, the repository uses a dedicated 5-branch strategy:

| Branch | Purpose | Primary Operator | Invariants |
|---|---|---|---|
| `main` | **Production Release** | Both | Cleanest branch. Contains only thoroughly tested, production-ready code and finalized documentation. |
| `dev` | **Integration & System Test** | Both | Integration branch where `vi` and `truong` merge feature code for full workspace testing (`cargo test --workspace`). |
| `vi` | **Vi's Feature Branch** | Vi | AI domain traits, prompts, cognitive runtime, and sidecar implementations. |
| `truong` | **Truong's Feature Branch** | Truong | Backend systems, SQLite storage, CLI commands, and capability gateway sandboxing. |
| `report` | **Documentation & Status Sync** | Both | Dedicated branch to commit sprint reports, design notes, and reviews in `dev_docs/` **without polluting the `dev` code commit history**. |

### Daily Operational Workflow:
1. **Feature Implementation:** Vi develops on `vi`; Truong develops on `truong`.
2. **Status & Documentation Sync:** Switch to `report`, write updates into `dev_docs/vi/reports/` or `dev_docs/truong/reports/`, commit, and push for peer review.
3. **Integration Merge:** Once a milestone is ready, open a PR / merge from `vi` or `truong` into `dev`. Run full regression tests on `dev`.
4. **Production Release:** Once `dev` passes all verification suites and proves stable, merge `dev` into `main`.

---

## 3. Code Hand-Off & Transaction Protocol

### Interface Contract Rule
1. **Step 1:** Vi defines Rust `struct` or `trait` models in `crates/core-domain` (e.g., `struct Task { pub id: TaskId, pub status: TaskStatus }`).
2. **Step 2:** Truong consumes these shared types to implement `INSERT INTO tasks...` persistence in `crates/persistence-sqlite` or exposes them via `crates/local-api`.
3. **Step 3:** Neither engineer renames shared structs, traits, or contract files without prior mutual alignment.

### Atomic Transaction Boundaries
To guarantee crash resilience and prevent orphaned side effects, Custos enforces strict transaction boundaries:
- **Before executing any external side-effect:** The runtime MUST persist:
  1. `ActionIntent` with a unique idempotency key.
  2. `ExecutionPermit` identity and scope.
  3. Expected task state and Outbox entry.
- **After receiving side-effect result:** The runtime MUST persist:
  1. Execution result, exit code, and stdout/stderr references.
  2. `Receipt` and content-addressed `Artifact` hashes.
  3. Resulting domain events and next task state transition.

---

## 4. The First Shared Vertical Slice (`repo_explain`)

Before implementing mutable coding workflows (`bug_fix`) or sandboxed execution, Vi and Truong jointly construct the `repo_explain` vertical slice. This slice proves the entire architectural loop end-to-end without risking destructive mutations or requiring live LLM API keys:

### Scenario
A user runs: `custos run "Which module is responsible for persisting tasks, and why can the UI not directly access the database?"`

### Sequence of Execution
```text
CLI (custos-cli)
  └──> Local API (Axum)
        └──> Task Kernel (Validates Task & commits to SQLite Event Store)
              └──> Workspace Engine (Scans fixture repo, builds file inventory)
                    └──> Context Compiler (Extracts README and symbol ranges)
                          └──> FakeProvider (Returns explanation citing exact files & line ranges)
                                └──> Citation Verifier (Validates cited paths and ranges exist)
                                      └──> Artifact Store (Saves response and context manifest)
                                            └──> Evidence Engine (Binds citations to snapshot -> Succeeded)
                                                  └──> Crash Test (Restart daemon -> Timeline fully preserved)
```

### Why this slice is mandatory:
- Zero external API costs: Uses `FakeProvider` for deterministic, repeatable testing.
- Exercises all foundational components: CLI, API, Kernel, SQLite, Workspace Intelligence, Artifact Store, and Evidence Verification.
- Establishes the exact pattern for the subsequent `bug_fix` slice (which adds worktree isolation, permits, and test execution).

---

## 5. System-Wide Definition of Done (DoD)

A feature or pull request is NOT "Done" merely because it compiles. It must satisfy:

### Feature DoD Checklist
- [ ] **Domain Contract:** Typed contracts and invariants defined in `crates/core-domain`.
- [ ] **Invalid Case Handling:** Defensive parsing; invalid inputs return typed domain errors (`thiserror`).
- [ ] **Zero Unwraps:** No `.unwrap()` or `.expect()` in non-test Rust production code.
- [ ] **Test Coverage:** Unit tests for logic, contract tests for traits, integration tests for storage/IPC.
- [ ] **Telemetry:** Structured tracing spans with correlation IDs (`task_id`, `run_id`).
- [ ] **Redaction:** Zero sensitive tokens, keys, or private code logged in telemetry.
- [ ] **Cancellation & Timeout:** Operations observe `tokio::select!` cancellation tokens.
- [ ] **Crash Resilience:** Committed state recovers correctly upon process restart.
- [ ] **Documentation:** Module README updated or ADR created if architectural boundaries change.

### Workflow DoD Checklist
- [ ] Can run end-to-end against a local fixture repository.
- [ ] Generates an auditable Evidence Bundle with verifiable receipts.

---

## 6. Architecture Comprehension Checklist (12 Golden Questions)

Before modifying core code, every contributor must be able to answer these 12 questions:

1. **Why is Task (not chat) the central source of truth?**  
   *Because tasks define durable goals, invariants, budgets, and verifiable outcomes that survive model context resets and process restarts.*
2. **How does System 1 (Judgment) differ from System 2 (Deliberation)?**  
   *System 1 provides sub-second, low-cost reflex classification and risk screening without capability minting; System 2 provides multi-step deep reasoning and code synthesis.*
3. **Why are providers strictly forbidden from calling the shell directly?**  
   *To enforce least privilege, sandboxing, and prevent unchecked side effects. Only the Capability Gateway can execute commands after validating an `ExecutionPermit`.*
4. **How does a `CapabilityGrant` differ from an `ExecutionPermit`?**  
   *A Grant defines general permission scope for a task; a Permit is a single-use token cryptographically bound to the exact payload hash of a specific action.*
5. **What are the distinctions between Output, Artifact, Receipt, and Evidence?**  
   *Output is an unverified model assertion; an Artifact is an immutable content-addressed deliverable; a Receipt is proof that a tool actually ran; Evidence is an artifact/receipt accepted by a verifier.*
6. **How does Custos hand off work between providers (e.g. Codex -> Claude)?**  
   *Via a structured, provider-neutral `ContinuationPacket` containing goals, decisions, and artifacts—never by dumping raw chat transcripts.*
7. **Why is MCP strictly an integration boundary rather than an internal bus?**  
   *MCP is an external tool discovery and invocation protocol. Internal orchestration is strictly governed by the Task Kernel and core domain events.*
8. **How does Context differ from Memory?**  
   *Memory is knowledge preserved across tasks/workspaces with provenance and lifecycle; Context is a token-budgeted bundle assembled for a single inference call.*
9. **How does Custos determine the next step after a crash?**  
   *The Kernel inspects non-terminal tasks, reconciles the outbox, checks external execution receipts, and resumes safely without blindly retrying non-idempotent actions.*
10. **Why is verifier closure required before marking a task as `Succeeded`?**  
    *A task cannot succeed merely because an LLM claims it is done; objective verifiers must confirm that tests passed, diffs exist, and acceptance criteria are met.*
11. **What are Domain Packs permitted and forbidden to do?**  
    *Packs define workflows, prompts, and verifiers. They are forbidden from directly accessing the DB, minting permits, or bypassing Kernel state transitions.*
12. **What exact closed loop does the MVP prove?**  
    *The end-to-end loop: Create task -> Profile repo -> Compile context -> Invoke provider -> Propose action -> Approve -> Execute in worktree -> Collect receipt -> Verify -> Build evidence bundle -> Recover after restart -> Complete.*

---

## 7. Reporting & Documentation Conventions

- **Daily / Sprint Status:** Commit updates to [SPRINT_STATUS.md](./SPRINT_STATUS.md) on the `report` branch.
- **Vi's Notes & Reports:** Store in `dev_docs/vi/notes/` and `dev_docs/vi/reports/`.
- **Truong's Notes & Reports:** Store in `dev_docs/truong/notes/` and `dev_docs/truong/reports/`.
- **Format:** Standard GitHub Markdown, 100% Technical English, Zero Emojis.
