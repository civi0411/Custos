# Custos Source Path Map and C0 Baseline Inventory

> **Historical snapshot:** This inventory uses an earlier crate layout. It is preserved for provenance, not active navigation or ownership. See [status rules](README.md) and the [implementation blueprint](../development/implementation-blueprint.md).

## Document Metadata

- Status: Baseline Verified (C0 Inventory Phase)
- Date: 2026-09-25
- Repository: `Custos`
- Target Branch: `vi`
- Commit SHA: `acc6dbd2a1d7648227b26c4327200008d11951f7`
- Upstream Reference: Goose (`9adae14b64587a26275fe7c4a822a8e8ccdbd3fd`)
- Owners: Vi (AI Systems), Truong (Core Platform & Security), Vinh (Agent Systems & Coordination)

---

## 1. Baseline Git State & Working Tree

### 1.1 Uncommitted Modifications in Working Tree
The following 4 documentation and specification files contain active uncommitted updates that are preserved:
- `README.md`
- `docs/00-start-here.md`
- `docs/architecture/overview.md`
- `docs/canonical-specification.md`

### 1.2 Verification Suite (52 Tests Passing, 0 Failing)

Command executed: `cargo test --workspace`  
Result: 52 passed, 0 failed, 0 ignored.

| Test Suite / Binary | Enclosing Crate | Test Count | Status | Notes |
|---|---|---|---|---|
| `test_full_cqrs_lifecycle`, `test_reducer_advancement` | `custos-task-kernel` | 2 | PASSED | CQRS command processing, state machine transitions, event emission |
| `test_task_crud_and_list`, `test_span_lifecycle`, `test_foreign_key_enforcement`, `test_continuation_packets` | `custos-persistence-sqlite` | 4 | PASSED | SQLite WAL CRUD, foreign key cascade, span tracking, continuation persistence |
| `test_fresh_database_has_all_tables`, `test_incremental_upgrade_path_preserves_data` | `custos-persistence-sqlite` (integration) | 2 | PASSED | Migrations 0001 through 0005 schema validation and idempotency across all 13 tables |
| `test_scan_and_symbol_extraction`, `test_code_graph_and_neighborhood`, `test_llm_view_and_skeleton` | `custos-repo-intelligence` | 3 | PASSED | Tree-sitter code graph, symbol extraction, skeleton generation |
| `test_hash_verifier`, `test_command_exit_code_verifier_success/failure`, `test_pipeline_evaluate_requirements`, `test_citation_verifier_valid_and_invalid`, `test_semantic_support_and_unsupported_citation_vector` | `custos-evidence-engine` | 6 | PASSED | Hash checks, process exit codes, pipeline verification, syntactic citation checks, semantic support evaluation |
| `test_relevance_ranking`, `test_token_budget_enforcement` | `custos-context-compiler` | 2 | PASSED | Relevance ranking, token budgeting |
| `test_run_lifecycle`, `test_valid_task_transitions`, `test_new_id_has_prefix`, `test_digest_deterministic`, `test_packet_creation_and_verification`, `test_task_revision_and_stale_detection`, `test_action_lifecycle_and_no_blind_retry`, `test_worker_run_bounded_retry`, `test_workflow_ir_construction`, `test_verification_claim_stale_transition` | `custos-core-domain` | 10 | PASSED | Strongly-typed IDs, immutable domain invariants, TaskRevision, ActionLifecycleState, WorkerRun retry, WorkflowIR, EvidenceStatus |
| `test_local_api_dispatcher_create_and_get` | `custos-local-api` | 1 | PASSED | IPC request/response dispatcher, task creation, retrieval, and advancement |
| `test_local_model_provider_ollama_config` | `custos-adapter-provider-local-model` | 1 | PASSED | Ollama HTTP endpoint configuration and generation contract |
| `test_rdc_full_cycle` | `custos-cognitive-runtime` | 1 | PASSED | Resolve-Delegate-Check deliberation loop and stale check validation |
| `test_path_traversal_prevention`, `test_tools_registry_definitions` | `custos-adapter-tools` | 2 | PASSED | Path traversal prevention, symlink safety, and standard tools registry |
| Deterministic gate tests | `custos-capability-gateway` | 3 | PASSED | Permit gating, low/high risk classification |
| `test_fake_provider_conformance` | `custos-tests-contract` | 1 | PASSED | ModelPort contract conformance test against FakeProvider |
| `test_all_canonical_schemas_exist_and_valid_json` | `custos-tests-contract` | 1 | PASSED | Protocol JSON schemas syntax and presence |
| `test_full_file_backed_lifecycle_and_restart` | `custos-tests-e2e` | 1 | PASSED | CLI file-backed restart and session persistence |
| `test_repo_explain_vertical_slice_end_to_end` | `custos-tests-e2e` | 1 | PASSED | Complete vertical slice for repo explain task |
| UI art, assets, layout tests | `custos-cli` | 11 | PASSED | CLI rendering, banner, prompt, diff, spinner |

