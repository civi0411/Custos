# CUSTOS ARCHITECTURE INDEX — 10 THEMATIC PILLARS

> **Classification:** Normative Architecture Blueprint Suite  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Repository Root Index:** See [Custos Documentation Overview](../README.md).  
> **Physical Codebase Map:** See [Monorepo Deep File Catalog](../development/codebase-architecture.md).

The `docs/architecture/` suite contains 40 authoritative technical specifications defining the architecture of the Custos Supervised Agent Development Environment (SADE). Every subsystem, invariant, mathematical formulation, and operational protocol is organized strictly across **10 Master Architectural Themes**.

---

## Thematic Architecture Navigation Matrix

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                          THE 10 MASTER ARCHITECTURAL THEMES                             │
├──────────────────────────────┬──────────────────────────────┬───────────────────────────┤
│ Theme 1: SADE & Agent Lake   │ Theme 2: Kernel & State FSMs │ Theme 3: Authority Gateway│
│ Theme 4: Governed Agent Loop │ Theme 5: Evidence & Gate     │ Theme 6: Measured OI      │
│ Theme 7: Storage & Partition │ Theme 8: Compute Relay & GPU │ Theme 9: 4 Workbenches    │
│ Theme 10: Protocols & UI     │                              │                           │
└──────────────────────────────┴──────────────────────────────┴───────────────────────────┘
```

---

### Theme 1: SADE Product Vision, Agent Lake Ecosystem & Multi-Lens Paradigm

This theme establishes Custos not as a toy prompt wrapper or naive multi-agent framework, but as a formal **Supervised Agent Development Environment (SADE)** with an interconnected "Agent Lake" (Living Commons) where diverse agents collaborate via Unified Resource References (URR).

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`sade-agent-system-design.md`](sade-agent-system-design.md)** | Definitive system contract for SADE: defines the 5 distinct meanings of "agent", modular workbench boundaries, and cross-lens continuation without execution leak. | Lens switching never grants execution permits; Tasks survive across lenses. |
| **[`sade-design-and-supervision.md`](sade-design-and-supervision.md)** | Trilateral coordination protocol among S1 Fast Intuitive heuristics, S2 Deliberative Planning, and sovereign Human supervision. | S1 provides telemetry; S2 proposes plans; Human/Kernel grants permits. |
| **[`sade-foundations.md`](sade-foundations.md)** | Extraction of a generic supervised execution kernel from OrCa; 7 execution primitives $\times$ 4 domain workbenches. | Generic kernel isolation; zero domain math inside platform core. |
| **[`sade-agent-lake-ecosystem.md`](sade-agent-lake-ecosystem.md)** | Architecture of the "Agent Lake" (Living Commons): shared event bus, URR resource descriptors, and symbiotic multi-agent life cycle. | All cross-agent artifacts carry cryptographically verifiable URR digests. |
| **[`sade-platform-deep-research-and-refactor.md`](sade-platform-deep-research-and-refactor.md)** | Comprehensive audit of 538 platform files, Hexagonal/Ports-and-Adapters structure, and typed dependency direction. | Unidirectional dependency graph: domain $\rightarrow$ core $\rightarrow$ runtime $\rightarrow$ daemon. |
| **[`workbench-lenses.md`](workbench-lenses.md)** | Presentation separation of 4 specialist lenses (Copilot, Engineering, Research, Quant) over shared resource anchors. | Screens are view projections; restoring a view never dispatches side-effects. |

---

### Theme 2: Task Kernel, Finite State Machines & Durable Continuity

The Task is the primary, first-class citizen in Custos. While sessions, models, and network connections are ephemeral, the Task persists indefinitely with cryptographic state auditing and crash recovery.

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`kernel-and-task-lifecycle.md`](kernel-and-task-lifecycle.md)** | Task state machine, deterministic state transitions (`Draft` $\rightarrow$ `Admitted` $\rightarrow$ `Active` $\rightarrow$ `Verifying` $\rightarrow$ `Completed`), revisions, and ContinuationPackets. | Transitions mandate explicit `ActorId` and valid cryptographic signatures. |
| **[`run-lifecycle.md`](run-lifecycle.md)** | Execution lifecycle of `Run` and `WorkerRun`, optimistic concurrency via `state_version` (CAS), and handling of the `Uncertain` status. | Connection drop transitions attempt to `Uncertain`; blind retries are blocked. |
| **[`domain-ownership.md`](domain-ownership.md)** | Domain type ownership ledger, enforcing Zero-Kernel Leakage and strict Ring 0 purity in `platform/domain`. | `custos-domain` has 0 I/O dependencies (no async, no tokio, no sql, no fs). |
| **[`workspace-binding-rules.md`](workspace-binding-rules.md)** | Rules for binding tasks to workspace directories, Git checkouts, and isolated worktrees. | Multi-agent concurrency operates strictly in isolated Git worktrees. |

---

### Theme 3: Authority Engine, Security Defense & Capability Gateway

Models are strictly untrusted or conditionally trusted generators of intent. All physical mutations mandate two-phase authorization, taint tracking, and cryptographic permits.

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`authority-and-capability-gateway.md`](authority-and-capability-gateway.md)** | Formal authority model: `Scope`, `Grant`, `ActionIntent`, `ExecutionPermit`, and two-phase mediation (`authorize()` $\rightarrow$ `execute()`). | Every physical side-effect requires a single-use signed `ExecutionPermit` (INV-03). |
| **[`security-and-threat-defense.md`](security-and-threat-defense.md)** | Comprehensive threat modeling: anti-Confused-Deputy defense, Taint tracking (`Tainted<T>`), and OS sandboxing (Seatbelt / Bubblewrap). | Untrusted external data cannot be promoted to operational policy (INV-04). |
| **[`capability-catalog-and-skills.md`](capability-catalog-and-skills.md)** | Typed capability manifests, dynamic skill discovery, and mediation guarantees across tool adapters. | Tools register declarative permission requirements before invocation. |

---

### Theme 4: Governed Agent Loop, CLI Agent Supervision & Process Lifecycle

Governs local execution of autonomous agent loops, eliminating runaway API costs, memory leaks, and orphaned background zombie processes.

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`governed-agent-loop.md`](governed-agent-loop.md)** | Multi-turn agent loop with two-phase mediation, `SupervisedAgentRunner` decorator, `LimitedWriter` streaming output caps, and `ProcessDropGuard` (kill `-pid` with SIGKILL). | Prevents duplicate physical dispatch; kills all orphaned zombie subprocesses on drop. |
| **[`agent-environment.md`](agent-environment.md)** | Agent Execution Environment (AEE) specification: honest assurance declarations (`custos-mediated`, `provider-governed`, `observe-only`) and 7 gates `ENV-01..ENV-07`. | Execution assurance cannot be spoofed or escalated without verification. |
| **[`notebook-kernel-binding.md`](notebook-kernel-binding.md)** | Scoped Jupyter Python kernel execution, optimistic `cell_version` concurrency, output invalidation on code mutation, and composite session keying. | Mutating cell code immediately invalidates downstream execution outputs. |

---

### Theme 5: Evidence Engine, Completion Gate & Verifier Isolation

Completion without independent, verifiable evidence is a hallucination. Custos replaces self-reported model claims with an unbypassable Completion Gate.

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`evidence-and-completion-gate.md`](evidence-and-completion-gate.md)** | Completion Gate architecture: Fact $\rightarrow$ Extraction $\rightarrow$ Semantic evidence hierarchy, immutable CAS digests, and verifier isolation. | Task criterion transitions to `Pass` only with verified `EvidenceRecord` (INV-05). |
| **[`three-vertical-acceptance.md`](three-vertical-acceptance.md)** | Cross-pack verification framework across Coding, Research, and Assistant verticals. | Shared evidence protocols; specialized domain verifiers. |
| **[`supervised-experiment-execution-and-recovery.md`](supervised-experiment-execution-and-recovery.md)** | Supervision of scientific and machine learning experiments, milestone checkpoints, and honest assessment. | Failed checkpoints record explicit failure causes; speculative completion denied. |

---

### Theme 6: Measured Orchestration Intelligence (OI) & Economic Optimization

Rejects theoretical multi-agent claims by establishing an honest, measured optimization substrate anchored by Golden Acceptance Test `G12`.

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`measured-orchestration-intelligence.md`](measured-orchestration-intelligence.md)** | Pareto Multi-Objective Selector ($U = w_q Q - w_c C - w_t T$), Real Model Pricing Catalog, Prompt Cache Affinity, 7 Topologies (T0–T6, featuring T5 Lead-Worker DAG coordinator), and 0-Token FastPathFilter (< 1ms, \$0.00). | Pinned single model is first-class baseline; multi-agent admitted only with measured gain. |
| **[`cognitive-fabric-and-orchestration.md`](cognitive-fabric-and-orchestration.md)** | Integration of S1 fast heuristics, S2 deliberative replanning, and graph delta reachability with settled node preservation. | Delta replanning carries committed obligations; uncertain effects block blind retry. |

---

### Theme 7: Storage Architecture, Relational SQLite WAL & Partition Engine

Provides durable local persistence with zero WAL starvation, transactional outbox guarantees, and scalable multi-database partitioning.

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`command-ledger.md`](command-ledger.md)** | Append-only immutable command ledger, reconciliation protocol, and idempotency tracking. | Historical records are append-only; human corrections use compensating transactions. |
| **[`data-context-and-memory.md`](data-context-and-memory.md)** | 3-tier memory model (Working, Episodic, Semantic), token budget compaction, and single-writer SQLite WAL connection. | Single-writer connection prevents WAL corruption; CAS store is content-addressed. |

---

### Theme 8: Heterogeneous Compute, Accelerator Relay & Checkpoint Continuity

Enables seamless execution across local Apple Silicon (Metal/llama.cpp) and remote high-performance cloud GPUs without VRAM starvation or checkpoint loss.

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`compute-relay-and-heterogeneous-failover.md`](compute-relay-and-heterogeneous-failover.md)** | Compute relay architecture, Unsloth training script portability, and heterogeneous failover across local/cloud targets. | Fencing of old executors on handoff; target machine re-validates credentials. |
| **[`gpu-training-and-inference-coexistence.md`](gpu-training-and-inference-coexistence.md)** | Lazy `LlamaCppBackend` initialization, dynamic VRAM reservation, and memory-safe coexistence between inference and training. | Zero GPU allocation on daemon startup; models load strictly on demand. |
| **[`gpu-workload-admission-and-coordination.md`](gpu-workload-admission-and-coordination.md)** | GPU admission gate: verifies hardware compatibility, VRAM bounds, and accelerator specs before launching jobs. | Unmet accelerator requirements fail-closed with `DomainError::Validation`. |
| **[`remote-compute-colab-kaggle.md`](remote-compute-colab-kaggle.md)** | Remote compute adapters for ephemeral environments (Google Colab, Kaggle Kernels). | Ephemeral instances must sync snapshots to durable storage before shutdown. |
| **[`remote-compute-external-gpu.md`](remote-compute-external-gpu.md)** | Rented GPU fleet lifecycle management via SSH/tunneling with heartbeat monitoring. | Heartbeat loss marks execution status `Uncertain`; prevents phantom success claims. |
| **[`training-studio-checkpoint-continuity.md`](training-studio-checkpoint-continuity.md)** | 4-level resumability checkpoints, bounded SHA-256 integrity verification, and resume eligibility assessment. | Checkpoint resume requires valid parent attempt digest and artifact isolation. |

---

### Theme 9: Vertical Specialist Workbenches (Assistant, Coding, Research, Quant)

Encapsulates deep domain specialization into 4 isolated workbenches without polluting platform kernel abstractions.

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`domain-packs-and-workflows.md`](domain-packs-and-workflows.md)** | Master specification of the 4 vertical workbenches: roles, jobs, schemas, and verifier obligations. | Workbenches declare explicit capability dependencies; no cross-workbench coupling. |
| **[`research-workbench-blueprint.md`](research-workbench-blueprint.md)** | Research workbench blueprint: claims, passage anchors, 4-step ALCE verification pipeline, and review handoff saga rollback. | Citation does not equal support; failing review batch rolls back all created tasks. |
| **[`quant-workbench.md`](quant-workbench.md)** | Quantitative trading workbench: alpha discovery, backtest runner, risk supervisor, and broker execution plans. | 100% Zero-Mock for real market data; financial math isolated inside workbench. |
| **[`quant-domain-models.md`](quant-domain-models.md)** | Pure domain models for financial data: `MarketBar`, `OrderTicket`, `Tearsheet`, and trade logs. | Zero-I/O pure entities; deterministic decimal financial arithmetic. |
| **[`quant-pack.md`](quant-pack.md)** | Packaging and verifier implementation: `DeflatedSharpeVerifier` (family-wise error correction) and `LookaheadBiasChecker`. | Rejects strategies showing lookahead data leakage or deflated Sharpe ratios. |
| **[`on-demand-packs.md`](on-demand-packs.md)** | On-demand workbench installation, progressive capability activation, and package isolation. | Workbenches can be disabled without affecting core platform daemon operation. |

---

### Theme 10: Connectivity Hubs, Desktop UI & Operational Parity

Delivers the human interface and external world integration: high-performance Local API, Desktop React 19 UI, and 5 automated quality gates.

| Document | Core Responsibility & Question Solved | Key Enforced Invariants |
|---|---|---|
| **[`protocol-and-connectivity-hubs.md`](protocol-and-connectivity-hubs.md)** | Unified connectivity hub: Local API (HTTP/IPC), MCP Federation, ACP streaming, and A2A Agent Cards. | Protocol DTOs reside strictly in `sdk/protocol`; core platform is transport-neutral. |
| **[`provider-and-adapter-boundaries.md`](provider-and-adapter-boundaries.md)** | Architectural boundaries between Model Providers, Agent Runtime Ports, and External Capabilities. | Transports are audited with cost/transport metadata on every attempt (INV-09). |
| **[`desktop-core-interaction-map.md`](desktop-core-interaction-map.md)** | Desktop ↔ Core interaction blueprint: ScreenContext, AppContext, daemon client streaming, and error settlement. | Desktop UI maintains Zero-Mock in production; state reconciles with daemon snapshots. |
| **[`agent-workspace-and-ui.md`](agent-workspace-and-ui.md)** | Desktop component architecture: FilesChangesPane, ScreenSwitcherBar, ActivityRail, and 3-tier Run Inspector. | Presentation regions (work areas) are decoupled from background execution authority. |
| **[`upstream-parity-and-release-operations.md`](upstream-parity-and-release-operations.md)** | Release operations, upstream parity tracking against Goose/OrCa, and 5-gate automated release runner (`cargo xtask gate`). | 100% pass on all 5 release gates; zero warnings on `cargo clippy`; source ceiling $\le 600$ lines. |
