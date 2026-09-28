# Custos decision queue

**Status:** active shared-decision register, 2026-09-28. A row is not an accepted ADR. `AGENTS.md` defines reviewers and ownership.

| ID | Decision | Driver | Required reviewers | Blocks | State |
|---|---|---|---|---|---|
| D-01 | Accept C-01 Session–Task transaction, idempotency and legacy-read behavior | Vinh + Truong | Vi | Durable promotion contract | Proposed |
| D-02 | Accept C-02 Run–Step leases, checkpoints, cancellation and recovery | Vinh | Truong + Vi | Daemon workflow composition | Proposed |
| D-03 | Accept C-03 Action–Attempt–Receipt authority and uncertainty model | Truong + Vinh | Vi | Controlled effects in daemon | Proposed |
| D-04 | Replace caller-authored `VerificationClaim` completion with trusted assessment identities | Truong + Vi | Vinh | Proof-carrying completion | Required now |
| D-05 | Keep, narrow or remove `custos-gateway` | Vi | Truong + Vinh | Any model-hub production wiring | Required before gateway work |
| D-06 | Select canonical context/compaction/memory ownership across overlapping crates | Vi | Vinh + Truong | F1 context path cleanup | Open |
| D-07 | Select canonical provider contract boundaries among SDK/types packages | Vi | Truong + Vinh | Route provenance schema | Open |
| D-08 | Define first raw executor, sandbox/control mode and supported OS matrix | Truong | Vinh + Vi | F2 effect implementation | Open |
| D-09 | Decide 9Router and Agentgateway adoption per deployment profile after conformance | Vi / Truong | All | Optional P1/P2 deployment | Deferred after F1/F2 |

## Decision packet

Each decision PR or meeting note contains:

- context, non-goals and affected current source paths;
- one canonical writer and storage boundary;
- versioned types/commands/events and compatibility window;
- valid, invalid, crash, retry and cancellation fixtures;
- authority, privacy, egress, budget and observability consequences;
- alternatives with explicit rejection reasons;
- migration, activation, rollback or roll-forward plan;
- owner, all required reviewers, date and code SHA.

Record the accepted result as an ADR, update C-01 through C-04 when applicable, then change code. If reviewers disagree, preserve the existing runtime behavior and narrow the proposed change; do not create a parallel contract.