---

## 2. Database Migrations Status

SQLite migrations located in `crates/persistence-sqlite/migrations/`:

| Migration File | Tables Created | Status | Notes |
|---|---|---|---|
| `0001_core.sql` | `tasks`, `spans`, `continuation_packets` | EXISTS | Core state machine storage, parent-child span hierarchy |
| `0002_runtime.sql` | `task_leases`, `outbox_messages` | EXISTS | Worker coordination, optimistic concurrency leases, outbox events |
| `0003_authority.sql` | `grants`, `permits`, `approval_requests`, `audit_log` | EXISTS | Zero-trust authority storage, cryptographic permit tracking, audit hash-chain |
| `0004_usage.sql` | `usage_ledger`, `token_reservations` | EXISTS | Custos-native budget reservation and token settlement ledger (PR-02) |
| `0005_evidence.sql` | `evidence_bundles`, `verification_receipts` | EXISTS | Durable evidence bundles, criterion tracking, and verification receipts (PR-02) |


---

## 3. Workspace Crates Source Path Map

Verified against active filesystem and `Cargo.toml`.

| Crate Path | Primary Language | Owner | Status | Action | Architectural Role & Implementation Details |
|---|---|---|---|---|---|
| `crates/core-domain` | Rust | Vi | EXISTS | EXTEND | Zero-dependency core contracts. Implements `Task`, `TaskId`, `TaskStatus`, `ExecutionPermit`, `ContinuationPacket`. Needs `TaskRevision` and `stale` logic in PR-01. |
| `crates/task-kernel` | Rust | Truong | EXISTS | EXTEND | CQRS task lifecycle reducer and event publisher. Manages state machine invariants. |
| `crates/workflow-runtime` | Rust | Vinh | EXISTS | EXTEND | Concurrency engine, worker leases, outbox pattern dispatcher, DAG execution. |
| `crates/capability-gateway` | Rust | Truong | EXISTS | EXTEND | Gate for all tool execution. Implements `DeterministicGate`. Requires execution permit validation before dispatch. |
| `crates/cognitive-runtime` | Rust | Vi | EXISTS (Partial) | EXTEND | Deliberation and cognitive flows. `rdc.rs` currently contains enum scaffold; requires real execution loop. |
| `crates/judgment-contracts` | Rust | Vi | EXISTS | KEEP | Contracts and types for System 1 / System 2 judgments. |
| `crates/deliberation-contracts` | Rust | Vi | EXISTS | KEEP | Deliberation request and response protocol types. |
| `crates/context-compiler` | Rust | Vi | EXISTS | EXTEND | Token budgeting, fact retrieval, relevance ranking. |
| `crates/evidence-engine` | Rust | Truong | EXISTS | EXTEND | Hash, exit code, and physical citation verifiers. Requires addition of semantic support verification in PR-07. |
| `crates/memory-service` | Rust | Vi | EXISTS (Scaffold) | EXTEND | 5-tier memory abstraction. Trait definitions present; needs storage-backed implementations. |
| `crates/repo-intelligence` | Rust | Vi | EXISTS | EXTEND | AST parsing, code graph, skeleton extraction. Underpins Engineering Pack. |
| `crates/artifact-store` | Rust | Truong | EXISTS | EXTEND | CAS (Content-Addressed Storage) for build artifacts and intermediate diffs. |
| `crates/persistence-sqlite` | Rust + SQL | Truong | EXISTS | EXTEND | SQLite WAL store with connection pooling and idempotent migrations. |
| `crates/local-api` | Rust | Truong | EXISTS (Scaffold) | IMPLEMENT | Local IPC daemon HTTP/RPC endpoints. Scheduled for completion in PR-02b. |
| `crates/provider-sdk` | Rust | Vi | EXISTS | EXTEND | Provider abstractions (`ModelPort`). Zero direct cloud dependencies in core. |
| `crates/domain-pack-sdk` | Rust | Vi | EXISTS | EXTEND | Domain pack loading, recipe manifests, verification profiles. |
| `crates/observability` | Rust | Truong | EXISTS | EXTEND | Tracing subscriber setup, span exporters, structured telemetry. |
| `crates/authority-engine` | Rust | Truong | EXISTS | EXTEND | Zero-trust policy evaluation, risk classification, approval requests, audit hash-chain. |
| `apps/custosd` | Rust | Truong | EXISTS (Scaffold) | IMPLEMENT | Background runtime daemon supervisor. Scheduled for bootstrapping in PR-02b. |
| `apps/custos-cli` | Rust | Truong | EXISTS | EXTEND | CLI client interface with clap and terminal UI components. |
| `apps/custos-vscode` | TypeScript | Truong | EXISTS (Stub) | DEFER | VS Code extension client. Scheduled for PR-10a. |
| `adapters/providers/fake` | Rust | Vi | EXISTS | KEEP | Mock provider implementation for deterministic CI testing without API keys. |
| `adapters/providers/claude` | Rust | Vi | EXISTS (Scaffold) | IMPLEMENT | External Claude provider adapter. Scheduled for PR-03. |
| `adapters/providers/codex` | Rust | Vi | EXISTS (Scaffold) | IMPLEMENT | External OpenAI/Codex provider adapter. Scheduled for PR-03. |
| `adapters/providers/antigravity` | Rust | Vi | EXISTS (Scaffold) | IMPLEMENT | Antigravity engine adapter. Scheduled for PR-03. |
| `adapters/providers/local-model` | Rust | Vi | EXISTS (Scaffold) | IMPLEMENT | Local model provider. Prioritizes Ollama HTTP adapter before in-process inference. |
| `adapters/judgments/rules` | Rust | Vi | EXISTS | EXTEND | Rule-based heuristics for fast System 1 decisions. |
| `adapters/judgments/onnx` | Rust | Vi | EXISTS (Scaffold) | DEFER | Local ONNX runtime judgment engine. Gated after benchmark. |
| `adapters/judgments/jev` | Rust | Vi | EXISTS (Scaffold) | DEFER | Judgment Evaluation Vector adapter. Shadow mode only. |
| `adapters/sandboxes/macos-seatbelt` | Rust | Truong | EXISTS (Scaffold) | IMPLEMENT | macOS sandbox-exec Seatbelt profile generator and execution wrapper. |
| `adapters/sandboxes/linux-bubblewrap` | Rust | Truong | EXISTS (Scaffold) | IMPLEMENT | Linux bubblewrap (bwrap) namespace sandbox wrapper. |
| `adapters/tools` | Rust | Truong | EXISTS (Scaffold) | CONSOLIDATE | Consolidated tool implementations (FS, Shell, Git). Currently empty structs. |
| `tests/contract` | Rust | Truong + Vi | EXISTS | EXTEND | Provider conformance and schema validation integration tests. |
| `tests/e2e` | Rust | All | EXISTS | EXTEND | End-to-end integration tests, including vertical slice `repo_explain_slice.rs`. |
| `xtask` | Rust | Truong | EXISTS | KEEP | Developer automation tooling and cargo xtask workflows. |

