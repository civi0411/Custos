# Monorepo Topology, Crate Boundaries & Physical Codebase Catalog

> **Classification:** Definitive Monorepo Physical Catalog & Architecture Map 
> **Source of Truth:** Authoritatively governed by [Custos Master Specification](../../Custos.md). 
> **Documentation Triad:** Synchronized with [`../../Custos.md`](../../Custos.md) and [`../architecture/README.md`](../architecture/README.md).

---

## 1. System Invariants & Architectural Rings

Custos is structured as a **Topic-Driven Monorepo** organized into four concentric architectural rings with strict inward dependency flow:

```
[ Ring 4: Applications & SDK ]
 apps/daemon, apps/cli, apps/desktop, sdk/client-rust
 
 
[ Ring 3: Vertical Workbenches ]
 workbenches/assistant, coding, quant, research
 
 
[ Ring 2: Platform Engine ]
 platform/engine (Governed loop, S1/S2 OI, verifiers)
 
 
[ Ring 1: Storage & Integrations ]
 platform/storage (SQLite WAL), platform/integrations (models, adapters)
 
 
[ Ring 0: Contracts & Pure Domain ]
 platform/contracts (Zero-I/O domain entities, traits, port interfaces)
```

### Inviolable Architectural Invariants

1. **INV-01 (Zero-I/O Domain Invariant):** `platform/contracts` (`custos-contracts`) has 0 I/O dependencies (no `tokio`, no `rusqlite`, no `reqwest`, no `std::fs`).
2. **INV-07 (Single-Writer SQLite WAL):** `platform/storage` (`custos-storage`) manages single-writer connection pooling with dedicated multi-reader pools to prevent WAL starvation and database contention.
3. **INV-11 (Zero-Mock in Production):** Production paths execute genuine logic. Mocks and synthetic stubs are restricted to `#[cfg(any(test, feature = "test-support"))]`.
4. **INV-12 (Process Group & Zombie Elimination):** Subprocesses spawned by harnesses or sandboxes in `platform/integrations` mandate `setpgid(0)` and are terminated cleanly via `SIGKILL` on `-pid` upon drop.
5. **Thin Daemon Composition:** `apps/daemon` serves strictly as the Sole Composition Root and thin Local API transport adapter; domain-specific business logic is isolated within vertical workbenches.

---

## 2. Monorepo Crate Registry (17 Workspace Packages)

The workspace manifest (`Cargo.toml`) defines 17 canonical crates:

| Package Name | Physical Path | Ring | Architectural Role | Key Invariants |
|---|---|---|---|---|
| `custos-contracts` | `platform/contracts` | Ring 0 | Pure domain entities, port traits, event schemas | Zero-I/O, deterministic math, no async |
| `custos-storage` | `platform/storage` | Ring 1 | SQLite WAL persistence, partition engine, Outbox, CAS | Single-writer connection pool |
| `custos-integrations`| `platform/integrations`| Ring 1 | Model providers, sandbox, harnesses, compute relays | Process group isolation, secure credentials |
| `custos-engine` | `platform/engine` | Ring 2 | Task execution loop, S1/S2 OI, Completion Gate | Budgeted execution, verifier isolation |
| `custos-workbench-assistant` | `workbenches/assistant` | Ring 3 | Assistant drafts, contact books, outbox actions | Workbench isolation, 5-part layout |
| `custos-workbench-coding` | `workbenches/coding` | Ring 3 | Code editing, AST parsing, Git worktree isolation | Git worktree sandboxing, test verifiers |
| `custos-workbench-quant` | `workbenches/quant` | Ring 3 | Financial models, backtest engine, order execution | Zero-Mock market data, Sharpe verifiers |
| `custos-workbench-research` | `workbenches/research` | Ring 3 | Claim synthesis, passage anchors, ALCE verifiers | Compensating saga rollback in batch creation |
| `custos-daemon` | `apps/daemon` | Ring 4 | Sole server composition root, Local API dispatcher | Thin transport, no direct SQLite mutations |
| `custos-app-cli` | `apps/cli` | Ring 4 | Pure Rust headless terminal CLI | Communicates strictly via `custos-sdk` RPC |
| `custos-desktop` | `apps/desktop/native` | Ring 4 | Thin Tauri v2 native desktop wrapper | Zero business logic, thin host |
| `custos-protocol` | `sdk/protocol` | Cross-Ring | Wire protocol envelopes, RPC methods, DTO schemas | Transport-neutral, Serde symmetric |
| `custos-sdk` | `sdk/client-rust` | Ring 4 | Typed Rust client over Local API transport | Reusable client library |
| `tests-contract` | `tests/contract` | Test | Invariant and contract validation test suites | Automated compile and runtime checks |
| `tests-e2e` | `tests/e2e` | Test | End-to-end integration workflows | Full daemon lifecycle validation |
| `tests-testkit` | `tests/testkit` | Test | Test harnesses, factories, and assertion tools | Isolated test helpers |
| `xtask` | `xtask` | Tooling | Quality gate automation, boundary checks | Automated CI verification runner |

