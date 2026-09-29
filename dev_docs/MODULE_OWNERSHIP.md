# Custos — Module Ownership & Allocation Matrix

> **Document ID:** DEV-OWNER-01  
> **Status:** Active Operational Ownership Matrix  
> **Detailed Blueprint:** See [`TEAM_WORK_ALLOCATION.md`](TEAM_WORK_ALLOCATION.md) for full crate breakdown, sequence diagrams, and workload ratio.  
> **Repository Rules:** Governed by [`AGENTS.md`](../AGENTS.md) and [`docs/architecture/reference-architecture.md`](../docs/architecture/reference-architecture.md).

---

## 1. Primary Ownership Matrix

The workspace allocates ownership across the three team members according to the **Core Trunk & Pluggable Branches** architecture:

| Area | Primary Owner | Required Collaboration | Scope & Responsibilities |
|---|:---:|---|---|
| **Core Trunk & Domain Authority** | **Vi** | Truong (Persistence) + Vinh (Workflow) | `custos-domain`, `custos-kernel`, `custos-local-api`, `custos-security` (Zero-I/O domain, StateMachine, CQRS, contracts C-01..C-04) |
| **Daemon Composition Root** | **Vi** | Truong + Vinh | `custos-daemon` (Sole production composition root; wires storage, workflows, cognitive, and security) |
| **AI, Cognitive & Context Platform** | **Vi** | Truong (Privacy) + Vinh (Runtime handoff) | `custos-cognitive`, `custos-context`, `custos-context-management`, `custos-memory-service`, `providers/**`, `judgments/**`, `packs/**`, `tools/repo_intelligent`, `evals/` |
| **Durable Storage & OS Sandboxing** | **Truong** | Vi (Domain entities/evidence) | `custos-persistence` (SQLite WAL mode, schema migrations, event store, outbox), `sandboxes/**` (Linux bubblewrap, macOS seatbelt), `custos-download-manager` |
| **Workflow Engine & Protocols** | **Vinh** | Vi (Cognitive steps) + Truong (Persistence) | `custos-workflow`, `custos-session`, `custos-agent`, `custos-bridge`, `custos-mcp`, `custos-adapters-mcp` |
| **User Experience, CLI & Clients** | **Vinh** | Vi (UX flow) + Truong (API IPC) | `custos-cli` (Terminal interactive vibe, ratatui, clap), `ui/**`, `packages/**` |
| **Experimental Gateway** | **Vi** *(Caretaker)* | Truong + Vinh | `custos-gateway` (Frozen test mock; non-normative until ADR redesign) |

---

## 2. Core Invariants & Governance

1. **Domain & Kernel Authority:** The `custos-domain` and `custos-kernel` packages are owned directly by **Vi** as Chief Architect. Shared domain types, state transitions, public API DTOs, and evidence criteria are modified only with Vi's approval.
2. **Daemon Integrity:** `crates/app/custos-daemon` is the single composition root. Only Vi merges integration code that wires new adapters into the production daemon.
3. **Storage Engine Independence:** Truong implements the `DatabaseStore` traits defined by Vi. `custos-persistence` must satisfy SQLite WAL mode, crash recovery, and migration invariance without leaking database types into core domain.
4. **Workflow & Client Decoupling:** Vinh implements the `WorkflowEngine` and `ClientTransport` traits. CLI and MCP adapters interact with the daemon strictly through versioned `LocalApiClient` IPC calls.
5. **Cross-Boundary Changes:** Modifying shared contracts (C-01 through C-04 in [`docs/contracts/README.md`](../docs/contracts/README.md)) requires review and approval from all three maintainers.
