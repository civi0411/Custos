# Custos decision queue

**Status:** Active shared-decision register
**Updated:** 2026-09-30
**Authority:** A row records an unresolved decision, not an accepted design. Accepted decisions require an ADR and the reviewers defined in [`AGENTS.md`](../AGENTS.md).

| ID | Decision | Driver | Required reviewers | Blocks | State |
|---|---|---|---|---|---|
| D-01 | Define the atomic Session-to-Task promotion, idempotency key and legacy-read behavior. | Vinh + Truong | Vi | Durable promotion and restart recovery | Proposed |
| D-02 | Define Run/Step leases, checkpoints, cancellation, retry and crash takeover. | Vinh | Truong + Vi | Durable worker execution | Proposed |
| D-03 | Define Action/Attempt/Permit/Receipt identity, uncertainty and reconciliation semantics. | Truong + Vinh | Vi | Controlled effects | Proposed |
| D-04 | Replace caller-authored completion claims with trusted verifier identities and subject-bound evidence. | Truong + Vi | Vinh | Proof-carrying completion | P0 open |
| D-05 | Assign one responsibility to the internal gateway module and specify routing ownership between OI and adapters. | Vi | Truong + Vinh | Production model/agent routing | Open |
| D-06 | Define ContextPack ownership, budgeting, redaction, provenance and memory write-back. | Vi | Truong + Vinh | F1 explain and research quality | Proposed |
| D-07 | Freeze the boundaries of `ModelPort`, `AgentRuntimePort`, `CapabilityPort`, `JudgmentPort` and protocol gateways. | Vi + Vinh | Truong | Adapter conformance and daemon wiring | Partially implemented |
| D-08 | Select the first production effect executor, OS sandbox profile and supported platform matrix. | Truong | Vinh + Vi | F2 coding workflow | Open |
| D-09 | Adopt, adapt or reject 9Router and Agentgateway independently; select at most one model-routing edge per attempt and evaluate Agentgateway protocol-edge duties separately. | Vi + Truong | All | Optional deployment profiles | Deferred after F1/F2 |
| D-10 | Decide whether Local API dispatch remains in `custos-bridge` or moves behind an application service owned by the daemon. | Truong + Vinh | Vi | Stable API/application boundary | Open |
| D-11 | Define how Task status projects Run, Effect, Criterion and Approval state without collapsing their independent state machines. | Truong | Vinh + Vi | Correct UI/status reporting | Proposed |
| D-12 | Upgrade and pin a patched SQLite build, add runtime version enforcement, and define WAL checkpoint/crash fixtures. | Truong | Vinh + Vi | Any durability or release claim | P0 open |
| D-13 | Review evidence dependency identity, conservative invalidation, historical outcome applicability, and cross-pack handoff under C-02/C-04; see the evidence-driven workflow ADR proposal. | Vi + Truong | Vinh | Reusable evidence and change-driven revalidation | Proposed |
| D-14 | Define setup profile, credential references, installation previews, readiness probes, and scoped activation through existing effect contracts. | Vinh + Truong | Vi | Repeatable automatic onboarding | Proposed |

## Required decision packet

Every decision PR or meeting record must contain:

- context, non-goals and affected current source paths;
- one canonical writer and storage boundary;
- versioned commands, events and types with a compatibility window;
- valid, invalid, crash, retry, cancellation and revocation fixtures;
- authority, privacy, egress, budget and observability consequences;
- alternatives and explicit rejection reasons;
- migration, activation and rollback or roll-forward plan;
- owner, required reviewers, date and code SHA.

After acceptance, create or update the ADR, update the relevant contract and status matrix, then change code. If review is unresolved, preserve current runtime behavior; do not create a parallel contract.
