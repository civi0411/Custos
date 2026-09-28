# Custos documentation

**Document ID:** DOC-HUB-01. **Status:** active navigation and authority map. **Reviewed:** 2026-09-28 against the current dirty checkout and a fresh Custos Nexus index. This page is not evidence that the target architecture is implemented.

## Read in this order

1. [Product identity](product/identity.md) and [scope](product/scope.md): what Custos is for.
2. [Current implementation baseline](status/README.md) and [latest local audit](status/local-dev-audit-2026-09-28.md): what the checked source and tests establish.
3. [Target overview](architecture/target-architecture.md), then the [reference architecture](architecture/reference-architecture.md): the compact and full end-state designs.
4. [Runtime flows](architecture/runtime-flows.md): exact ownership and failure behavior for Session, Task, model, agent, MCP, effect, evidence and recovery paths.
5. [Contract register](contracts/README.md): the four cross-team contracts that must be accepted before changing shared types.
6. [Implementation blueprint](development/implementation-blueprint.md) and [PR-00 gates](development/pr-00-gates.md): how three people build it incrementally.
7. [Repository structure](development/repository-structure.md) and [naming policy](development/naming-conventions.md): current packages, dependency graph, canonical names and migration debt.
8. [Development work hub](../dev_docs/README.md): active work, ownership, blockers and evidence handoff.

## Document authority

| Kind | Source | Meaning |
|---|---|---|
| Source truth | Rust/SQL/configuration at a pinned commit and reproducible tests | What is implemented, wired, and verified at that commit. |
| Governance | [`AGENTS.md`](../AGENTS.md) and accepted ADRs | Editing rules and decisions. Existing ownership disagreement is explicitly unresolved. |
| Accepted contracts | Versioned contract plus an accepted ADR and conformance fixtures | Normative behavior for producers and consumers. |
| Target design | `architecture/target-architecture.md` and domain architecture pages | Desired state; not automatically present in the current daemon. |
| Status snapshot | `status/` with date, commit, dirty-state note, command and result | Observation valid for that snapshot only. |
| Work plan | `development/` and `dev_docs/` | Proposed sequencing and estimates, not feature verification. |
| Research/vendor | `specifications/`, `goose/`, analyses and external sources | Input for decisions, not Custos authority. |

The legacy [V8 master specification](canonical-specification.md) is retained as a design baseline. Its models, code snippets and directory sketches are proposals unless a current contract or source-backed status page says otherwise. Earlier pages headed “Canonical Baseline” have the same limitation. The [document register](document-register.md) records each family and its disposition.

## Vocabulary for status claims

- **Designed:** specified but not necessarily coded.
- **Implemented:** code exists in a crate or adapter.
- **Wired:** a production entrypoint actually calls it.
- **Verified:** a pinned test or repeatable inspection demonstrates the stated behavior, including relevant failure paths.
- **Degraded/Unknown:** part of the chain cannot be observed or established.

No page may turn `Implemented` into `Wired`, or `Wired` into `Verified`, by inference from a crate name, schema, README, or passing build alone.

## Navigation by concern

| Concern | Start here |
|---|---|
| Task, Session, Run, evidence | [Target architecture](architecture/target-architecture.md), [contract register](contracts/README.md), [task lifecycle](architecture/task-lifecycle.md) |
| End-to-end request and recovery flows | [Runtime flows](architecture/runtime-flows.md) |
| Multi-System-One/Two and model hub | [Intelligence Hub](architecture/intelligence-hub.md), [cognitive fabric](architecture/cognitive-fabric.md) |
| MCP, coding-agent and gateway connections | [Connectivity hubs](architecture/connectivity-hubs.md), [research findings](research/connectivity-gateway-findings.md) |
| Controlled effects and recovery | [Capability gateway](architecture/capability-gateway.md), [crash recovery](architecture/crash-recovery.md), [security](security/capability-model.md) |
| Current crate layout and delivery | [Implementation blueprint](development/implementation-blueprint.md), [PR-00 gates](development/pr-00-gates.md) |
| Goose, 9Router, Agentgateway | [Research register](research/README.md) and [vendor boundary](vendor/README.md) |
| Goose-derived names and compatibility | [Naming migration register](status/goose-naming-migration.md) and [naming policy](development/naming-conventions.md) |
| Historic plans and snapshots | [Document register](document-register.md), not the active sprint |

## Language and ownership

Current repository policy requires technical English under `docs/` and `dev_docs/`, with translations under `docs/i18n/`. Existing Vietnamese legacy files are preserved as historical inputs rather than silently rewritten. The lead directed this documentation restructuring and current ownership map; that direction does **not** imply that target behavior exists or a shared contract has passed implementation review. Record material contract decisions in ADRs before the corresponding code PR merges.
