# GPU Training, Evaluation, and Inference Coexistence Architecture (GPU-C / GPU-D)

> **Status:** Canonical Architecture SSOT  
> **Layer:** Layer 0 (`custos-domain`), Layer 1 (`custos-core`), Layer 2 (`custos-runtime`), Layer 3 (`custos-adapters`)  
> **Reference Plan:** `docs/development/gpu-workload-components-plan.md` (§6, §7, §8, §10, §17)  
> **Golden Verification Suite:** `tests/e2e/tests/gpu_training_eval_coexistence_golden_test.rs` (7/7 PASS)

---

## 1. Architectural Principles & Invariants

Heterogeneous hardware acceleration requires deterministic, cooperative resource arbitration across four distinct workload classes:
1. **Interactive Notebook Kernels (`ComputeWorkloadKind::NotebookKernel`)**
2. **Supervised Training Runs (`ComputeWorkloadKind::TrainingRun`)**
3. **Model Evaluation & Validation Runs (`ComputeWorkloadKind::EvaluationRun`)**
4. **Local Reasoning Inference (`ComputeWorkloadKind::LocalInference`)**

```
+───────────────────────────────────────────────────────────────────────────────────+
|                        WORKLOAD ADMISSION COORDINATOR                             |
|                                                                                   |
|  [Notebook Kernels]     [Training Runner]     [Eval Runner]    [Local Inference]  |
|         │                      │                    │                  │          |
|         │ (Shared/Exclusive)   │ (Exclusive)        │ (Sequential)     │ (Shared) |
|         ▼                      ▼                    ▼                  ▼          |
|  ┌─────────────────────────────────────────────────────────────────────────────┐  |
|  │                        HARDWARE DEVICE ARBITRATION                           │  |
|  │                                                                             │  |
|  │  dev_cpu_0          dev_mps_0 (Apple Metal)      dev_cuda_0 (NVIDIA RTX/A100)│  |
|  │  [Shared Leases]    [Concurrent Notebooks]       [Mutual Exclusion Enforced]│  |
|  └─────────────────────────────────────────────────────────────────────────────┘  |
+───────────────────────────────────────────────────────────────────────────────────+
```

### Inviolable Invariants:
1. **Anti-Dual-Launch & Exclusive Mutual Exclusion:**
   - Any workload claiming an exclusive compute lease (`require_exclusive: true`, such as a supervised fine-tuning or training run) prohibits any other workload from obtaining a lease on that physical accelerator.
   - Competing workloads fail closed with `DomainError::Conflict` or report `WorkloadAdmissionVerdict::Queued` with exact owner attribution.
2. **Inter-Workload Collision Avoidance (Training vs. Evaluation):**
   - When training occupies an accelerator, evaluation runners cannot spawn duplicate weights or optimizer contexts into the same device.
   - Evaluation runs sequentially at protected checkpoint boundaries or targets an independent admitted compute device.
3. **Fail-Closed Contention vs. Silent Cloud Fallback:**
   - When a requested local accelerator is occupied by training, incoming local inference queries must queue or fail closed with explicit reason.
   - Silently falling back to paid cloud providers, changing accounts, or terminating training to serve chat is strictly prohibited.
4. **Eviction Race Protection:**
   - Loaded model instances track in-flight requests via atomic counters.
   - An eviction request (`evict_model`) received while requests are in-flight fails closed immediately, preventing segfaults and GPU context corruption.
5. **Zero Zombie Leases on Fault:**
   - On process crash, kernel reset, simulated OOM, or cancellation, all associated hardware leases are synchronously reclaimed by `release_lease()` or swept by `sweep_orphaned_leases()`.

---

## 2. Component Structure

### 2.1 Pure Domain Models (`crates/custos-domain/src/research/experiment.rs`)
- `FrozenTrainingRecipe`: Immutable launch spec containing model, dataset URI, split, learning rate, batch size, and deterministic SHA-256 content digest.
- `TrainingStepTelemetry`: Monotonic step index, epoch, loss, learning rate, and memory usage.
- `TrainingExecutionReceipt`: Terminal receipt detailing total steps, final loss, created checkpoints, and final execution state.
- `ModelEvaluationSpec`: Target candidate checkpoint ID, held-out dataset URI, rubric version, sample limit, and target device.
- `ModelEvaluationReport`: Scored report capturing sample count, pass/fail counts, accuracy, loss, and `passed_hurdle` boolean.

### 2.2 Local Training Runner (`crates/custos-runtime/src/compute/training.rs`)
- Coordinates the lifecycle: `Planned` $\to$ `Preflighted` $\to$ `Submitting` $\to$ `Submitted` $\to$ `Running` $\to$ `Completed` $\to$ `Collected`.
- Demands exclusive admission before launching training steps.
- Produces valid `TrainingCheckpointManifest` records with `CheckpointCommitStatus::CommittedProtected` at scheduled step boundaries.
- Reclaims the compute lease unconditionally upon completion, early cancellation, or simulated OOM.

### 2.3 Model Evaluation Runner (`crates/custos-runtime/src/compute/evaluation.rs`)
- Coordinates evaluation against held-out validation datasets.
- Rejects parallel model loading when the target accelerator is held by an active training run.
- Computes accuracy and loss metrics, releasing the lease immediately upon completion.

### 2.4 Local Inference Coexistence Seam (`crates/custos-adapters/src/local_inference/mod.rs`)
- Integrates `InferenceRuntime` with `WorkloadAdmissionPort`.
- Checks admission before loading model weights into hardware backends (`llama.cpp` / `MLX`).
- Tracks active requests on `ModelSlot` via `active_requests: AtomicUsize` and RAII `InFlightGuard`.
- Rejects eviction during in-flight generation and releases the hardware lease upon confirmed unload.

---

## 3. Verification Matrix

| Scenario | Invariant Validated | Result |
|---|---|---|
| Scenario 1: Tiny CPU Training Run | Monotonic steps, valid checkpoint manifests, zero zombie leases | **PASS** |
| Scenario 2: Anti-Dual-Launch | Exclusive mutual exclusion, competing training run queued | **PASS** |
| Scenario 3: Training vs. Eval Collision | Eval blocked from spawning duplicate model into occupied VRAM | **PASS** |
| Scenario 4: Sequential Safe-Point Handoff | Training completes $\to$ checkpoint committed $\to$ eval succeeds | **PASS** |
| Scenario 5: Inference Coexistence | Inference blocked fail-closed when GPU held by training; no cloud fallback | **PASS** |
| Scenario 6: Eviction Race Protection | In-flight request guards against eviction race; clean release once drained | **PASS** |
| Scenario 7: Fault & OOM Recovery | Simulated OOM immediately reclaims lease, zero zombie allocations | **PASS** |
