# Custos active workboard

**As of:** 2026-09-28. **HEAD:** `205468e32c19395c3d907d898da3845aabfa9d82` on `dev`. **Checkout:** materially dirty and not reproducible from the branch. See the [current local audit](../docs/status/local-dev-audit-2026-09-28.md).

| Work item | Lead | State | Current evidence | Exit condition |
|---|---|---|---|---|
| PR-00 reproducible baseline | Truong coordinates | **Blocked** | Rust metadata/check/fmt/Clippy and focused tests pass locally | Clean feature-branch checkout reproduces gates; every deletion/untracked tree classified; CI deduplicated |
| Agent and document authority | Vi coordinates | **Active** | Tool adapters now defer to `AGENTS.md`; historical reports isolated | Maintainers review policy diff; no competing canonical document or stale tool rule |
| Documentation architecture | Vi coordinates | **Ready for review** | Nexus-refreshed authority map, current gap matrix and runtime-flow contract | All three review target/current separation and accept or amend open decisions |
| C-01 Session–Task | Vinh + Truong | **Implemented in part** | Store-backed Session/journal and daemon restart E2E pass | Accepted versioned contract, idempotent promotion fixture, malformed/legacy migration behavior |
| C-02 Run–Step | Vinh | **Proposed** | Workflow types/tests exist | Durable lease/checkpoint/cancel/restart process fixture; Kernel remains sole Task writer |
| C-03 Action–Attempt–Receipt | Truong + Vinh | **Implemented in test harness only** | Deterministic read/list/preview, denial, receipt fixtures pass | Daemon-owned attempt-before-dispatch, real executor, crash uncertainty and reconciliation |
| C-04 Criterion–Evidence | Truong + Vi | **Unsafe boundary** | Missing/partial evidence is rejected | Client cannot forge claims; daemon resolves trusted current assessments by ID and checks criterion/revision/effects |
| F1 read-only explain | Vi + Vinh + Truong | **Active** | Daemon process/restart path exists | Real ContextPack reaches provider through daemon; exact citation and unsupported-claim fixture persist across restart |
| F2 controlled code effect | Truong + Vinh + Vi | **Active** | Effect and proof components compose in E2E test process | Full action-to-evidence path executes inside daemon composition; stale/uncertain paths block completion |
| Intelligence/gateway boundary | Vi + Vinh + Truong | **Blocked on design cleanup** | `custos-gateway` is test-only mock; cognitive pipeline overlaps | One routing authority; separate ModelPort, AgentRuntimePort, ToolPort, and effect authority; adopt/defer ADR |
| 9Router / Agentgateway | Vi / Truong | **Research only** | Target boundaries documented | Pinned versions, primary-source contract, fake-upstream conformance, explicit fallback owner; not on F1 critical path |
| Goose naming N0–N1 | Vi coordinates | **Active** | Naming guard passes with 574 debt files | Compatibility taxonomy, Custos-first dual read, redacted telemetry, migration fixtures |
| Desktop / VS Code / bot | Vinh + subject owners | **Deferred** | Adjacent packages exist; no current product E2E | Versioned API/protocol integration and non-Rust CI without direct Task database access |

## Immediate order

1. Make the current restructure reproducible without semantic expansion.
2. Close forged-evidence acceptance before describing proof-carrying completion.
3. Move the read-only effect path into daemon composition with durable attempt and receipt state.
4. Finish F1 with a real ContextPack/provider/verifier path.
5. Only then adopt optional model/network gateways or broaden clients.

## Next bounded changes

| Change | Primary | Required review | Must not include |
|---|---|---|---|
| Baseline migration PR | Truong coordinates | Vi + Vinh | New product behavior, broad renames, gateway activation |
| Trusted evidence contract and fixtures | Truong + Vi | Vinh | Provider/UI expansion or unrelated schema cleanup |
| Daemon effect composition and recovery | Truong + Vinh | Vi | 9Router/Agentgateway integration or hidden native-agent tools |
| Real F1 ContextPack/provider slice | Vi + Vinh | Truong | Mutating tools or completion claims unsupported by trusted evidence |

An item becomes `Verified` only with a pinned clean SHA, exact command or call path, relevant failure test, owner, reviewer, and stated limitation. Dated reports under owner workspaces are historical records and do not update this board.
