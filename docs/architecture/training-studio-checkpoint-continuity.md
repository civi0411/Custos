# Training Studio, Remote Targets & Checkpoint Continuity (F10)

> SADE Master Specification — Phase 9 / F10 Architectural Blueprint
> Governs: `custos-domain`, `custos-core`, `custos-adapters`, `custos-persistence`, and Golden Acceptance Test `G11`.

---

## 1. Context & Motivation

Remote training and fine-tuning across heterogeneous GPU backends (RunPod, Vast.ai, Dedicated SSH, Colab, Kaggle) present unique architectural challenges:
1. **Worker Crashes & Compute Outages:** Long-running training jobs on spot or on-demand instances frequently terminate abruptly due to preemption, out-of-memory (OOM), or network partitioning.
2. **Zombie Writers & Split-Brain:** A crashed or lagging worker that resumes unexpectedly can attempt to push stale checkpoints, overwriting newer, valid training progress.
3. **Unsafe Checkpoint Resumption:** Attempting to resume training from an incomplete, uncommitted, or corrupt checkpoint causes model divergence, wasted compute budgets, or silent NaN loss propagation.
4. **Duplicate Remote Invocations:** Submitting identical training jobs repeatedly without idempotent deduplication causes duplicate pod provisioning and double-billing.

Phase 9 (F10) establishes deterministic checkpoint lineage, single-writer generation-CAS promotion fencing, startup crash reconciliation to Uncertain, and fail-closed pre-resume validation.

---

## 2. Core Architecture & Workflow

```
[ Local SADE Daemon / Client ]
            │
            ▼
┌────────────────────────────────────────────────────────┐
│ ExperimentAttemptRepository (custos-persistence)       │
│                                                        │
│ 1. Atomic attempt persistence + outbox event           │
│ 2. Idempotent Invocations (unique idempotency_key)    │
│ 3. Generation CAS Fencing on checkpoint promotion      │
│ 4. reconcile_attempts_on_startup() -> Uncertain        │
└────────────────────────────────────────────────────────┘
            │
            ▼
┌────────────────────────────────────────────────────────┐
│ Pre-Resume Gate (custos-domain)                        │
│                                                        │
│ TrainingCheckpointManifest::validate_for_state_resume()│
│ - Require commit_status: CommittedProtected            │
│ - Require resumability_level: TrainingStateResume      │
│ - Require complete file bundle:                        │
│     * model/adapter safetensors                        │
│     * optimizer.pt, scheduler.pt                       │
│     * rng_state.pth, trainer_state.json                │
│ - Reject path traversal (..) & digest mismatches       │
└────────────────────────────────────────────────────────┘
            │
            ▼
┌────────────────────────────────────────────────────────┐
│ Remote Target Dispatch & Handoff                       │
│                                                        │
│ Target A (RunPod, Gen 1) ──► Checkpoint 1 (Gen 1)       │
│                                      │                 │
│                                      ▼ Lineage Handoff  │
│ Target B (SSH / Vast, Gen 2) ──► Checkpoint 2 (Gen 2)  │
│                                                        │
│ Stale Target A attempts Gen 1 promotion                │
│ ──► REJECTED: PromotionOutcome::StaleGeneration(2)    │
└────────────────────────────────────────────────────────┘
```

---

## 3. Invariant Specification

### 3.1 Immutable Lineage & Cross-Target Continuation (G11)
- Every training execution is modeled as an `ExecutionAttempt` with an immutable `experiment_id`, `recipe_id`, and `backend` (`ComputeBackendKind`).
- When training transitions across targets (e.g. from a spot instance on RunPod to a dedicated server via SSH), the new attempt references the parent checkpoint via `parent_checkpoint_id` and increments the fencing `generation`.
- The entire attempt chain is retrievable via `list_attempts_for_experiment()`, ensuring full scientific provenance and reproducibility.

### 3.2 Crash Recovery & Startup Reconciliation
- When the daemon starts up or restarts following an ungraceful shutdown, `reconcile_attempts_on_startup()` inspects all attempts.
- Attempts in in-flight states (`submitting`, `submitted`, `running`, `stopping`) are transitioned to `ExecState::Uncertain`.
- Resources in `ReleaseRequested` state are transitioned to `ResourceState::Unknown`.
- **Zero Blind Retries:** Uncertain attempts are never blindly re-dispatched. They require explicit operator reconciliation, eliminating duplicate GPU lease creation and runaway billing.

### 3.3 Generation CAS Fencing & Zombie Writer Defense
- Checkpoint promotions are serialized in the `checkpoint_promotions` table with primary key `(experiment_id, generation)`.
- When Attempt 2 promotes a checkpoint at `generation: 2`, any delayed or rogue request from Attempt 1 (`generation: 1`) is rejected with:
  ```rust
  PromotionOutcome::StaleGeneration { actual_generation: 2 }
  ```
- This guarantees single-writer semantics and prevents split-brain checkpoint corruption across distributed environments.

### 3.4 Pre-Resume Validation (Fail-Closed Gate)
- Calling `validate_for_state_resume()` verifies that:
  - `commit_status == CheckpointCommitStatus::CommittedProtected` (uncommitted `LocalStaged` or `Observed` checkpoints are blocked).
  - All essential Trainer state files are present and inventoried with valid 64-character hex digests.
  - No file paths contain directory traversal sequences (`..`), colons, or backslashes.

### 3.5 Idempotent Remote Invocations
- Remote compute operations are recorded via `record_invocation()`.
- Replay attempts using an existing `idempotency_key` are rejected with `DomainError::Conflict`, protecting external cloud provider APIs from double-spawning.

---

## 4. Test Verification Protocol

Execute the Golden Acceptance Test G11 suite:
```bash
cargo test -p custos-tests-e2e --test training_vertical_golden_test
```
