# Custos active workboard

**Observed:** 2026-09-30
**Branch:** `refactor/11-crates-consolidation`
**Base HEAD:** `ccd5232aefb0b5d9e2fb8c01cd8337e47070e030`
**Checkout:** materially dirty; not a transferable baseline

## Current gate

The team is at Gate 1: establish a clean eleven-crate baseline without claiming
that experimental ports or imported modules are daemon-wired. Product feature
expansion must not outrun trusted evidence and durable effect closure.

## Start-now packets for the three people

These packets allow parallel research and fixtures on the current checkout;
they do **not** authorize an unreviewed shared DTO or a release claim. Each
person keeps one primary packet and one review obligation active at a time.

| Packet and driver | Exact scope | Review/handoff evidence | Stops before |
|---|---|---|---|
| P-01 Vi: F1 and evidence semantics | `custos-runtime/src/context`, `custos-provider`, `custos-packs/engineering`, `evals`; define pinned ContextPack, citations, omissions and direct-model baseline | Truong reviews privacy/evidence identity; Vinh reviews provider-to-Run handoff; fixture for stale/unsupported citation | C-04 schema or daemon completion change without joint review |
| P-02 Truong: durable trust baseline | `Cargo.lock`, `custos-persistence`, `custos-core`, `custos-daemon`, `tests/crash`; patch SQLite, assert runtime version, add failing forged-claim and WAL/restart fixtures | Vi reviews criterion meaning; Vinh reviews recovery; record exact SQLite version and commands | Claiming proof closure before accepted C-04 and process E2E |
| P-03 Vinh: recoverable Session/agent boundary | `custos-bridge`, `custos-runtime/src/{session,workflow,agent}`, `tests/e2e`, `custos-sdk`; pin idempotent promotion, lease/cancel, and Local API client fixtures | Truong reviews state/durability; Vi reviews agent/tool assurance; duplicate/restart tests | C-01/C-02 public DTO mutation or agent-native effects without contract |

**Joint decision packet J-01:** the three maintainers review C-04 evidence
identity and C-03 effect attempt/permit bindings with valid, forged, stale,
duplicate, uncertain, and restart examples. Record acceptance in an ADR and
contract version before P-01/P-02/P-03 integrate their shared types. Vi
coordinates the decision and evaluation; Truong owns trusted implementation;
Vinh owns lifecycle and process fixtures. Use
[`DEV-REPO-01`](../docs/development/repository-structure.md) for the full
edit-zone map and [`DECISION_QUEUE.md`](DECISION_QUEUE.md) for open choices.

| Workstream | Driver | State | Current evidence | Exit gate |
|---|---|---|---|---|
| Eleven-crate consolidation | Truong coordinates | Active | Workspace compiles; full tests pass; dependency guard passes | Format clean; bounded reviewed commits; clean SHA; docs and CI agree |
| Documentation authority and topology | Vi coordinates | Review | Compact canonical docs; role-based reading paths, current/target repository map, three start-now packets, and one ADR proposal | Truong and Vinh review edit zones and C-02/C-04 refinement; keep proposal/implementation distinct |
| C-01 Session/Task bridge | Vinh + Truong | Partial | Persistent Session/journal and bridge operations exist | Atomic/idempotent promotion and durable binding fixtures |
| C-02 Run/Workflow | Vinh | Partial | Types, scheduler and checkpoint tests exist | WorkflowRevision/compiler, durable lease/cancel/restart through daemon |
| C-03 Effect authority | Truong + Vinh | Unsafe prototype | CapabilityPort and adapters compile; SQL tables exist | Exact durable permit + EffectAttempt + outbox + uncertain reconciliation |
| C-04 Trusted evidence | Truong + Vi | P0 blocker | Missing evidence is rejected | Remove caller-authored canonical claims; validate Task/revision/criterion/source/receipt/verifier identity |
| F1 read-only Engineering | Vi + Vinh; Truong reviews | Partial | Test-process repo explain slice exists | Real daemon ContextPack -> ModelPort -> cited EvidenceRecord -> persisted Outcome |
| F2 controlled mutation | Truong + Vinh; Vi reviews | Blocked by C-03/C-04 | Prototype developer/MCP capabilities and controlled-effect test exist | Daemon-owned exact permit/effect/receipt/evidence and crash fixture |
| Context path consolidation | Vi | Open | Multiple context/compaction paths compile | One canonical compiler and ownership ADR; daemon F1 uses it |
| Gateway/routing responsibility | Vi; all review | Frozen | Cognitive and gateway policy overlap; mock dispatch | ADR chooses one routing authority and names external adapter boundary |
| 9Router / Agentgateway | Vi + Vinh; Truong security review | Deferred | Architecture positioning only | F1/F2 stable, upstream pinned, shadow conformance proves value |
| Goose migration | Subject owners; Vi coordinates | Active debt | Naming guard reports 523 files | Per-file disposition and compatibility removal gates |
| SQLite WAL safety | Truong | P0 open | Bundled SQLite 3.45.0 is in upstream WAL-reset bug range | Fixed SQLite build, runtime assertion, concurrent writer/checkpoint crash fixtures |
| CLI/UI/packages | Vinh | Deferred from core gate | Adjacent clients and wrappers exist | Versioned Local API, generated bindings, no direct state or duplicated orchestration |
| Cross-pack evidence and revalidation | Vi + Truong; Vinh reviews | Designed proposal | Architecture sections 15–20 and runtime F4/F5; D-13 pending | Changed-source and merged-artifact fixtures; no stale closure or cross-workspace reuse |
| Automatic setup | Vinh + Truong; Vi reviews | Designed proposal | Runtime F8 and D-14 pending | Idempotent approved installation, readiness probe, failure recovery, no untrusted hooks during discovery |

## Immediate sequence

1. Format and split the current dirty consolidation into reviewable commits.
2. Upgrade/pin SQLite and add WAL concurrency/crash regression coverage.
3. Accept C-04 trusted evidence identity and completion contract.
4. Accept C-03 exact permit, EffectAttempt, Receipt, and reconciliation contract.
5. Implement repositories and daemon composition for C-03/C-04.
6. Complete F1 on the actual daemon path.
7. Complete F2 with safe filesystem/process execution and crash injection.
8. Implement OutcomeBundle and usage settlement.
9. Expand Research and Assistant vertical slices.
10. Evaluate JudgmentPort and optional gateway profiles.

## Merge evidence required

Every claimed exit gate records:

- clean commit SHA and branch;
- exact commands and relevant output summary;
- valid, denied, stale, uncertain, crash, and restart behavior as applicable;
- contract/schema versions;
- owner and reviewers;
- remaining limitation and rollback or roll-forward plan.
