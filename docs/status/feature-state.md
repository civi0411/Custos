# Custos Feature Implementation State

> **Historical snapshot:** This page refers to an earlier checkout and PR naming. It is not the active status board. Use [status rules](README.md) and the [current workboard](../../dev_docs/SPRINT_STATUS.md); re-run any verification before citing it for the present tree.

## Tracking Metadata

- Super Plan Version: V4 Final
- Current Phase: C0 Verification & PR-00
- Repository: `Custos`
- Target Branch: `vi`
- Commit SHA: `acc6dbd2a1d7648227b26c4327200008d11951f7`

---

## 1. Work Package and PR Status Matrix

| PR ID | Target Scope | Owner | Issues | Current State | Verification Gate / Stopping Condition |
|---|---|---|---|---|---|
| **PR-00** | Inventory & C0 Baseline | Vinh + Truong | F01, F02 | VERIFIED & READY | `source-path-map.md`, `codebase-inventory.md`, and `GOOSE_SOURCE.md` authored. 52 tests passing. |
| **PR-01** | Core Domain Contracts | All (Vi signs off) | F03, F04, F05 | FOUNDATION MERGED | `TaskRevision`, `stale` base detection, `ActionLifecycleState` (no blind retry), `WorkerRun` retry, `WorkflowIR`. 10 tests passing. |
| **PR-02** | Persistence & Store Engine | Truong | F06, F09, F10 | FOUNDATION MERGED | Migrations `0004_usage.sql` (native schema) and `0005_evidence.sql`. 13 canonical tables verified in tests. |
| **PR-02b** | Minimal Local API & Daemon | Truong | F07, F08 (partial) | FOUNDATION MERGED | `LocalApiDispatcher` (`create`, `get`, `cancel`, `advance`, `list`), daemon supervisor runtime bootstrap. Gate for PR-04 ready. |
| **PR-G01** | Goose SHA Pin & Provider Spike | Vinh | G01, G02 | DOCUMENTED | Pinned upstream documentation committed in `GOOSE_SOURCE.md`. Conformance tests pass. |
| **PR-03** | Model Provider Adapters | Vi | M01, M02 | SKELETON READY | `OllamaHttpConfig` & HTTP client adapter skeleton. Zero direct cloud SDKs. Egress preflight check. |
| **PR-04** | Engineering Read Pack | Vi + Truong | M03, M04, E01 | READY TO EXPAND | Extended passing `repo_explain_slice.rs`. Consolidated tools with path traversal prevention. |
| **PR-05** | Authority & Capability Gateway | Truong + Vinh | X01, X02, X03 | SKELETON READY | ExecutionPermit minting/validation in `DeterministicGate`. Tool execution permit enforcement. |
| **PR-06** | Engineering Patch Pack | Vi + Vinh | E02, E03, E04 | PROPOSED | Base hash mismatch detection, symlink jailbreak prevention, verification runner on final snapshot. |
| **PR-07** | Research Reading Pack | Vi | R01, R02, R03 | SKELETON READY | `SemanticSupportEvaluator` implemented and tested against Vector 7 (`unsupported_citation`). |
| **PR-08** | Assistant Draft Pack | Vi + Truong | A01, A02 | PROPOSED | Identity/recipient validation. Draft preview without outbound dispatch. Personal memory isolation. |
| **PR-09** | Assistant Action Pack | Truong + Vinh | A03 | PROPOSED | Connector approval reconciliation. No blind resend on timeout. UNCERTAIN state resolution. |
| **PR-10a** | Basic UI Across Packs | Vinh + Truong | U01, U02, U03, P01 | PROPOSED | CLI task cards, VS Code client, approval inbox. Accurate known/unknown outcome indicators. |
| **PR-10b** | Advanced Workflow Engine | Vinh | F08 (full), W01, W02 | PROPOSED | Typed DAG compiler, child grant delegation, bounded dynamic retries. Gated after Q01/Q02 pass. |
| **PR-11** | Optimization & System 1 Routing | Vi | M05, Q01-Q04 | PROPOSED | Paired evaluation vs baseline. S1 routing enabled only for verified classes. |


---

## 2. Mandatory Verification Test Vectors

| # | Test Vector Identifier | Primary Target | Expected Assertion | Target PR |
|---|---|---|---|---|
| 1 | `no_permit` | Capability Gateway | Worker requests write without valid `ExecutionPermit` -> Rejected, zero disk mutations, audit log = `action_denied`. | PR-05 |
| 2 | `revoked_pre_dispatch` | Authority Engine | Human approval revoked prior to dispatch -> Permit invalidated, zero side-effects. | PR-05 |
| 3 | `stale_base` | Engineering Pack | Target file modified after patch preview -> Generates `ConflictArtifact`, preserves user changes. | PR-06 |
| 4 | `crash_post_effect` | Capability Gateway | Connector executes effect, daemon crashes before receipt -> State recorded as `UNCERTAIN`, no blind retry. | PR-05 |
| 5 | `receipt_post_crash` | Persistence Engine | Receipt stored in SQLite but worker crashes -> Resume injects receipt, effect count remains exactly 1. | PR-05 |
| 6 | `stale_evidence` | Evidence Engine | Source hash modified after verification -> Evidence marked stale, task does NOT transition to `SUCCEEDED`. | PR-06 |
| 7 | `unsupported_citation` | Research Pack | Snippet exists in source but semantic support fails -> `SemanticSupportEvaluator` fails, overall status not PASS. | PR-07 |
| 8 | `wrong_recipient` | Assistant Pack | Ambiguous recipient -> UI prompts for stable ID resolution, zero outbound calls. | PR-08 |
| 9 | `changed_payload` | Assistant Pack | Draft payload altered after approval -> Approval digest invalidated, re-approval mandatory. | PR-09 |
| 10 | `egress_denied` | Provider SDK | Cloud model requested for private local task -> Authority blocks egress, zero external network calls. | PR-03 |
| 11 | `user_pin` | Cognitive Runtime | Dynamic router attempts to switch model when user pinned -> Router override rejected, reports conflict. | PR-11 |
| 12 | `budget_parallel` | Workflow Runtime | Concurrent workers compete for remaining budget -> Admission control rejects secondary request. | PR-10b |
