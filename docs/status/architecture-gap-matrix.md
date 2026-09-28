# Custos architecture gap matrix

**Document ID:** STATUS-GAP-01. **Snapshot:** 2026-09-28 dirty checkout at `205468e32c19395c3d907d898da3845aabfa9d82`. **Authority:** source-backed status, not target design or a clean-commit certification. See the [full local audit](local-dev-audit-2026-09-28.md).

| Boundary | Current evidence | Status | Next exit gate |
|---|---|---|---|
| Reproducible baseline | 42 workspace members compile and focused gates pass, but most restructured files are untracked and old paths appear deleted | Blocked | Clean feature-branch checkout reproduces metadata, guards, tests and classified migration diff |
| Local API / CLI | CLI uses `ProcessTransport` and `LocalApiClient`; daemon reuses `custos-local-api` DTOs | Wired for tested commands | Version/auth/cursor/idempotency contract and complete supported-command review |
| Session durability | Daemon injects `SessionManager::with_store`; process E2E proves Session, journal, binding and Task restart persistence | Verified in focused fixture | Accept C-01, malformed/legacy migration and promotion idempotency fixtures |
| Task terminal transition | API and Kernel reject direct `advance -> Succeeded`; completion calls `CompletionGate` | Wired | Trusted C-04 assessments, revision binding and unresolved-effect check |
| Evidence trust | Local API accepts full caller-supplied `VerificationClaim`; gate trusts `passed` and recognized verifier text | Unsafe | Client submits evidence/assessment IDs; daemon loads immutable trusted records and validates Task/criterion/revisions/verifier |
| Controlled tools | Deterministic read/list/preview enforce path containment and produce receipts in tests | Implemented, not daemon-wired | Daemon-owned C-03 action/attempt/receipt service with real executor and process E2E |
| Crash ambiguity | No durable attempt-before-dispatch/reconciliation path is proven through daemon | Designed | Crash-after-dispatch fixture yields `Uncertain`, reconciliation and no blind replay |
| Workflow | Workflow types and tests exist; daemon bootstrap does not compose the bounded runtime | Implemented in isolation | C-02 lease/checkpoint/cancel/restart through production entrypoint |
| Context/provider F1 | Context/cognitive/provider packages exist; daemon bootstrap does not compose a real ContextPack-to-provider path | Implemented in isolation | Exact source snapshot reaches provider; citation/unsupported-claim/restart fixtures pass |
| Model routing | Cognitive policy selects scalar tier/provider/model values; no durable versioned candidate registry/attempt ledger | Partial | Role registry, eligibility filters, actual-route provenance, budget reserve/settle and evals |
| `custos-gateway` | `route_and_execute` is called only by its own test; dispatch and usage/cost are synthetic | Experimental/frozen | ADR chooses removal, narrow port extraction or replacement; never second orchestration authority |
| MCP capability hub | MCP crates and adapters exist; per-user/per-Task projection is not proven in daemon | Partial | Connection lifecycle, schema pinning, capability projection and exact argument authority fixtures |
| External coding agents | Goose-derived runtime/UI/provider material exists; native-tool interception is unproven | Unknown per adapter | Capability handshake and `Custos-mediated`/`Provider-governed`/`Observe-only` conformance per agent |
| 9Router / Agentgateway | Architecture and research exist; no production integration is present | Research only | Pin versions, fake-upstream conformance, one fallback owner, provenance/cancel/usage/failure tests |
| Persistence boundary | Task/Session persistence works; Bridge depends on runtime Session and concrete persistence | Transitional debt | Narrow ports or move Bridge orchestration outward without breaking restart fixtures |
| Non-Rust clients | Desktop is Goose-derived, VS Code is a shell, bot calls Anthropic directly; current Rust CI does not cover them | Deferred | Governed Local API/agent adapter integration plus JS/TS/OIDC CI |
| Out-of-workspace manifests | Five manifests exist outside the 42-member workspace | Unvalidated | Adopt narrowly with license/dependency/tests or classify as source pool/test support |

Nexus was used to discover candidate call paths: it found `dispatch_for_task` only in the security trait/tests and controlled-effects E2E, and `route_and_execute` only in the gateway test. Direct source inspection confirmed daemon bootstrap currently constructs only persistence, Task, Session, Bridge and Local API services. Nexus edges are syntactic and do not prove runtime reachability.

Priority is fixed: baseline → trusted evidence → daemon-owned effects/recovery → real F1 → F2/F3 → optional hubs and clients. A row moves to `Verified` only with a pinned clean SHA, exact command/test, relevant failure path and stated limitation.
