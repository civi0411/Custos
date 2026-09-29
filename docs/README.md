# Custos Documentation Hub

**Document ID:** DOC-HUB-01  
**Status:** Active navigation, governance, and authority map  
**Reviewed:** 2026-09-28 against 42-crate workspace and latest verification audit  
**Quickstart Guide:** For role-based onboarding paths, see [`00-start-here.md`](00-start-here.md)

---

## 1. Primary Reading Order

Follow this sequence to build a rigorous understanding of Custos:

1. **Orientation & Pathways:** [`00-start-here.md`](00-start-here.md) — Role-based pathways and core runtime invariants.
2. **Product Identity & Scope:** [`product/identity.md`](product/identity.md) and [`product/scope.md`](product/scope.md) — What Custos is, problems solved, and non-goals.
3. **Current Verified Baseline:** [`status/local-dev-audit-2026-09-28.md`](status/local-dev-audit-2026-09-28.md) and [`status/README.md`](status/README.md) — What the checked source code and E2E tests establish today.
4. **Reference Architecture:** [`architecture/reference-architecture.md`](architecture/reference-architecture.md) — Complete 6-plane architecture, membranes, and system boundaries.
5. **Runtime Flows:** [`architecture/runtime-flows.md`](architecture/runtime-flows.md) — End-to-end trace of Task intake, cognitive routing, effect sandboxes, and evidence closure.
6. **Cross-Team Contracts:** [`contracts/README.md`](contracts/README.md) — The normative contracts (C-01 Local API, C-02 Task, C-03 Action/Permits, C-04 Evidence).
7. **Implementation Blueprint & Gates:** [`development/implementation-blueprint.md`](development/implementation-blueprint.md) and [`development/pr-00-gates.md`](development/pr-00-gates.md) — PR delivery milestones and quality gates.
8. **Repository Structure:** [`development/repository-structure.md`](development/repository-structure.md) — 42-crate dependency tree and layering boundaries.
9. **Active Sprint & Team Ownership:** [`../dev_docs/README.md`](../dev_docs/README.md) — Current work, module ownership, and active blockers.

---

## 2. Document Authority Hierarchy

To resolve contradictions across documents, Custos enforces a strict authority hierarchy:

| Tier | Category | Sources | Meaning & Authority |
|---|---|---|---|
| **Tier 1** | **Source Truth** | Rust source code, SQL migrations, Cargo workspace metadata, and reproducible tests | Definitive authority on what is implemented, wired, and verified at the pinned commit. |
| **Tier 2** | **Governance** | [`AGENTS.md`](../AGENTS.md) and accepted ADRs under [`adr/`](adr/README.md) | Authoritative rules for engineering standards, architecture decisions, and code modifications. |
| **Tier 3** | **Normative Contracts** | Formal schemas in [`schemas/`](../schemas) and contracts in [`contracts/`](contracts/README.md) | Binding cross-team contracts for local API, task events, execution permits, and evidence. |
| **Tier 4** | **Target Architecture** | [`architecture/reference-architecture.md`](architecture/reference-architecture.md) & [`architecture/target-architecture.md`](architecture/target-architecture.md) | Official architectural target. Describes intended end-state; not proof of current implementation. |
| **Tier 5** | **Audit Snapshots** | Dated files in [`status/`](status/README.md) | Empirical records of repository status at a specific date and commit. |
| **Tier 6** | **Historical & Vendor** | [`canonical-specification.md`](canonical-specification.md), [`specifications/`](specifications/README.md), [`goose/`](goose/README.md), [`archive/`](archive/README.md) | Historical context, original design proposals, and upstream references. Completely non-normative. |

---

## 3. Status Verification Vocabulary

No document may claim a feature is ready without adhering to the status claim rules:

- **Designed:** Specified in documentation or schemas; not yet implemented in Rust code.
- **Implemented:** Source code exists in a crate and compiles cleanly under `cargo check --workspace`.
- **Wired:** Production composition root (`custos-daemon` or `custos-cli`) connects the feature.
- **Verified:** Tested end-to-end with repeatable verification (unit, integration, or E2E tests).
- **Degraded / Experimental:** Known issues, test gaps, or unhandled failure modes exist.

---

## 4. Navigation by Concern

| Technical Concern | Primary Documentation | Secondary References |
|---|---|---|
| **Task Lifecycle & State Machine** | [`architecture/task-lifecycle.md`](architecture/task-lifecycle.md) | [`crates/core/custos-kernel`](../crates/core/custos-kernel), [`adr/task-state-machine-alignment.md`](adr/task-state-machine-alignment.md) |
| **End-to-End Execution Flows** | [`architecture/runtime-flows.md`](architecture/runtime-flows.md) | [`architecture/reference-architecture.md`](architecture/reference-architecture.md) |
| **Cognitive Routing (System 0/1/2)** | [`architecture/cognitive-fabric.md`](architecture/cognitive-fabric.md) | [`architecture/COGNITIVE_ARCHITECTURE_S1_S2.md`](architecture/COGNITIVE_ARCHITECTURE_S1_S2.md), [`architecture/intelligence-hub.md`](architecture/intelligence-hub.md) |
| **Capability Sandbox & Permits** | [`architecture/capability-gateway.md`](architecture/capability-gateway.md) | [`security/capability-model.md`](security/capability-model.md), [`crates/runtime/custos-security`](../crates/runtime/custos-security) |
| **Evidence & Completion Verification** | [`architecture/evidence-verification.md`](architecture/evidence-verification.md) | [`contracts/README.md`](contracts/README.md) (Contract C-04) |
| **Context Compilation & Memory** | [`architecture/context-memory.md`](architecture/context-memory.md) | [`crates/runtime/custos-context`](../crates/runtime/custos-context), [`crates/runtime/custos-memory-service`](../crates/runtime/custos-memory-service) |
| **Persistence & Storage Engine** | [`architecture/persistence.md`](architecture/persistence.md) | [`crates/infrastructure/custos-persistence`](../crates/infrastructure/custos-persistence) |
| **Crash Recovery & Resumption** | [`architecture/crash-recovery.md`](architecture/crash-recovery.md) | [`architecture/deployment.md`](architecture/deployment.md) |
| **MCP & External Connectivity** | [`architecture/connectivity-hubs.md`](architecture/connectivity-hubs.md) | [`research/connectivity-gateway-findings.md`](research/connectivity-gateway-findings.md), [`crates/adapters/custos-mcp`](../crates/adapters/custos-mcp) |
| **42-Crate Monorepo Structure** | [`development/repository-structure.md`](development/repository-structure.md) | [`development/codebase.md`](development/codebase.md), [`Cargo.toml`](../Cargo.toml) |
| **Quality & PR Gates** | [`development/pr-00-gates.md`](development/pr-00-gates.md) | [`development/testing.md`](development/testing.md) |
| **Goose Upstream & Migration** | [`status/goose-naming-migration.md`](status/goose-naming-migration.md) | [`vendor/README.md`](vendor/README.md), [`adr/goose-compatibility-boundary.md`](adr/goose-compatibility-boundary.md) |

---

## 5. Language and Translation Policy

All primary technical documentation under `docs/` and internal engineering coordination under `dev_docs/` is authored and maintained in **Technical English**. 

Localized translations for wider audiences are maintained under [`docs/i18n/`](i18n/README.md):
- [Vietnamese Translation](i18n/README.vi.md)
- [German Translation](i18n/README.de.md)
- [Chinese Translation](i18n/README.zh.md)

Historical Vietnamese design and research drafts are preserved under [`docs/archive/`](archive/README.md) for provenance, and are completely non-normative.
