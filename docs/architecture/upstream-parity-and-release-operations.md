# Upstream Parity, Release Upgrade & Operational Recovery (F12)

> SADE Master Specification — Phase 11 / F12 Architectural Blueprint  
> Governs: `custos-domain`, `custos-persistence`, `custos-runtime`, `custos-adapters`, and Golden Acceptance Test `G13`.

---

## 1. Context & Invariants

Autonomous agent systems operating across local workstations and remote cloud clusters encounter frequent restarts, daemon crashes, and version upgrades. In naive frameworks:
1. **Destructive Cleanups:** Startup scripts frequently delete unverified state, purge local worktrees, or drop database records, destroying user work in progress.
2. **Duplicate Remote Resource Allocation:** Crashes during in-flight cloud tasks often cause daemons to forget remote pod IDs, launching duplicate expensive GPU instances while leaking orphaned compute.
3. **Automated Blind Retries (INV-03 Violation):** In-flight outbox effects (such as payments, email dispatches, or git pushes) interrupted mid-flight are naively re-executed on restart, causing duplicate side-effects.
4. **Data Loss During Schema Upgrades:** Releasing a new binary version causes migration failures or alters legacy schemas, rendering older task journals and historical runs unreadable.

Phase 11 (F12) establishes sovereign durability and operational recovery anchored by Golden Acceptance Test `G13`:
> **G13 Operational Invariant:**  
> *"Release upgrade/restart/orphan resource recovery | Legacy data readable, active jobs discoverable, no destructive cleanup."*

---

## 2. Upstream Parity Capability Matrix

Custos provides full architectural parity with existing state-of-the-art agent and research frameworks while adding strict Zero-I/O domain guarantees and sovereign fencing:

| Capability Dimension | Upstream Reference | Custos Architectural Implementation | Guarantees & Verification |
|---|---|---|---|
| **Sovereign Dispatch Claims** | OrCa / Slurm | `DispatchClaim` CAS transitions (`Pending` -> `Dispatched` -> `Released`) | Node and task fencing prevents split-brain execution across multiple daemons. |
| **Outbox Effect Fencing** | OrCa (Gate 3) | `OutboxEntry` + `EffectAttempt` with `idempotency_key` and `OutboxStatus::Uncertain` | Invariant INV-03: Never blindly retry in-flight side-effects after a restart. |
| **Reproducible Research & Notebooks** | Open Science / Jupyter | Notebook binding with reactive cell hash invalidation (`notebook_cells`) | Cell source edits invalidate stale outputs and bump state versions monotonically. |
| **Supervised Checkpoint Lineage** | PyTorch Trainer / HuggingFace | Two-axis execution attempt (`ExecState`, `ResourceState`) + `checkpoint_promotions` | Incomplete/uncommitted checkpoints block state resume; zombie compute fenced. |
| **Non-Destructive Dirty Worktrees** | Goose / Git Worktree | `WorkspaceCoordinator` + `LocalWorkspaceProvider::inspect_dirty` | Uncommitted user changes block physical deletion without explicit force consent. |
| **Forward Migration Durability** | SQLite WAL / Flyway | 41 idempotent, monotonic migrations (`0001_core` -> `0041_...`) | Legacy data created on early schema versions remains 100% intact and readable. |

---

## 3. Crash & Startup Reconciliation State Machine

When `custos-daemon` boots or recovers from an unexpected termination, it triggers unified reconciliation via `SqliteTaskStore::reconcile_all_on_startup()`.

```
                    [ Unexpected Crash / Daemon Boot ]
                                    │
                                    ▼
                    [ reconcile_all_on_startup() ]
                                    │
    ┌────────────────┬──────────────┴───────────────┬────────────────┐
    ▼                ▼                              ▼                ▼
[ Remote Runs ]  [ Local Runs ]                [ Outbox Entries ] [ Experiment Attempts ]
    │                │                              │                │
Has external_ref     Pure local execution           Status:          ExecState:
or cloud backend?    No cloud handles               `dispatching`    in-flight
    │                │                              │                │
    ▼                ▼                              ▼                ▼
`Uncertain`      `Failed`                       `Uncertain`      `Uncertain`
(Preserves pod   (Records ended_at,             (Fences blind    (Releases fenced,
 handle for live  bumps state_version)           retry per        preserves parent
 observation)                                    INV-03)          checkpoint ref)
```

### Detailed Component Transitions

