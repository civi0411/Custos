# Custos Codebase Inventory

> **Historical snapshot:** The paths and counts below belong to the dated checkout recorded in this document, not necessarily today's Cargo workspace. See [status rules](README.md) and re-run `cargo metadata` at PR-00.

## Overview

- Date: 2026-09-25
- Commit SHA: `acc6dbd2a1d7648227b26c4327200008d11951f7`
- Rust Edition: 2021
- Workspace Crates: 34 crates across `apps/`, `crates/`, `adapters/`, `tests/`, `xtask/`

---

## 1. Subsystem Breakdown and Crate Metrics

| Crate Name | Path | Owner | Test Count | Key Exports & Traits | Dependencies |
|---|---|---|---|---|---|
| `custos-core-domain` | `crates/core-domain` | Vi | 5 | `Task`, `TaskId`, `TaskStatus`, `ExecutionPermit`, `ContinuationPacket`, `DomainError` | Zero internal or external I/O crates |
| `custos-task-kernel` | `crates/task-kernel` | Truong | 2 | `TaskService`, `TaskCommand`, `TaskEvent`, `TaskReducer`, `TaskStateMachine` | `core-domain`, `chrono`, `tracing`, `thiserror` |
| `custos-workflow-runtime` | `crates/workflow-runtime` | Vinh | 0 (units) | `TaskLease`, `OutboxMessage`, `Dispatcher`, `WorkerLeaseManager` | `core-domain`, `task-kernel`, `tokio`, `tracing` |
| `custos-capability-gateway` | `crates/capability-gateway` | Truong | 0 (units) | `DeterministicGate`, `CapabilityGateway`, `GatewayError` | `core-domain`, `authority-engine`, `tracing` |
| `custos-cognitive-runtime` | `crates/cognitive-runtime` | Vi | 0 (units) | `RdcPhase`, `CognitiveArbiter`, `DeliberationLoop` | `core-domain`, `judgment-contracts`, `deliberation-contracts` |
| `custos-judgment-contracts` | `crates/judgment-contracts` | Vi | 0 | `JudgmentRequest`, `JudgmentResult`, `ConfidenceScore` | `core-domain`, `serde` |
| `custos-deliberation-contracts` | `crates/deliberation-contracts` | Vi | 0 | `DeliberationRequest`, `DeliberationResponse` | `core-domain`, `serde` |
| `custos-context-compiler` | `crates/context-compiler` | Vi | 2 | `ContextCompiler`, `Recipe`, `TokenBudget`, `RankedFact` | `core-domain`, `tracing` |
| `custos-evidence-engine` | `crates/evidence-engine` | Truong | 5 | `EvidencePipeline`, `EvidenceBundle`, `HashVerifier`, `ExitCodeVerifier`, `CitationVerifier` | `core-domain`, `sha2`, `tracing` |
| `custos-memory-service` | `crates/memory-service` | Vi | 0 | `MemoryTier`, `WorkingMemory`, `EpisodicMemory`, `SemanticMemory` | `core-domain`, `async-trait` |
| `custos-repo-intelligence` | `crates/repo-intelligence` | Vi | 3 | `RepoScanner`, `CodeGraph`, `SymbolExtractor`, `LlmView` | `tree-sitter`, `ignore`, `tracing` |
| `custos-artifact-store` | `crates/artifact-store` | Truong | 0 | `ArtifactStore`, `CasStorage`, `ArtifactManifest` | `core-domain`, `sha2`, `tokio` |
| `custos-persistence-sqlite` | `crates/persistence-sqlite` | Truong | 6 (4 unit + 2 int) | `SqliteStore`, `TaskRepository`, `SpanRepository`, `ContinuationRepository` | `rusqlite`, `core-domain`, `tracing` |
| `custos-local-api` | `crates/local-api` | Truong | 0 | `LocalApiServer`, `ApiRoutes`, `EventStream` | `axum`, `tokio`, `serde_json` |
| `custos-provider-sdk` | `crates/provider-sdk` | Vi | 0 | `ModelPort`, `ProviderRequest`, `ProviderEvent`, `TokenUsage` | `core-domain`, `async-trait` |
| `custos-domain-pack-sdk` | `crates/domain-pack-sdk` | Vi | 0 | `PackManifest`, `RecipeDefinition`, `PackLoader` | `serde_yaml`, `serde_json` |
| `custos-observability` | `crates/observability` | Truong | 0 | `TelemetryConfig`, `init_tracing_subscriber` | `tracing-subscriber`, `tracing` |
| `custos-authority-engine` | `crates/authority-engine` | Truong | 0 | `AuthorityEngine`, `PolicyEvaluator`, `GrantManager`, `AuditLog` | `core-domain`, `sha2`, `chrono` |
| `custosd` | `apps/custosd` | Truong | 0 | `CustosDaemon`, `DaemonSupervisor` | `tokio`, `clap`, `persistence-sqlite`, `local-api` |
| `custos-cli` | `apps/custos-cli` | Truong | 11 | `CliApp`, `CliCommand`, `TerminalRenderer`, UI components | `clap`, `tokio`, `crossterm` |
| `custos-adapter-provider-fake` | `adapters/providers/fake` | Vi | 0 | `FakeModelProvider`, `FakeStream` | `provider-sdk`, `core-domain` |
| `custos-adapter-provider-claude` | `adapters/providers/claude` | Vi | 0 | Claude Messages API adapter | `provider-sdk`, `reqwest` / `http` |
| `custos-adapter-provider-codex` | `adapters/providers/codex` | Vi | 0 | OpenAI / Codex adapter | `provider-sdk` |
| `custos-adapter-provider-antigravity` | `adapters/providers/antigravity` | Vi | 0 | Antigravity engine adapter | `provider-sdk` |
| `custos-adapter-provider-local-model` | `adapters/providers/local-model` | Vi | 0 | Local Ollama HTTP adapter | `provider-sdk` |
| `custos-adapter-judgment-rules` | `adapters/judgments/rules` | Vi | 0 | System 1 rule-based classifier | `judgment-contracts` |
| `custos-adapter-judgment-onnx` | `adapters/judgments/onnx` | Vi | 0 | Local ONNX judgment runtime | `judgment-contracts` |
| `custos-adapter-judgment-jev` | `adapters/judgments/jev` | Vi | 0 | Judgment Evaluation Vector engine | `judgment-contracts` |
| `custos-adapter-sandbox-macos-seatbelt` | `adapters/sandboxes/macos-seatbelt` | Truong | 0 | `sandbox-exec` process wrapper | `capability-gateway` |
| `custos-adapter-sandbox-linux-bubblewrap` | `adapters/sandboxes/linux-bubblewrap` | Truong | 0 | `bwrap` container execution wrapper | `capability-gateway` |
| `custos-adapter-tools` | `adapters/tools` | Truong | 0 | Consolidated FS, Shell, Git tools | `capability-gateway` |
| `custos-tests-contract` | `tests/contract` | Truong + Vi | 2 | Schema and provider conformance tests | `provider-sdk`, `core-domain` |
| `custos-tests-e2e` | `tests/e2e` | All | 2 | End-to-end and vertical slice tests | Full workspace integration |
| `xtask` | `xtask` | Truong | 0 | Build, lint, and verification automation | `clap` |

---

## 2. Invariant Verification

1. **Acyclic Dependency Flow:**
   - `core-domain` has zero workspace dependencies.
   - `task-kernel` depends only on `core-domain`.
   - `persistence-sqlite` and `capability-gateway` sit above `task-kernel`.
   - `custosd` acts as the composition root.
2. **Persistence Boundary:**
   - Direct SQLite access is strictly encapsulated in `custos-persistence-sqlite`.
3. **Provider Isolation:**
   - No cloud LLM vendor SDKs are imported into core crates. All provider traffic routes through `provider-sdk`.
