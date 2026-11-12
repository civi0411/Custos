# Run Lifecycle & Honest Recovery Specification

> **Scope**: `platform/domain/src/task/run/`, `platform/storage/src/repositories/run/`, `apps/daemon/src/runtime.rs`  
> **Classification**: Architecture Reference  
> **Status**: Canonical Specification (Phase 1 / F04)

---

## 1. Run Status State Machine

Execution runs for Tasks and WorkerRuns follow a state machine:

```
        Pending
           │
           ▼
        Active ───────► Suspended
           │                 │
           ├───► Uncertain   │
           │        │        │
           ▼        ▼        ▼
       Completed / Failed / Cancelled (Terminal)
```

### The `Uncertain` Status
`Uncertain` indicates that the local daemon lost contact with the running process or remote worker (e.g. following a daemon crash or power loss).
- `is_terminal()` evaluates to `false` for `Uncertain`.
- Never resubmit an `Uncertain` job blindly to prevent double-spending or duplicate worker conflicts.

---

## 2. Startup Reconciliation & Periodic Polling

### 2.1 Partitioned Startup Reconciliation
When the daemon restarts, `reconcile_runs_on_startup()` partitions active runs:
1. **Local Runs (`external_job_ref IS NULL`)**: Local processes died with the daemon; marked safely as `Failed`.
2. **Remote Runs (`external_job_ref IS NOT NULL`)**: Remote GPU pods or batch kernels may still be executing on the cloud target; marked as `Uncertain`.

### 2.2 Background Reconciler Task
The daemon spawns a dedicated background task running every 60 seconds:
- Queries all runs with `RunStatus::Uncertain`.
- If an uncertain run exceeds the maximum expiration window (24 hours), it is resolved to `Failed`.
- For compute attempts, observations update the attempt state machine via `ReconciledRunning`, `ReconciledCompleted`, or `ReconciledFailed`.

---

## 3. Multi-Candidate Race & Scoped Claim Fencing

To allow concurrent multi-model races (e.g. Claude, GPT, DeepSeek) on candidate worktrees for the same Task without mutual lockouts:
1. `StartRunCommand` supplies `node_id: Option<String>` (e.g. `Some("candidate:<candidate_id>")`).
2. `WorkflowDispatcher::claim_ready_task` scopes the active claim index key to `"{task_id}::{node_id}"`.
3. Whole-task runs (`node_id == None`) continue to enforce single active worker run fencing per task.
4. Candidate runs (`node_id.is_some()`) enforce mutual exclusion strictly per candidate ID, enabling concurrent execution in isolated worktrees.
5. `CandidateRaceSpec` defines race parameters (`task_id`, `base_workspace_id`, `candidates`, `test_command`, `timeout_seconds`).
6. `CandidateRaceResult` captures the comparative scorecard (`CandidateExecutionReport`) across candidates and identifies the winning model.
7. `v1.workspaces.candidates.race` coordinates multi-model execution and Test Oracle evaluation, returning ranked verdicts for 1-click winner integration.