---

## 3. Subsystem Physical Layout & Module Inventory

### 3.1. `platform/contracts` (`custos-contracts`) — Ring 0

The foundational layer defining all domain aggregates, value objects, and abstract ports.

| Module | Source Path | Architectural Responsibility | Key Structs / Traits |
|---|---|---|---|
| `authority` | `platform/contracts/src/authority/` | Action intents, capability grants, execution permits | `ActionIntent`, `ExecutionPermit`, `Scope`, `Grant` |
| `context` | `platform/contracts/src/context/` | Context compilation contracts, token budgets | `ContextPack`, `ContextReceipt`, `TokenBudget` |
| `contributions`| `platform/contracts/src/contributions/`| Dynamic module and skill contribution metadata | `ContributionManifest`, `SkillDescriptor` |
| `conversation` | `platform/contracts/src/conversation/` | Session identities, turn models, messages | `SessionId`, `ConversationTurn`, `UserMessage` |
| `evidence` | `platform/contracts/src/evidence/` | Verification records, criteria evaluation, CAS hashes | `EvidenceRecord`, `CriterionStatus`, `CasHash` |
| `execution` | `platform/contracts/src/execution/` | Runtime ports, compute recipes, telemetry contracts | `AgentRuntimePort`, `ModelPort`, `ComputePort` |
| `identity` | `platform/contracts/src/identity/` | Actor attribution, signing keys, provenance | `ActorId`, `ProvenanceRecord` |
| `inference` | `platform/contracts/src/inference/` | Inference requests, streaming events, usage tracking | `ProviderRequest`, `ProviderEvent`, `UsageReceipt` |
| `resources` | `platform/contracts/src/resources/` | Unified Resource References (URR), workspaces | `ResourceRef`, `WorkspaceId`, `ExecutionWorkspace` |
| `task` | `platform/contracts/src/task/` | Task aggregate root, state machine, revisions | `Task`, `TaskContract`, `TaskStatus`, `Revision` |

### 3.2. `platform/storage` (`custos-storage`) — Ring 1

Durable relational persistence backed by SQLite WAL with dynamic database partitioning.

| Module | Source Path | Architectural Responsibility | Key Structs / Traits |
|---|---|---|---|
| `connection` | `platform/storage/src/connection/` | Connection pooling (single-writer / multi-reader) | `StoragePool`, `SingleWriterConnection` |
| `store` | `platform/storage/src/store/` | SQLite transaction orchestration, schema migrations | `SqliteStore`, `MigrationRunner` |
| `repositories`| `platform/storage/src/repositories/`| Specialized entity repositories | `TaskRepository`, `SessionRepository`, `EvidenceRepository` |
| `artifacts` | `platform/storage/src/artifacts/` | Content-addressed storage (CAS) file manager | `CasArtifactStore`, `ArtifactMetadata` |
| `transactions`| `platform/storage/src/transactions/`| Transactional Outbox pattern engine | `OutboxQueue`, `OutboxMessage` |
| `recovery` | `platform/storage/src/recovery/` | Crash reconciliation and state recovery | `ReconciliationPass`, `CrashRecoveryEngine` |

### 3.3. `platform/integrations` (`custos-integrations`) — Ring 1

External provider adapters, execution sandboxes, and hardware compute bridges.

| Module | Source Path | Architectural Responsibility | Key Structs / Traits |
|---|---|---|---|
| `models` | `platform/integrations/src/models/` | Cloud LLM provider codecs (OpenAI, Anthropic, Gemini, Groq, DeepSeek) & local `llama.cpp` | `OpenAiCodec`, `AnthropicCodec`, `LlamaCppBackend` |
| `harnesses` | `platform/integrations/src/harnesses/`| CLI agent supervisors (Claude Code, Codex, OpenCode) | `ClaudeCodeHarness`, `CodexHarness`, `OpenCodeHarness` |
| `sandbox` | `platform/integrations/src/sandbox/` | OS sandbox enforcement (macOS Seatbelt, Linux Bubblewrap) | `SeatbeltProfile`, `BubblewrapSandbox`, `ProcessDropGuard` |
| `protocols` | `platform/integrations/src/protocols/`| External protocol adapters (MCP Federation, A2A Agent Cards) | `McpClientAdapter`, `A2aCardRouter` |
| `compute` | `platform/integrations/src/compute/` | Accelerator relays (Google Colab, Kaggle, SSH GPU fleet) | `ColabRelay`, `KaggleRelay`, `SshGpuFleetAdapter` |
| `credentials`| `platform/integrations/src/credentials/`| Credential storage, environment variable resolvers | `CredentialVault`, `ApiKeyResolver` |
| `filesystem` | `platform/integrations/src/filesystem/` | Safe filesystem operations and Git worktree provisioning | `WorktreeManager`, `SafeFileSystem` |
| `terminal` | `platform/integrations/src/terminal/` | PTY driver and interactive terminal process handling | `PtySessionManager`, `PtyStream` |
| `notebook` | `platform/integrations/src/notebook/` | Scoped Jupyter Python kernel execution | `JupyterKernelAdapter`, `CellOutputBuffer` |

