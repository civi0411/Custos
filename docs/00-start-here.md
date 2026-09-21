# Custos Documentation — Start Here

> **Status:** Canonical Baseline v4.0-draft  
> **Last Updated:** 2026-09-21  
> **Scope:** Architecture, Product, Implementation, Governance

Welcome to the official technical documentation for **Custos** — Human-Centered Agentic Work Runtime.

This documentation combines standards from **arc42**, the **C4 model**, and **ADR (Architectural Decision Records)** to provide a comprehensive view from product philosophy and cognitive models to system protocols and implementation details.

---

## 1. Role-Based Reading Pathways

Select your reading path according to your focus within the project:

### Pathway 0: The Master Blueprint (All Contributors)
Focus: Understanding the complete product direction, canonical architecture, and logic.
- **Start Here:** Read the [Canonical Specification](./canonical-specification.md) from end to end. This is the single source of truth for the final product vision.

### Systems & Backend Engineers (Rust / Systems)
Focus: Kernel, State Machine, Capability Gateway, Sandboxing, Persistence.
- Start with [System Architecture Overview](./architecture/overview.md) and [Task Lifecycle](./architecture/task-lifecycle.md)
- Explore safe execution at [Capability Gateway](./architecture/capability-gateway.md)
- Study persistent storage at [Persistence & Storage](./architecture/persistence.md)
- Review error handling and resilience at [Crash Recovery](./architecture/crash-recovery.md)
- Inspect monorepo structure and crate layout at [Codebase & Monorepo](./development/codebase.md)

### AI Engineers & Cognitive Architects (AI / Prompts / Agents)
Focus: Cognitive Control Fabric, System One, Providers, Context & Memory.
- Study [Cognitive Control Fabric](./architecture/cognitive-fabric.md) (System One vs System Two, RDC protocol)
- Review model integration contracts at [Provider Interoperability](./architecture/provider-interop.md)
- Inspect memory and context management at [Context & Memory Architecture](./architecture/context-memory.md)
- Explore specialized domain packs: [Engineering Pack](./domains/engineering.md), [Research Pack](./domains/research.md), [Personal Pack](./domains/personal.md)

### Security & Platform Engineers
Focus: Trust Boundaries, Exact-Payload Approvals, Sandboxing, Secrets.
- Review [Threat Model (STRIDE)](./security/threat-model.md)
- Review [Capability Model](./security/capability-model.md)
- Review [Privacy & Data Governance](./security/privacy.md)
- Inspect local deployment and isolation at [Deployment Architecture](./architecture/deployment.md)

### Product Managers, QA & Contributors
Focus: Product Identity, Invariants, Roadmap, Testing Architecture.
- Read [Product Identity](./product/identity.md) and [Product Scope & MVP DoD](./product/scope.md)
- Review [Capability Map](./product/capability-map.md)
- Review [Core Concepts Glossary](./reference/concepts.md) and [Principles & Invariants](./reference/invariants.md)
- Review [16-Week Roadmap](./development/roadmap.md) and [Testing Architecture](./development/testing.md)

### Development Team & AI Collaboration
Focus: Internal collaboration between Vi (AI Engineer) & Truong (Software Engineer), Git workflow, sprint deliverables.
- Review [Internal Coordination Hub](../dev_docs/README.md)
- Review [Active Sprint Status](../dev_docs/SPRINT_STATUS.md)
- Review [AI Assistant Vibecoding Rules](../AGENTS.md)

---

## 2. Canonical Technical Documentation Directory

```text
docs/
├── 00-start-here.md                 # This guide (Overview & Reading Pathways)
├── canonical-specification.md       # Master Blueprint (Product Direction, Architecture & Logic)
│
├── product/                         # Product Strategy & Definition
│   ├── identity.md                  # Identity, personas, JTBD, core advantages, non-goals
│   ├── scope.md                     # Release horizons (H1/H2/H3), MVP Definition of Done
│   └── capability-map.md            # 6-domain capability taxonomy and priority matrix
│
├── architecture/                    # Technical Architecture Specifications
│   ├── overview.md                  # 7 layers, 7 planes, 3 membranes, C4 diagrams
│   ├── task-lifecycle.md            # Domain model, Task State Machine, SQLite schema, runtime scenarios
│   ├── cognitive-fabric.md          # System One vs System Two, RDC protocol, Pluggable Backends
│   ├── capability-gateway.md        # ExecutionPermit, Tool Registration, MCP client, Sandboxing
│   ├── evidence-verification.md     # 6-tier Evidence, Verification Pipeline, Outcome Bundle
│   ├── communication.md             # Internal communication, Star Topology, CP Message Envelope
│   ├── provider-interop.md          # ProviderPort, model adapters, ContinuationPacket, switching
│   ├── context-memory.md            # ContextPack scoring, 5-tier memory, promotion pipeline
│   ├── persistence.md               # SQLite + WAL, Event Store, CAS, Outbox, Backup
│   ├── crash-recovery.md            # 7-fault matrix, Continuation Contract, Reconciliation loop
│   └── deployment.md                # Local daemon, Sandbox macOS/Linux, CLI & VS Code connector
│
├── domains/                         # Domain Packs
│   ├── engineering.md               # Coding Agent pipeline, Worktrees, Repo intelligence
│   ├── research.md                  # Research Agent, Claim-Evidence matrix, Obsidian export
│   └── personal.md                  # Assistant Agent, Autonomy Ladder, Connectors
│
├── security/                        # Security & Trust
│   ├── threat-model.md              # STRIDE threat model, trust boundaries, prompt injection defense
│   ├── capability-model.md          # Capability grants, Exact-payload human approval, Secrets
│   └── privacy.md                   # Zero default telemetry egress, local data retention
│
├── development/                     # Engineering Processes
│   ├── codebase.md                  # Monorepo structure, crate dependencies, toolchain
│   ├── roadmap.md                   # 16-week plan, 10 vertical slices, acceptance criteria
│   ├── testing.md                   # Test pyramid, crash/path/approval matrices
│   ├── oss-adoption.md              # 5-mode OSS adoption, scorecard, 7 mandatory spikes
│   └── observability.md             # OpenTelemetry tracing, metrics, local structured logging
│
├── reference/                       # Reference Materials
│   ├── concepts.md                  # Comprehensive glossary (20+ core terms)
│   ├── invariants.md                # 10 irreversible decisions, 7 principles, 8 invariants
│   ├── naming.md                    # Canonical naming conventions (25+ categories)
│   ├── comparisons.md               # Architectural comparisons: LangGraph, Temporal, AutoGen, Claude Code
│   └── sources.md                   # Foundational research and technical references
│
└── adr/                             # Architecture Decision Records
    └── README.md                    # Index of 37 formal decisions (ADR-0001 through ADR-0037)
```

---

## 3. Documentation Governance Principles

1. **Reality First:** Every diagram, directory tree, and trait code references concrete executable components.
2. **Explicit Contracts:** All public interfaces must declare explicit data types and responsibility definitions.
3. **Verified Evidence:** All claims regarding performance or cost efficiency must be backed by benchmark evidence or explicitly designated as a *hypothesis*.
4. **Synchronized Baseline:** Source code, database schemas, and documentation are strictly versioned together under the same *canonical baseline*.
