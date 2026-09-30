# Custos development work hub

`dev_docs/` is an execution board, not an architecture library. It contains
only this operating model, the current sprint board, and the unresolved
decision queue.

## Sources of truth

| Need | Source |
|---|---|
| Repository policy and ownership | [`../AGENTS.md`](../AGENTS.md) |
| Product and system target | [`ARCH-DEF-01`](../docs/architecture/definitive-product-architecture.md) |
| Runtime flows | [`ARCH-FLOW-01`](../docs/architecture/runtime-flows.md) |
| Physical repository map | [`DEV-REPO-01`](../docs/development/repository-structure.md) |
| Proposed shared contracts | [`contracts/README.md`](../docs/contracts/README.md) |
| Current implementation truth | [`STATUS-CURRENT-01`](../docs/status/README.md) |
| Active priorities | [`SPRINT_STATUS.md`](SPRINT_STATUS.md) |
| Unresolved decisions | [`DECISION_QUEUE.md`](DECISION_QUEUE.md) |

## Team lanes

| Driver | Primary lane | Cross-review |
|---|---|---|
| Vi | Product semantics, architecture, context/memory, routing, providers, packs, evaluation, upstream research | Truong for authority/privacy/effects; Vinh for runtime/agent handoff |
| Truong | Domain, Kernel, persistence, authority, effects, evidence closure, daemon/API, sandbox, release | Vi for product/evidence semantics; Vinh for lifecycle/recovery |
| Vinh | Session/bridge, workflow, AgentRuntimePort, MCP/ACP/A2A, CLI/UI/SDK, integration coordination | Truong for durability/effects; Vi for agent/UX semantics |

`AGENTS.md` is authoritative when this summary differs. Shared domain,
authority, persistence, public DTO, and port changes require their producer,
consumer, owner, and required reviewers.

## Capacity model for a three-person team

The planned **implementation and research effort** is approximately Vi 55%,
Truong 23%, and Vinh 22% across the program. This is a planning range, not a
claim about hours already spent or a way to override code ownership. Rebalance
at each gate from observed throughput. Vi can drive 50-60% by owning product
architecture, source dissection, context/model/routing work, all three pack
semantics, evaluation fixtures, and integration acceptance; the SEs retain the
security-critical code they own.

| Person | Primary deliverables | Must not become the sole approval for |
|---|---|---|
| Vi, lead and AI/DS | Product decisions; research; context/memory; model and S1/OI experiments; pack criteria; evals; user-facing outcome review; cross-lane integration plan | His own C-04 semantics, egress policy, or unmeasured model-routing claims |
| Truong, core/platform SE | Domain/Kernel; SQLite and CAS; exact authority/effects; evidence closure; daemon; sandbox; release safety | Product acceptance criteria without Vi; workflow recovery without Vinh |
| Vinh, runtime/client SE | Session and workflow; external agent/MCP/ACP; API consumers; CLI/UI/SDK; setup and client integration | Effect dispatch/reconciliation without Truong; pack semantics without Vi |

The critical path is **Truong's durable trust boundary + Vi's evidence
semantics + Vinh's recoverable lifecycle**, reviewed together. During that
path, Vi's independent research/evals and Vinh's client/agent fixtures may
advance, but no one integrates a dependent feature before its contract gate.
Keep one shared-contract decision in active review at a time; avoid three
parallel rewrites of Task, permit, and evidence types. The [repository
map](../docs/development/repository-structure.md) names edit zones and safe
independent starting work.

## Product spine

```text
Session -> TaskContract -> Context/Route -> Run/Worker
        -> ActionIntent -> Permit -> EffectAttempt -> Receipt
        -> Evidence -> CriterionAssessment -> Outcome/Continuation
```

Each state family has one writer. Models and external agents propose; the
Kernel authorizes and closes Tasks; adapters execute; trusted verifiers assess.
Clients, routers, and protocol gateways remain outside canonical ownership.

## Work packet

Every active item in `SPRINT_STATUS.md` or its linked issue/PR must state:

- identifier, user outcome, owner, reviewers, and target gate;
- current evidence and exact affected paths;
- contract/ADR dependencies and explicit non-goals;
- valid, denied, stale, duplicate, uncertain, crash, and restart fixtures as
  applicable;
- observability, privacy, egress, cost, migration, and compatibility impact;
- rollback or roll-forward behavior;
- commands and evidence required to move from Implemented to Wired or Verified.

Do not create per-owner README files, weekly report copies, or another workboard.
Use issues/PRs for detailed execution history and update the one sprint row with
the current result.

## Change flow

1. Pin branch, commit, dirty state, workspace inventory, and upstream revisions.
2. Resolve a shared contract or decision before implementation when semantics
   would otherwise diverge.
3. Separate mechanical movement, contract changes, implementation, daemon
   composition, and client changes into reviewable units.
4. Test the production daemon path and its failure path, not only an in-process
   assembly.
5. Update `docs/status/README.md` with exact evidence; update architecture only
   when the target or boundary changed.

## Status language

Use `Designed`, `Implemented`, `Wired`, `Verified`, `Experimental`, or `Absent`.
Do not report percentages or “done” without a stated scope and repeatable exit
evidence.
