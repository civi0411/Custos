# Cross-team contract register

**Status:** proposed contracts. No item in this register is accepted solely because it appears here. The code, legacy schema and prior V8 design may differ. Acceptance requires the three maintainers' recorded approval for shared domain, security, persistence and public API semantics.

## Contract template

```text
ID and version; owner; producer; consumers; reviewers; decision link
Purpose and non-goals
Canonical writer/store and reads
IDs, revisions, expected versions, idempotency keys
Commands/events/responses and schema compatibility
States and legal transitions, including Unknown/Uncertain
Transaction, crash, retry, recovery and cancellation semantics
Authority, privacy, budget and data retention boundary
Valid and invalid fixtures with exact expected outcomes
Producer, consumer, migration and process-level tests
Rollback or roll-forward strategy and open questions
```

“Accepted” means owner and reviewers signed a versioned decision, valid/invalid fixtures run at both sides of the boundary, the writer is unique, migration/compatibility and recovery are tested, and PRs cite the exact contract version. A diagram or meeting note alone is not acceptance.

## C-01: Session–Task

SessionRun supports fast read-only interaction; a durable Task owns explicit goal, scope and acceptance criteria. `PromoteSession` is idempotent by command ID and binds a Session to one Task in a transaction with Task/revision/event. Repeating the same command/payload returns the same Task ID; reusing the key with a different payload is Conflict. A stale expected Session version is rejected without a partial Task.

Legacy `SessionStatus::Promoted {task_id}` remains readable during migration. The target representation is a binding event; Session can subsequently be Active, Paused or Closed. Do not reinterpret malformed status as Active. The current repository read path does this via `unwrap_or(SessionStatus::Active)`, so it is a migration defect to address before new writes.

**Valid fixture:** `command_id=cmd-01`, `session_id=ses-01`, expected version 3, contract with goal, scope and unique criterion IDs → one Task and one binding. **Invalid:** missing expected version, duplicate criterion ID, stale version or same command ID with changed payload → typed rejection and no partial rows.

## C-02: Run–Step

Workflow owns Run/Step and bounded retry, checkpoint, cancellation and handoff. Kernel alone owns Task state. Step transitions include not-started/running/waiting/completed/failed/uncertain as appropriate to the accepted schema. A resumed worker consumes a versioned continuation packet and cannot recreate a completed side effect. Leases fence stale workers; concurrent claims of one step produce one winner.

**Valid fixture:** restart after checkpoint reloads the same run cursor. **Invalid:** stale lease or duplicate step completion after new lease epoch → rejected; no Task terminal transition from the worker.

## C-03: Action–Attempt–Receipt

An ActionIntent is a proposal, not authority. Authority checks policy/grant and issues a permit for the intended capability, exact or scoped payload, relevant revision/preconditions and expiry. The gateway persists an attempt before dispatch to a real executor, then records a receipt. If dispatch occurred and the result is lost, the state is `Uncertain`; non-idempotent effects are reconciled rather than blindly retried. A synthetic JSON receipt without calling an executor cannot be marked Executed.

**Valid fixture:** an allowed, current permit produces one dispatched attempt and a real receipt tied to action/payload digest. **Invalid:** expired permit, mismatched digest, denied path, duplicate non-idempotent retry → zero new effects and a recorded denial or uncertainty.

## C-04: Criterion–Evidence

Criterion IDs are stable within a Task contract revision. An assessment references criterion ID, contract revision, subject/source version, evidence ID and verifier version. `Pass`, `Fail` and `Unknown` are distinct; a located source is not automatically support for a claim. Completion requires all required current criteria to Pass, required artifacts to exist and relevant effects to be resolved. A required waiver is an audited human contract revision, not an evaluator shortcut.

**Valid fixture:** exact span/hash and current subject revision support the claim. **Invalid:** forged hash, mismatched span, old source revision, unrelated criterion or unsupported causal claim → Fail/Unknown, never Pass.

## Pending decisions

Confirm canonical DTO home, owner of `custos-domain`, enum compatibility window, transaction boundaries across Session/Task, policy for external agent native tools, criterion coverage and evidence thresholds. These questions block cross-team schema PRs, but not isolated fixture or adapter research.
