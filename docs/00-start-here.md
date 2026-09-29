# Custos Documentation — Start Here

> **Document ID:** DOC-START-01  
> **Classification:** Authoritative Onboarding Guide & Information Architecture  
> **Target Audience:** Systems Engineers, AI Engineers, Security Architects, UI/Client Developers, and Open-Source Contributors  
> **Normative Architecture:** [`ARCH-REF-01`](architecture/reference-architecture.md) & [`ARCH-FLOW-01`](architecture/runtime-flows.md)  
> **Source Truth:** Verified against the 42-crate Cargo workspace and test suite (2026-09-28)

Welcome to the technical documentation for **Custos** — a human-governed, local-first runtime for specialized agentic work.

Custos combines principles from **arc42**, the **C4 model**, and **ADR (Architectural Decision Records)** to provide an uncompromising, verifiable engineering baseline.

---

## 1. Role-Based Reading Pathways

Select your onboarding pathway based on your technical discipline and responsibilities:

### 🦀 Systems & Core Runtime Engineers
*Focus: Kernel CQRS, State Machine, SQLite persistence, process lifecycle, capability sandbox.*
1. **Master System Design:** Read [`Reference Architecture`](architecture/reference-architecture.md) and [`Runtime Flows`](architecture/runtime-flows.md).
2. **Task State & Invariants:** Read [`Task Lifecycle`](architecture/task-lifecycle.md) and inspect [`crates/core/custos-kernel/src/state_machine.rs`](../crates/core/custos-kernel/src/state_machine.rs).
3. **Execution Sandbox & Permits:** Study [`Capability Gateway`](architecture/capability-gateway.md) and [`crates/runtime/custos-security`](../crates/runtime/custos-security).
4. **Durable Persistence:** Review [`Persistence Architecture`](architecture/persistence.md) and [`crates/infrastructure/custos-persistence`](../crates/infrastructure/custos-persistence).
5. **Crash Recovery & Idempotence:** Study [`Crash Recovery`](architecture/crash-recovery.md).
6. **Workspace Structure:** Read [`Repository Structure`](development/repository-structure.md) for the 42-crate dependency boundary.

### 🧠 AI Engineers & Cognitive Architects
*Focus: Multi-tier cognitive routing (System 0/1/2), ContextPacks, ModelPort, memory indexing.*
1. **Cognitive Routing Architecture:** Study [`Cognitive Fabric`](architecture/cognitive-fabric.md) and [`System 1 & System 2 Architecture`](architecture/COGNITIVE_ARCHITECTURE_S1_S2.md).
2. **Context Compilation:** Review [`Context & Memory Architecture`](architecture/context-memory.md) and [`crates/runtime/custos-context`](../crates/runtime/custos-context).
3. **Model & Provider Adapters:** Inspect [`Provider Interoperability`](architecture/provider-interop.md) and [`Connectivity Hubs`](architecture/connectivity-hubs.md).
4. **Domain Workflows:** Explore [`Engineering Domain`](domains/engineering.md), [`Research Domain`](domains/research.md), and [`Personal Domain`](domains/personal.md).

### 🛡️ Security & Platform Engineers
*Focus: Threat models, PathSandbox containment, cryptographic evidence, secret isolation.*
1. **Threat Model:** Study the STRIDE analysis in [`Threat Model`](security/threat-model.md).
2. **Capability & Approval Model:** Review [`Capability Model`](security/capability-model.md).
3. **Deterministic Gate & Verification:** Inspect [`Evidence Verification`](architecture/evidence-verification.md) and test proof in [`tests/e2e/tests/controlled_effects_proof_closure.rs`](../tests/e2e/tests/controlled_effects_proof_closure.rs).
4. **Data Privacy & Egress:** Review [`Privacy & Egress Policy`](security/privacy.md).

