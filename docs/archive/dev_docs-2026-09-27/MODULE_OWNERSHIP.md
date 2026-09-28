# Custos Module Ownership & Review Matrix (GOV05)

> **Historical mirror:** Do not use this copy to assign current module ownership. The active [proposal](../../../dev_docs/MODULE_OWNERSHIP.md) still conflicts with `AGENTS.md` and requires a three-maintainer ADR.

> **Status:** Canonical Baseline v4.0 (Gate 0 / GOV05)  
> **Source:** Conforms to `AGENTS.md` Rule 1 & `dev_docs/README.md` Section 3

This document defines the formal ownership, primary reviewer, architectural boundary, and maturity rating for every crate, adapter, application, and governance surface across the Custos workspace.

---

## 1. Core Rust Crates

| Crate Path | Primary Owner | Mandatory Reviewer | Architectural Responsibility | Maturity | Gate Target |
|---|---|---|---|:---:|:---:|
| `crates/core-domain` | **Vi** | Truong + Vinh | Pure, immutable domain contracts, state machines, types. Zero I/O. | **F** | Gate 0 / 1 |
| `crates/task-kernel` | **Truong** | Vinh | CQRS commands, event reducers, optimistic concurrency, state persistence. | **F** | Gate 0 / 1 |
| `crates/persistence-sqlite` | **Truong** | Vi | SQLite WAL persistence, connection pool, idempotent migrations. | **F** | Gate 0 / 1 |
| `crates/authority-engine` | **Truong** | Vi | Policy evaluation, grants, ephemeral execution permits, SHA-256 audit log. | **F** | Gate 0 / 1 |
| `crates/capability-gateway` | **Truong** | Vinh | Deterministic execution gate, permit verification, sandbox dispatch. | **F** | Gate 1 |
| `crates/evidence-engine` | **Truong** | Vi | Hash, citation, diff, and command verifiers; pipeline evaluation. | **F** | Gate 0 / 1 |
| `crates/artifact-store` | **Truong** | Vi | Content-addressed storage (CAS) for large files and outputs. | **P** | Gate 3 |
| `crates/local-api` | **Truong** | Vinh | Process-local Unix socket / named pipe transport and IPC listener. | **S** | Gate 3 |
| `crates/context-compiler` | **Vi** | Truong | Relevance scoring, token-budgeted `ContextPack` assembly, recipes. | **F** | Gate 1 |
| `crates/repo-intelligence` | **Vi** | Truong | AST parsing, code graph, workspace scanner, symbol ranking. | **F** | Gate 1 / 2 |
| `crates/cognitive-runtime` | **Vi** | Vinh | System 1 / System 2 cognitive routing, RDC arbitration. | **S** | Gate 4 |
| `crates/judgment-contracts` | **Vi** | Truong | Request/Result wire types for fast heuristics and judges. | **S** | Gate 4 |
| `crates/deliberation-contracts` | **Vi** | Vinh | Long-horizon reasoning, planning graph, multi-step critique. | **S** | Gate 4 |
| `crates/memory-service` | **Vi** | Truong | Session-, task-, and workspace-scoped memory retrieval. | **S** | Gate 3 |
| `crates/domain-pack-sdk` | **Vi** | Vinh | Pack manifest loader, schema validation, recipe registry. | **S** | Gate 3 |
| `crates/provider-sdk` | **Vi** | Truong | `ModelProvider` trait, `CapabilityDescriptor`, event streams. | **P** | Gate 1 / 4 |
| `crates/workflow-runtime` | **Vinh** | Truong | Worker state machines, DAG scheduling, handoff envelopes, leases. | **S** | Gate 1 / 3 |
| `crates/observability` | **Vinh** | Truong | OpenTelemetry metrics, tracing propagation, correlation IDs. | **P** | Gate 1 / 3 |

*Maturity Legend: **F** = Functional with passing tests; **P** = Partial / in-progress; **S** = Skeleton / trait contract.*

---

## 2. Adapters (Providers, Judgments, Sandboxes)

| Adapter Path | Primary Owner | Reviewer | Type | Status | Gate Target |
|---|---|---|---|:---:|:---:|
| `adapters/providers/fake` | **Vi** | Truong | Deterministic mock provider with streaming | **F** | Gate 1 |
| `adapters/providers/claude` | **Vi** | Truong | Anthropic Claude adapter over JSON-RPC | **S** | Gate 4 |
| `adapters/providers/codex` | **Vi** | Truong | OpenAI Codex / GPT-4o adapter | **S** | Gate 4 |
| `adapters/providers/antigravity` | **Vi** | Truong | Antigravity model engine adapter | **S** | Gate 4 |
| `adapters/providers/local-model` | **Vi** | Truong | Ollama / llama.cpp OpenAI-compatible | **S** | Gate 4 |
| `adapters/judgments/rules` | **Vi** | Vinh | Deterministic rule-based fast judge | **S** | Gate 4 |
| `adapters/judgments/onnx` | **Vi** | Truong | Local ONNX embedding / classifier judge | **S** | Gate 4 |
| `adapters/judgments/jev` | **Vi** | Vinh | Jev cognitive judgment engine sidecar | **S** | Gate 4 |
| `adapters/sandboxes/macos-seatbelt` | **Truong** | Vinh | macOS Seatbelt sandbox profile executor | **S** | Gate 5 |
| `adapters/sandboxes/linux-bubblewrap` | **Truong** | Vinh | Linux Bubblewrap (`bwrap`) sandbox executor | **S** | Gate 5 |
| `adapters/tools` | **Truong** | Vinh | Standard tool implementations (fs, git, bash) | **S** | Gate 3 |

---

## 3. Applications & Surfaces

| Path | Primary Owner | Reviewer | Description | Status | Gate Target |
|---|---|---|---|:---:|:---:|
| `apps/custos-cli` | **Truong** | Vi | Command line interface (`custos explain`, CRUD) | **F** | Gate 1 |
| `apps/custosd` | **Truong** | Vinh | Long-running background daemon process | **P** | Gate 3 |
| `apps/custos-vscode` | **Vinh** | Vi | VS Code extension (UI tree view & webview) | **S** | Gate 5 |

---

## 4. Shared Governance Surfaces

Changes to these paths require **unanimous three-party consensus** (Vi, Truong, Vinh):
- `Cargo.toml` (root workspace dependencies)
- `AGENTS.md` (team operating protocol)
- `schemas/protocol/*.v1.schema.json` (wire contracts)
- `schemas/packs/*.v1.schema.json` (domain pack format)
- `crates/persistence-sqlite/migrations/0001-0003` (historical migrations are immutable)
- `docs/canonical-specification.md`
