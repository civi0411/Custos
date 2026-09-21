# Standardized Naming Conventions

> **Status:** Canonical Baseline v4.0  
> **Source:** Part XII (§34) Canonical Specification

To enforce strict semantic consistency across Rust source code, data schemas, documentation, APIs, and persistence layers, Custos adheres to standardized naming conventions across all components:

---

## Global Naming Conventions Matrix

| Tier / Component | Casing & Format | Concrete Example | Notes & Enforcement Scope |
|---|---|---|---|
| **Product Name** | PascalCase | `Custos` | Official runtime brand name |
| **Repository Name** | kebab-case | `custos` | Git repository identifier |
| **Rust Crates** | kebab-case | `custos-kernel`, `custos-gateway`, `custos-cognitive` | Cargo workspace crate names |
| **Binary Executables** | lowercase | `custosd`, `custos` | `custosd` (daemon), `custos` (CLI) |
| **Modules (Rust/Python)** | snake_case | `task_state`, `event_store`, `context_scoring` | Source files (`.rs`, `.py`) and module namespaces |
| **Domain Entities** | PascalCase, singular | `Task`, `Contract`, `Artifact`, `Run` | Identifiable business entities with persistent state |
| **Value Objects** | PascalCase | `TaskId`, `ExecutionPermit`, `ContextPack` | Immutable, typed domain values |
| **Enums** | PascalCase | `TaskStatus`, `EvidenceLevel`, `RiskTier` | State enumerations |
| **Traits / Ports** | PascalCase, suffix `Port` | `ProviderPort`, `JudgmentPort`, `StoragePort` | Abstract interface boundaries |
| **Functions & Methods** | snake_case, verb prefix | `create_task()`, `verify_evidence()`, `commit_step()` | Deterministic business actions |
| **Commands (CQRS)** | PascalCase, imperative | `CreateTask`, `DispatchWorker`, `CancelTask` | State mutation requests |
| **Events (Event Sourced)** | PascalCase, past tense | `TaskCreated`, `WorkerDispatched`, `StepVerified` | Immutable historical facts |
| **Schemas (Versioned)** | `<ns>.<name>.v<n>` | `custos.task.v1`, `custos.permit.v1` | Versioned JSON and Protobuf schemas |
| **REST / IPC Endpoints** | kebab-case, plural | `/v1/tasks`, `/v1/execution-permits` | HTTP and RPC paths |
| **CLI Commands** | kebab-case | `custos task start`, `custos audit trace` | Terminal subcommands |
| **Domain Packs** | kebab-case | `engineering`, `research`, `personal` | Domain bundle identifiers |
| **Task Types** | `<domain>.<type>` | `engineering.bug_fix`, `research.literature_scan` | Task taxonomy identifiers |
| **Workflows** | `<domain>.<name>@<v>` | `engineering.bug_fix@1`, `research.claim_verify@2` | Versioned workflow blueprints |
| **Worker Roles** | `<domain>.<role>` | `engineering.explorer`, `engineering.patcher` | Ephemeral worker operational roles |
| **AI Providers** | lowercase | `codex`, `claude`, `antigravity`, `ollama` | Provider adapter identifiers |
| **Database Tables** | snake_case, plural | `tasks`, `task_events`, `execution_permits` | SQLite relational tables |
| **Foreign Keys / Columns** | snake_case | `task_id`, `created_at`, `payload_hash` | SQLite columns |
| **Error Types** | PascalCase, suffix `Error`| `TaskNotFoundError`, `PermitExpiredError` | Typed error enumerations |
| **Metrics / Telemetry** | snake_case, prefix `custos_` | `custos_task_duration_seconds`, `custos_tokens_used` | OpenTelemetry metric instruments |
| **Environment Variables** | UPPER_SNAKE_CASE | `CUSTOS_HOME`, `CUSTOS_LOG_LEVEL` | Environment configuration keys |

---

## Supplementary Naming Guidelines

1. **Clarity Over Brevity:** Avoid ambiguous abbreviations (e.g., use `verification_result` rather than `vr_res`).
2. **Eliminate Non-Semantic Jargon:** Avoid proprietary acronyms lacking technical clarity (remove JEP, UJE; standardize on `DecisionContract` and `ExecutionPermit`).
3. **Strict CQRS Separation:** Commands represent intent to modify state (subject to failure); Events record immutable historical facts in the Event Store.