1. **Remote Runs & Remote Worker Runs:**
   - Any active run with `metadata.external_job_ref`, cloud backend (`runpod`, `vast_ai`, `dedicated_ssh`, `colab`, `kaggle`), or an active dispatch claim with an assigned node transitions to `RunStatus::Uncertain`.
   - Associated worker runs also transition to `RunStatus::Uncertain`.
   - **Rationale:** Live cloud compute pods continue running independently in the cloud; marking them failed or deleting records would leak billed GPU instances or trigger duplicate pod spins.
2. **Local Worker Runs & Local Runs:**
   - Any active local run or worker run without remote backend metadata transitions fail-closed to `RunStatus::Failed`.
   - `ended_at` is stamped with the restart timestamp, and `state_version` is incremented.
   - **Rationale:** The daemon process hosting local execution died; the local subprocesses cannot have survived without their host supervisor.
3. **Outbox Side-Effects (Gate 3 / Invariant INV-03):**
   - In-flight outbox entries in `dispatching` state transition to `OutboxStatus::Uncertain`.
   - In-flight `effect_attempts` transition to `EffectStatus::Uncertain`.
   - **Rationale:** Whether the external third-party API (e.g. payment gateway, email server, deployment trigger) received and processed the packet is unknown. Blind re-execution is strictly forbidden.
4. **Experiment Attempts & Compute Resources:**
   - In-flight attempts in `submitting`, `submitted`, `running`, or `stopping` transition to `ExecState::Uncertain`.
   - Resources with pending release requests (`ReleaseRequested`) transition to `ResourceState::Unknown`.
   - Fencing generation checks prevent zombie worker processes from writing back to SQLite.
5. **Prepared Launch Attempts:**
   - Launch attempts left in `prepared` state transition to `LaunchStatus::Unknown` with detail `Daemon restart during in-flight execution; recovered fail-closed`.
6. **Orphan Dispatch Claims:**
   - Claims in `pending` or `dispatched` state lacking a valid `node_id` transition to `ClaimStatus::Released`.

---

## 4. Active Job Discoverability & Auditability

Recovery is strictly non-destructive. No records are purged or truncated from SQLite during reconciliation:

1. **Uncertain Run Discovery:**
   - All runs transitioned to `Uncertain` are discoverable via `SqliteTaskStore::list_uncertain_runs()`.
   - Operators and automated supervisors can query external cloud providers using the preserved `external_job_ref` and `backend` metadata to reconcile status (`Completed`, `Failed`, or running).
2. **Task & Session Lineage:**
   - Standard task queries (`list_runs_for_task()`, `get_task()`, `get_session()`) return complete historical execution graphs.
   - Audit logs, event journals, and budget ledger records remain intact for accounting and auditing.

---

## 5. Workspace Orphan & Host Recovery with Dirty Protection

Physical execution workspaces are decoupled from ephemeral daemon lifetimes:

### A. Healthy Host Directory Recovery
When a workspace points to an intact host folder:
- `WorkspaceCoordinator::recover_workspace()` verifies path existence and permissions.
- Status transitions from `Initializing` or stale states to `WorkspaceStatus::Ready`.

### B. Missing or Corrupt Host Directory Recovery
When a workspace points to a missing, unmounted, or deleted host directory:
- Custos **does not delete** the SQLite workspace metadata.
- Status transitions safely to `WorkspaceStatus::SetupFailed { reason: "Physical directory is missing or corrupt: ..." }`.
- Lineage, metadata, and owner task associations remain available for inspection or manual path remapping.

### C. Dirty Worktree Protection (Non-Destructive Deletion)
When deleting or archiving a physical workspace:
```rust
coordinator.archive_workspace_with_force(&workspace_id, delete_physical, force).await
```
- If `delete_physical = true` and `force = false`, `LocalWorkspaceProvider::inspect_dirty()` executes a porcelain git status inspection.
- If uncommitted modifications, untracked files, or deleted files exist, the coordinator returns:
  ```rust
  DomainError::Conflict("Workspace '...' contains uncommitted changes. Set force=true to discard.")
  ```
- User bytes on disk are preserved against accidental data destruction.

---

## 6. Forward Migration Guarantee

All migrations (0001 through 0041) are idempotent, forward-compatible, and transactional:
- Opening a legacy database created on early releases runs pending migrations without dropping historical tables (`tasks`, `sessions`, `continuation_packets`, `spans`).
- Verified by Scenario 1 of `G13`, guaranteeing continuous upgrade paths for long-lived installations.
