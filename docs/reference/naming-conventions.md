# Custos Standardized Naming Conventions

> **Classification:** Normative Implementation Reference  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Developer Reference](README.md).

To enforce absolute semantic consistency across Rust source code, IPC contracts, database schemas, CLI tools, and telemetry, Custos enforces the following standardized naming conventions across all modules.

---

## 1. Global Naming Matrix

| Tier / Component | Casing & Format | Concrete Example | Notes & Enforcement Scope |
|---|---|---|---|
| **Product Brand** | PascalCase | `Custos` | Official runtime brand name |
| **Repository Name** | kebab-case | `custos` | Git repository root identifier |
| **Rust Crates** | kebab-case | `custos-domain`, `custos-core`, `custos-persistence`, `custos-runtime` | Canonical Cargo workspace crate names (11 target crates) |
| **Binary Executables** | lowercase | `custosd`, `custos` | `custosd` (daemon service), `custos` (CLI utility) |
| **Source Modules** | snake_case | `task_state`, `event_store`, `context_scoring` | Source files (`.rs`) and Rust module namespaces |
| **Domain Entities** | PascalCase, singular | `Task`, `Contract`, `Artifact`, `Run` | Identifiable business entities with persistent lifecycle |
| **Value Objects** | PascalCase | `TaskId`, `ExecutionPermit`, `ContextPack` | Immutable, typed domain values |
| **State Enums** | PascalCase | `TaskStatus`, `EvidenceLevel`, `RiskTier` | Closed state and taxonomy enumerations |
| **Traits / Ports** | PascalCase, suffix `Port` | `ProviderPort`, `JudgmentPort`, `StoragePort` | Hexagonal architectural interface boundaries |
| **Functions & Methods** | snake_case, verb prefix | `create_task()`, `verify_evidence()`, `commit_step()` | Deterministic actions and state transitions |
| **CQRS Commands** | PascalCase, imperative | `CreateTask`, `DispatchWorker`, `CancelTask` | State mutation requests (may succeed or fail) |
| **Domain Events** | PascalCase, past tense | `TaskCreated`, `WorkerDispatched`, `StepVerified` | Immutable historical facts stored in the Event Store |
| **Wire Schemas** | `<ns>.<name>.v<n>` | `custos.task.schema.json`, `custos.envelope.v1.json` | Versioned JSON and Protobuf wire formats |
| **REST / IPC Endpoints** | kebab-case, plural | `/tasks`, `/execution-permits`, `/sessions` | HTTP and Unix Domain Socket paths |
| **CLI Commands** | kebab-case | `custos task start`, `custos audit trace` | Terminal subcommands and flags |
| **Domain Packs** | kebab-case | `engineering`, `research`, `assistant` | Pack bundle taxonomy identifiers |
| **Task Types** | `<domain>.<type>` | `engineering.bug_fix`, `research.literature_scan` | Task taxonomy categorization |
| **Workflows** | `<domain>.<name>@<v>` | `engineering.bug_fix@1`, `research.claim_verify@2` | Versioned workflow blueprints |
| **Worker Roles** | `<domain>.<role>` | `engineering.explorer`, `engineering.patcher` | Ephemeral worker operational roles |
| **AI Providers** | lowercase | `codex`, `claude`, `antigravity`, `ollama` | Provider adapter identifiers |
| **Database Tables** | snake_case, plural | `tasks`, `task_events`, `execution_permits` | SQLite relational storage tables |
| **Database Columns** | snake_case | `task_id`, `created_at`, `payload_hash` | Relational table columns and foreign keys |
| **Error Types** | PascalCase, suffix `Error` | `TaskNotFoundError`, `PermitExpiredError` | Typed Rust error definitions (`thiserror`) |
| **Metrics / Telemetry** | snake_case, prefix `custos_` | `custos_task_duration_seconds`, `custos_tokens_used` | OpenTelemetry metric instruments |
| **Environment Variables** | UPPER_SNAKE_CASE | `CUSTOS_HOME`, `CUSTOS_LOG_LEVEL` | Environment and process configuration keys |

---

## 2. Architectural Naming Rules

### 2.1 Clarity Over Brevity
- Avoid cryptic abbreviations in production code. Use `verification_result` rather than `vr_res`, `execution_permit` rather than `ep`.
- Standard abbreviations permitted: `id` (identifier), `ms` (milliseconds), `cas` (Content-Addressed Storage), `fsm` (Finite State Machine).

### 2.2 Semantic CQRS Separation
- **Commands are Imperative Verbs:** `ScheduleTask`, `RevokePermit`, `AppendEvent`. They represent an intent to modify state and can be rejected by the domain.
- **Events are Past Tense Verbs:** `TaskScheduled`, `PermitRevoked`, `EventAppended`. They represent an indisputable historical occurrence already committed to the ledger.

### 2.3 Explicit Boundary Identifiers
- Storage interfaces must be suffixed with `Repository` or `Port` (e.g., `TaskRepository`, `EventStorePort`).
- Adapter implementations in `custos-runtime` or `custos-persistence` must reflect their concrete backend (e.g., `SqliteTaskRepository`, `StdioMcpTransport`).
# CLI mode labels

The neutral interactive prompt is `custos`. Specialized prompt labels are `custos-code`, `custos-research`, and `custos-assistant`. The slash palette uses a two-level path: `/` exposes `/mode` and actions, while `/mode` exposes `/code`, `/research`, and `/assistant`.