### 3.4. `platform/engine` (`custos-engine`) — Ring 2

Supervised execution loop, cognitive coordination, task scheduling, and verification.

| Module | Source Path | Architectural Responsibility | Key Structs / Traits |
|---|---|---|---|
| `execution` | `platform/engine/src/execution/` | Governed agent loop, step mediation, budget checks | `GovernedAgentLoop`, `StepExecutionPlan` |
| `orchestration`| `platform/engine/src/orchestration/`| S1 FastPathFilter, S2 Replanner, Pareto OI Selector, T0–T6 Topologies | `ParetoSelector`, `FastPathFilter`, `T5DagCoordinator` |
| `authority` | `platform/engine/src/authority/` | Authority mediation, permit signing, taint tracking | `AuthorityEngine`, `TaintTrackingEngine` |
| `evidence` | `platform/engine/src/evidence/` | Completion Gate evaluation, criterion verifiers | `CompletionGate`, `CriteriaEvaluator` |
| `components` | `platform/engine/src/components/` | Workbench pack registry and dynamic discovery | `PackRegistry`, `ModuleLifecycleManager` |
| `context` | `platform/engine/src/context/` | Lexical and semantic context compiler, budget compaction | `ContextCompiler`, `BudgetCompactor` |
| `task` | `platform/engine/src/task/` | Task FSM state machine transitions, lifecycle hooks | `TaskLifecycleCoordinator`, `StateTransitionEngine` |
| `supervision` | `platform/engine/src/supervision/` | Agent run supervision, error handling, retry policies | `RunSupervisor`, `CircuitBreaker` |

### 3.5. Vertical Workbenches (`workbenches/*`) — Ring 3

Sovereign vertical domains adhering to the symmetrical 5-part structure:
`manifest/` (metadata), `migrations/` (DB tables), `recipes/` (workflows), `schemas/` (JSON specs), `src/` (Rust code).

- **`workbenches/assistant`:** Conversational workflows, assistant draft management (`AssistantDraft`), outbox communications, personal facts.
- **`workbenches/coding`:** Isolated Git worktree execution, AST parsing, automated code generation, and test verification gates.
- **`workbenches/quant`:** Financial time-series ingestion, alpha discovery, backtesting engine, `DeflatedSharpeVerifier`, `LookaheadBiasChecker`.
- **`workbenches/research`:** Literature extraction, claim synthesis (`ResearchClaim`), 4-step ALCE verification, and compensating saga rollbacks.

### 3.6. Applications (`apps/*`) & SDK (`sdk/*`) — Ring 4

- **`apps/daemon` (`custos-daemon`):** The Sole Composition Root. Assembles platform services, registers enabled workbenches, and exposes the Local API (HTTP and IPC).
- **`apps/cli` (`custos-app-cli`):** Pure Rust headless CLI (`custos`) consuming `custos-sdk` over Local API RPC.
- **`apps/desktop`:**
 - `apps/desktop/native`: Thin Tauri v2 native desktop application wrapper.
 - `apps/desktop/ui`: High-performance React 19 UI styled with Graphite JetBlack design tokens, organized around the 3-Tier Hierarchy (Project -> Workspace -> Worktree).
- **`sdk/protocol` (`custos-protocol`):** Serde wire envelopes, request/response DTOs, and RPC method schemas.
- **`sdk/client-rust` (`custos-sdk`):** Strongly typed async client (`LocalApiClient`) for daemon communication.

---

## 4. Documentation Triad & 5-Step Synchronization Protocol

All AI coding assistants (Cursor, Claude Code, Antigravity, Windsurf) MUST follow this sequential pipeline before modifying code:

1. **Step 1 (Architectural Consensus):** Clarify target crate ownership and verify boundary invariants.
2. **Step 2 (Update `Custos.md`):** Synchronize the root Single Source of Truth.
3. **Step 3 (Update `docs/` Topic Specs):** Document functional requirements in the matching topical specification under `docs/architecture/`.
4. **Step 4 (Update `codebase-architecture.md`):** Register modified or newly created source files in this physical catalog.
5. **Step 5 (Code Implementation & Gate Verification):** Implement code and verify against all automated quality gates.

---

## 5. Mandatory Quality Gates & Verification Commands

Every change must satisfy the three automated quality gates prior to commit:

```bash
# Gate 1: Rust Workspace Compilation & Linting
cargo check --workspace --all-targets --offline

# Gate 2: Architectural Boundaries & Line Ceiling (<= 600 LOC)
cargo xtask check-boundaries

# Gate 3: Desktop UI Canvas Smoke Test Suite
node tests/desktop/canvas-smoke.mjs
```
