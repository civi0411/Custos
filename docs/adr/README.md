# Architecture Decision Records (ADRs)

> **Status review required (2026-09-27):** The “Accepted” labels below are historical records. Before using either ADR to approve a shared-contract PR, verify the recorded maintainers, version, current type/schema compatibility and conformance fixtures. New contract proposals are in the [contract register](../contracts/README.md); current ownership is defined by `AGENTS.md` and shared contracts still require cross-owner review.

> **Status:** Historical ADR index; current acceptance requires recorded review
> **Standard:** Semantic Slug ADR Format (`<topic-slug>.md`)

The Custos decision repository adheres to the [Architectural Decision Records (ADR)](https://adr.github.io/) standard. Every consequential architectural choice, type contract alignment, and security boundary is captured in a dedicated, semantically named record.

The [document authority ADR](document-authority-and-ownership.md) and [Goose compatibility boundary](goose-compatibility-boundary.md) are lead-directed decisions with team/implementation review still pending.

---

## 1. Active Architecture Decision Records

| Decision Record | Scope / Domain | Status | Key Impact |
|---|---|:---:|---|
| **[Task State Machine & Status Taxonomy Alignment](task-state-machine-alignment.md)** | Core / Task | **Retained; conformance review required** | Formalizes `Draft`, `Queued`, `Running`, `Blocked`, `Succeeded`, `Failed`, `Cancelled` lifecycle and aliases |
| **[ActionIntent, ExecutionPermit, and ExecutionReceipt Contracts](action-intent-and-permit-contracts.md)** | Security / Authority | **Historical acceptance; review required** | Records naming intent; C-03 and current trust-closure fixtures govern new work |
| **[Goose Compatibility Boundary](goose-compatibility-boundary.md)** | Repository / Interop | **Lead-directed** | Separates Custos product identity from exact upstream and legacy wire/data names |

---

## 2. Canonical Architectural Decisions by Domain

These core principles govern the implementation across all gates (established in the System Blueprint and `AGENTS.md`):

### Core Architecture & State
- **Task as Core Durable Unit:** The primary entity of work is a Task with state, budgets, workflow, and evidence (not ephemeral chat sessions).
- **Local Daemon & Trust Boundary:** The trusted computing base runs locally (`custosd`); remote AI models and tools are untrusted external actors.
- **Custos-Owned Semantic Core:** Rust implements the immutable kernel, type contracts, and invariant reducers; no external framework dictates domain contracts.

### Storage & Persistence
- **SQLite WAL Event & State Store:** Single-writer multi-reader persistence with transactional task transitions and idempotent schema migrations.
- **Minimal Workflow Execution:** Lightweight, deterministic task and step persistence without heavy external workflow engines for MVP.
- **Derived Indexes are Non-Canonical:** Vector, graph, and AST symbol indexes can be rebuilt from source files at any time.
- **Content-Addressed Storage (CAS):** Large tool outputs, file snapshots, and artifacts are addressed by SHA-256 digest.

### Security, Authority & Sandboxing
- **Capability Gateway Enforcement:** Every side effect requires an active, unexpired `ExecutionPermit` minted by the `AuthorityEngine`.
- **Exact-Payload Binding:** Permits are cryptographically bound to action parameter hashes; no wildcard tool execution.
- **Confidence is Not Authority:** Model self-confidence or verbal assertions never bypass policy gates or evidence verification.
- **Tiered Sandboxing:** Execution containment isolated via Git worktrees, macOS Seatbelt profiles, and Linux Bubblewrap (`bwrap`).

### Cognitive Runtime & Model Interop
- **Cognitive Control Fabric (System 1 / System 2):** Fast deterministic rules / local classifiers handle screening; expensive frontier models handle deliberation.
- **Provider Port Abstraction:** AI model providers implement `ModelProvider` trait; no direct vendor SDK dependencies in core crates.
- **Request-Decision-Challenge (RDC):** Cognitive decisions are subject to reflexive validation before actions are proposed.
- **Pluggable System One:** Fast judgment is an extensible capability (rules, ONNX, Jev), not locked to any single vendor.

### Verification & Workflows
- **Evidence-Based Completion:** Tasks only complete when objective evidence (test passes, build receipts, symbol anchors) satisfies contract requirements.
- **Ephemeral Workers & Domain Packs:** Domain packs define declarative workflows; workers are spawned ephemerally and bounded by leases.
- **ContextPack Budgeting & Provenance:** Context compilation strictly enforces token budgets and calculates SHA-256 provenance hashes.

---

## 3. Creating a New ADR

New ADRs are created in `docs/adr/<descriptive-slug>.md` using the following template:

```markdown
# ADR: [Descriptive Decision Title]

## Context & Problem Statement
Describe the technical context, motivations, and requirements necessitating this architectural choice.

## Decision Outcome
The specific accepted decision and architectural approach adopted.

## Considered Options
- Option A: [Pros / Cons]
- Option B: [Pros / Cons]

## Consequences
- Positive: Expected benefits and architectural advantages.
- Negative / Risks: Technical trade-offs, added complexity, or operational constraints.

## Verification & References
Test suites, benchmarks, code paths, or PRs validating the implementation of this decision.
```