### 💻 Client & Application Developers (CLI, UI, Desktop)
*Focus: Experience Plane, LocalApiClient, IPC ProcessTransport, JSON-RPC, approval UX.*
1. **Client Contracts:** Review the cross-team contracts in [`Contract Register`](contracts/README.md) (specifically C-01 Local API and C-02 Task lifecycle).
2. **Daemon-CLI Protocol:** Study the implementation in [`crates/app/custos-local-api`](../crates/app/custos-local-api).
3. **CLI Reference & Modes:** Review [`crates/app/custos-cli`](../crates/app/custos-cli) supporting `vibe`, `explain`, `create`, and task inspection.
4. **Implementation Blueprint:** Track deliverable PRs in [`Implementation Blueprint`](development/implementation-blueprint.md).

### 📋 Product Managers, QA & Governance
*Focus: Product vision, invariants, empirical status, roadmap.*
1. **Product Thesis:** Read [`Product Identity`](product/identity.md) and [`Product Scope`](product/scope.md).
2. **Empirical Status:** Check [`Current Audit Baseline`](status/local-dev-audit-2026-09-28.md) and [`Status Rules`](status/README.md).
3. **Architectural Gap Matrix:** Review [`Architecture Gap Matrix`](status/architecture-gap-matrix.md).
4. **Quality Gates:** Verify requirements in [`PR-00 Gates`](development/pr-00-gates.md).

---

## 2. Canonical Documentation Directory

```text
docs/
├── README.md                          # DOC-HUB-01: Authority map, reading paths, status taxonomy
├── 00-start-here.md                   # This guide: Role-based pathways & quickstart
├── canonical-specification.md         # Master product vision and canonical design baseline
├── document-register.md               # Complete register of all docs and their normative status
│
├── architecture/                      # System Architecture & Technical Specifications
│   ├── reference-architecture.md      # ARCH-REF-01: Full system design, planes, and boundaries
│   ├── runtime-flows.md               # ARCH-FLOW-01: Intake, F1/F2/F3, effect, evidence, recovery
│   ├── target-architecture.md         # ARCH-TARGET-01: North-star end-state architecture
│   ├── task-lifecycle.md              # Domain model, Task State Machine, CQRS, epochs
│   ├── cognitive-fabric.md            # System 0/1/2 routing, CognitiveArbiter, RDC protocol
│   ├── capability-gateway.md          # ExecutionPermits, DeterministicGate, PathSandbox
│   ├── evidence-verification.md       # Cryptographic evidence pipeline, verifiers, closure
│   ├── context-memory.md              # Token-budgeted ContextPacks, memory indexing
│   ├── connectivity-hubs.md           # ModelPort, external coding agents, MCP, 9Router
│   ├── persistence.md                 # SQLite WAL, event store, CAS outbox, schema migrations
│   ├── crash-recovery.md              # Resumption, reconciliation loops, idempotent effects
│   ├── deployment.md                  # Local daemon, OS-level sandboxes, background runners
│   ├── communication.md               # IPC envelopes, star topology, local process transport
│   └── provider-interop.md            # Provider traits, ContinuationPacket, model handoffs
│
├── contracts/                         # Cross-Team Interoperability Contracts
│   └── README.md                      # Index of normative C-01, C-02, C-03, C-04 contracts
│
├── product/                           # Product Vision & Capabilities
│   ├── identity.md                    # Core problem, guardian philosophy, non-goals
│   ├── scope.md                       # Release horizons (H1/H2/H3), MVP Definition of Done
│   └── capability-map.md              # Capability taxonomy and priority matrix
│
├── domains/                           # Specialized Domain Packs
│   ├── engineering.md                 # Coding workflows, AST ranking, test-backed diffs
│   ├── research.md                    # Claim extraction, source citations, hypothesis matrix
│   ├── personal.md                    # Assistant workflows, privacy ladders, task coordination
│   └── AGENT_PATHOLOGIES_AND_SOLUTIONS.md # Agent failure modes and architectural mitigations
│
├── development/                       # Engineering Guidelines & Blueprints
│   ├── repository-structure.md        # Verified 42-crate Cargo layout & crate roles
│   ├── implementation-blueprint.md    # Phased delivery blueprint & PR sequencing
│   ├── pr-00-gates.md                 # Conformance checklist for landing changes
│   ├── naming-conventions.md          # Canonical naming policy (Custos vs Goose)
│   ├── testing.md                     # Test pyramid, unit/contract/e2e testing
│   ├── observability.md               # Structured logging, metrics, local traces
│   └── oss-adoption.md                # Open-source adoption criteria and scorecards
│
├── security/                          # Security & Trust Boundaries
│   ├── threat-model.md                # STRIDE analysis, prompt injection, compromised tools
│   ├── capability-model.md            # Capability grants, exact-payload human approvals
│   └── privacy.md                     # Zero telemetry egress, local secret storage
│
├── status/                            # Empirical Audit & Reality Tracking
│   ├── README.md                      # Status claim vocabulary (Designed/Implemented/Wired/Verified)
│   ├── local-dev-audit-2026-09-28.md  # Current verified snapshot and proof closure
│   ├── architecture-gap-matrix.md     # Implementation vs target architectural gaps
│   └── goose-naming-migration.md      # Upstream naming and compatibility register
│
├── reference/                         # Concepts, Glossaries & Comparisons
│   ├── concepts.md                    # Core concepts glossary
│   ├── invariants.md                  # Irreversible design decisions and runtime invariants
│   ├── naming.md                      # Canonical naming dictionary
│   ├── comparisons.md                 # Architectural comparisons with existing frameworks
│   ├── schema-mapping.md              # Database and JSON schema mappings
│   └── sources.md                     # Research references and citations
│
├── research/                          # Research Findings & Explorations
│   ├── README.md                      # Research index and vendor boundary rules
│   └── connectivity-gateway-findings.md # Analysis of gateway topologies
│
├── adr/                               # Architecture Decision Records
│   ├── README.md                      # ADR index, decision status, and template
│   ├── task-state-machine-alignment.md # Alignment of Task lifecycle states
│   ├── action-intent-and-permit-contracts.md # Strict ActionIntent / ExecutionPermit separation
│   ├── document-authority-and-ownership.md   # Authority rules for repo documentation
│   └── goose-compatibility-boundary.md      # Upstream compatibility boundaries
│
├── archive/                           # Historical Proposals & Legacy Archives
│   └── README.md                      # Disposition notes for archived materials
│
└── i18n/                              # Multi-Language Translations
    ├── README.md                      # i18n directory guide
    ├── README.vi.md                   # Vietnamese translation
    ├── README.de.md                   # German translation
    └── README.zh.md                   # Chinese translation
```