---

## 4. Scaffold and Missing Implementation Registry

The following files exist as structural scaffolds requiring concrete implementations during the rollout phases:

| File Path | Current State | Required Implementation | Target PR |
|---|---|---|---|
| `crates/cognitive-runtime/src/rdc.rs` | Enum definition only (`RdcPhase`) | Concrete Resolve-Delegate-Check deliberation loop | PR-01 / PR-03 |
| `adapters/tools/src/lib.rs` | Empty struct declarations | File system reading/writing, git operations, shell execution behind permits | PR-04 / PR-05 |
| `crates/local-api/src/lib.rs` | Stub routing | Axum HTTP server with task creation, polling, streaming events, and cancel | PR-02b |
| `apps/custosd/src/main.rs` | Minimal entrypoint | Daemon bootstrap, single SQLite writer, graceful shutdown signal handling | PR-02b |
| `adapters/providers/claude/src/lib.rs` | Skeleton traits | HTTP client adapter implementing `ModelPort` for Anthropic Messages API | PR-03 |
| `adapters/providers/local-model/src/lib.rs` | Skeleton traits | Ollama HTTP endpoint adapter implementing `ModelPort` | PR-03 |
| `crates/persistence-sqlite/migrations/0004_usage.sql` | Missing | Schema for token reservations, settled costs, attempt accounting | PR-02 |
| `crates/persistence-sqlite/migrations/0005_evidence.sql` | Missing | Schema for evidence bundles, cryptographic hashes, verification receipts | PR-02 |
| `third_party/GOOSE_SOURCE.md` | Missing | Pinned upstream SHA documentation and integration boundaries | PR-G01 |

---

## 5. Verification Sign-Off

- Baseline SHA `acc6dbd2a1d7648227b26c4327200008d11951f7` recorded.
- Working tree uncommitted files verified intact.
- 46 test suites passing in workspace.
- Checkpoint C0 inventory verified and documented.
