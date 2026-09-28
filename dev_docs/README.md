# Custos development work hub

**Status:** lead-directed coordination framework, 2026-09-27. This page organizes work; it does not assign disputed shared-module ownership or certify implementation. Read [`AGENTS.md`](../AGENTS.md) for repository rules and the [documentation authority map](../docs/README.md) for design versus code truth.

## Where work lives

| Need | Source |
|---|---|
| Current work and blockers | [Sprint status](SPRINT_STATUS.md) |
| Decisions blocking shared work | [Decision queue](DECISION_QUEUE.md) |
| Standard issue/PR handoff | [Work item template](WORK_ITEM_TEMPLATE.md) |
| Full target system design | [Reference architecture](../docs/architecture/reference-architecture.md) and [connectivity hubs](../docs/architecture/connectivity-hubs.md) |
| Three-person build sequence | [Implementation blueprint](../docs/development/implementation-blueprint.md) |
| PR-00 decisions, migration and rollback | [PR-00 gates](../docs/development/pr-00-gates.md) |
| Shared contract proposals | [Contract register](../docs/contracts/README.md) |
| Code-backed capability status | [Status method](../docs/status/README.md) and dated evidence reports |
| Latest local restructure audit | [2026-09-28 audit](../docs/status/local-dev-audit-2026-09-28.md) |
| Current ownership map | [Module ownership matrix](MODULE_OWNERSHIP.md), summarized from `AGENTS.md` |
| Goose naming migration | [Measured migration register](../docs/status/goose-naming-migration.md) and [naming policy](../docs/development/naming-conventions.md) |
| Vi's AI/product notes | [`vi/`](vi/README.md) |
| Truong's platform/security notes | [`truong/`](truong/README.md) |
| Vinh's runtime/coordination notes | [`vinh/`](vinh/README.md) |
| Earlier coordination text | [`archive/`](archive/README-2026-09-27-pre-restructure.md) |

## Working rules

One Task/Session/Run/effect/evidence state has one canonical writer. A model proposes; Authority grants; the Gateway performs controlled effects; the verifier assesses evidence; the Kernel closes a Task. CLI/IDE/bots and 9Router/Agentgateway do not write canonical Task state. A shared-boundary PR names its producer, consumer, owner and reviewer and cites a versioned contract. Status claims are tied to a code SHA and reproducible test, not a readme assertion.

For shared domain, security, persistence, DTO and effect changes, all three maintainers review. `AGENTS.md` is the current policy and the ownership matrix is its operational summary. Lead direction establishes working ownership but is not evidence that a changed domain contract passed producer/consumer review.

## Weekly rhythm

1. Pin code SHA, dirty-state summary, Cargo inventory, current test baseline and changed upstream versions.
2. Each owner posts a bounded deliverable, evidence command/result, blockers and next decision in their workspace.
3. Review shared contracts and failure fixtures before interface-changing PRs.
4. Run F1/F2/F3 through the daemon process when the relevant gate is claimed; retain failure logs and limitations.
5. Update [Sprint status](SPRINT_STATUS.md) with `Proposed`, `Active`, `Blocked` or `Verified at SHA`; never write “100% complete” without scoped proof.

## Delivery rule

Every active item has one outcome, one owner, named reviewers, affected contracts, a smallest process-level fixture and a rollback or roll-forward note. Work that changes shared DTO, Task/effect/evidence semantics starts as a decision/contract PR. Runtime wiring, adapters and UI follow in separate reviewable PRs. The team does not develop new features on top of an unshareable local restructure.

## Scope of historical material

The former work-hub prose and sprint report are preserved in `archive/`. Dated reports under `truong/reports/` and `vinh/reports/` are also historical author records: commands, branch names, paths, plans, and completion claims inside them are not active instructions. The divergent former `docs/dev_docs/` tree is preserved in [`docs/archive/dev_docs-2026-09-27/`](../docs/archive/dev_docs-2026-09-27/); it is not an active workboard. The large `docs/goose/` tree is imported upstream reference, not an implemented Custos capability list. See the [document register](../docs/document-register.md) before moving or deleting more material.