---

## 3. Status Claims & Verification Vocabulary

To maintain strict truthfulness across all documentation, authors must use the following standard terms:

| Label | Definition | Verification Requirement |
|---|---|---|
| **Designed** | Formal specification, contract, or schema exists. | Reviewed markdown or JSON schema file. |
| **Implemented** | Source code exists in one or more workspace crates. | Code compiles under `cargo check --workspace`. |
| **Wired** | An executable entrypoint (`custos-daemon` or `custos-cli`) connects the component. | Production composition root invokes the code. |
| **Verified** | Behavior is proven end-to-end under real conditions, including failure paths. | Passing integration/E2E test in `tests/e2e/`. |
| **Degraded / Experimental** | Implemented or wired, but known failure modes exist or tests fail. | Recorded in `docs/status/architecture-gap-matrix.md`. |

> [!IMPORTANT]
> A passing `cargo check` proves **Implemented**, never **Wired** or **Verified**. Always cite test proof when claiming verification.

---

## 4. Fundamental Runtime Invariants

Every engineer working on Custos must uphold these non-negotiable invariants:

1. **Zero-I/O Domain Core:** `crates/core/custos-domain` must remain completely free of network, filesystem, or async runtime dependencies.
2. **Durable State Authority:** Task state transitions are strictly owned by `crates/core/custos-kernel`. Neither LLMs nor client interfaces can bypass the state machine.
3. **Side-Effect Containment:** All mutations (filesystem, shell, network, external APIs) must flow through `crates/runtime/custos-security` with an issued `ExecutionPermit`.
4. **Evidence-Backed Completion:** No task may transition to `Succeeded` without verified artifacts satisfying the task's completion gate.
5. **Local-First Sovereignty:** All task ledgers, audit journals, and credentials reside on the user's host machine. Remote egress occurs only under explicit user policy.
