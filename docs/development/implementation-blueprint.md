# Implementation blueprint for the existing Custos tree

**Document ID:** DEV-BUILD-01. **Status:** sequencing reference, partially superseded by the 2026-09-28 implementation. Follow the [active workboard](../../dev_docs/SPRINT_STATUS.md) and [status evidence rules](../status/README.md); do not replay this PR graph as an agent backlog.

For the full top-level and package map, exact workspace inventory and dependency debt, use the [repository structure map](repository-structure.md).

## Build the existing tree, not a second platform

Cargo metadata on 2026-09-28 reported **42 workspace members** and five additional package manifests outside membership: `crates/core/custos-sdk`, `crates/runtime/custos-engine`, `crates/adapters/custos-acp-macros`, `tests/custos-test`, `tests/custos-test-support`. Presence outside the workspace is not proof that a package is obsolete; adoption needs a concrete use case, dependency/license review and tests.

| Layer | Existing homes | Work needed |
|---|---|
| Domain and Task | `crates/core/custos-domain`, `custos-kernel`, `custos-bridge` | One set of versioned contracts, single state writer, Session binding semantics. |
| Runtime | `crates/runtime/custos-session`, `custos-workflow`, `custos-agent`, `custos-cognitive`, `custos-context`, `custos-security` | Wire persistence, bounded steps, controlled effects, role routing and context. |
| Data and ports | `crates/infrastructure/custos-persistence`, `crates/core/custos-provider-sdk` | Additive migrations, route/effect ledger, provider provenance. |
| Adapters and packs | `crates/adapters/*`, `crates/packs/*`, `tools/repo_intelligent` | Conformance-tested executor/provider/repo adapters and typed pack workflows. |
| Application | `crates/app/custos-daemon`, `custos-local-api`, `custos-cli` | One composition root, one versioned API contract and thin client. |

Two major consolidation decisions remain: `custos-context` versus `custos-context-management` compaction/memory code, and `custos-provider-sdk` versus provider-types/sdk-types. Daemon API dispatch now reuses DTOs from `custos-local-api`; preserve that single public DTO direction. Select one canonical contract at a time and migrate consumers. Do not delete legacy code before a process-level test proves replacement.

## Flow gates

| Gate | Outcome | Non-negotiable proof |
|---|---|---|
| G0 | Baseline, ownership and four contract proposals | SHA/dirty state, Cargo inventory, current tests, owner ADR or explicit blocker. |
| G1 / F1 | Read-only `ask/explain` through daemon | ContextPack actually reaches provider; exact citation/hash check; restart reloads Session/outcome; no fake success. |
| G2 | Durable effect spine | Attempt before dispatch, permit validation, real executor, Uncertain/reconcile, backup/restore. |
| G3 / F2 | Coding bug fix | Patch/test evidence attached to current criterion and subject; no terminal success on synthetic receipt. |
| G4 / F3 | Research-to-code | Versioned sources, support/contradiction, typed handoff, implemented patch and verified limitations. |
| G5 | Self-setup and optional Hub sidecars | Doctor/SetupPlan, clean-machine flow, version-pinned gateway conformance and rollback. |

Goose-derived features enter through a named port/call path, with upstream SHA, license, dependency diff and rollback owner. Do not activate all of `custos-engine` to create an appearance of integration. 9Router and Agentgateway spikes are parallel research, not prerequisites for F1.

## Team boundaries

Truong: domain/kernel/persistence/security/daemon and effect correctness. Vinh: Session/Bridge/workflow/agent/MCP and API/IDE protocol implementation. Vi: cognition/context/Nexus/provider/Engineering and Research packs, evals and product outcome semantics. `custos-domain`, public DTOs and the four contracts require all three to review. `AGENTS.md` is authoritative and `dev_docs/MODULE_OWNERSHIP.md` summarizes its assignments.

## PR graph and critical path

| PR | Lead | Size estimate | Hard dependency | Exit |
|---|---|---|---|---|
| 00 Baseline/ADR | All; Truong coordinates | M | None | Owner decision or documented blocker, contracts and source baseline. |
| 01 API contract | Vinh | M | 00 shared DTO decision | One versioned DTO, command ID and cursor. |
| 02 Session store | Vinh + Truong | L | 00 C-01 | Restart, promotion dedup, legacy read. |
| 03 Context/provider | Vi | M | 00 seam | Real ContextPack and provider request. |
| 04 Bounded runtime | Vinh | L | 00 C-02 | Step/checkpoint/cancel/lease fixtures. |
| 05 Daemon F1 | Truong | L | 01–04 | Process E2E; no simulated outcome. |
| 06 Action ledger | Truong | L | 00 C-03 | Durable attempt, permit, receipt schema. |
| 07 Effect dispatcher | Vinh + Truong | L | 04, 06 | Fault injection, no blind retry. |
| 08 Coding pack | Vi + Vinh | M | 03, 04, 07 | Real patch/test/evidence. |
| 09 Evidence closure | Truong + Vi | L | 06–08, C-04 | Current required criteria and effect resolution. |
| 10 Research pack | Vi | M | 03; 09 for release | Supported claims and typed research handoff. |
| 11 Setup/IDE | Truong + Vinh | L | 01, 02, 05 | Clean setup/reconnect/capability matrix. |
| 12 Gateway spikes | Vi (9Router), Truong (Agentgateway) | M each | 03 adapter seam | Adopt/defer ADR; no F1 block. |

S is at most one person-day, M is two to three, L is four to six, before review and a 20% buffer. Re-estimate after fixtures. F1 critical path is `00 → (01,02,03,04) → 05`. F2 continues through `06 → 07 → 08 → 09`; PR 06 may run in parallel after G0. Split an L-sized PR if its diff crosses several independent ownership domains.

## First five working days

Days 1–2: Truong maps DB/API/daemon and baseline; Vinh maps the active Goose-derived harness and tool path; Vi prepares explain, bug-fix and unsupported-claim fixtures. Day 3: decide ownership and C-01–C-04, or document the unresolved blocker. Days 4–5: Truong proposes additive Session migration, Vinh tests bounded step/Bridge behavior, Vi tests ContextPack/provider quality. Draw one request across the daemon and then test the real seam rather than declaring a hand-drawn flow implemented.

For migration, flags, rollback, F1 acceptance and performance targets use [PR-00 gates](pr-00-gates.md). No code feature becomes Verified from this plan alone.
