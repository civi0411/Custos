# GPU Workload Components & Resource Admission (GPU-A / GPU-B)

> SADE Master Specification — Phase 12 / GPU Workload Components Blueprint  
> Governs: `custos-domain`, `custos-core`, `custos-runtime`, and Golden Acceptance Test `tests/e2e/tests/gpu_workload_admission_golden_test.rs`.

---

## 1. Context & Invariants

Autonomous research, coding, and multi-workbench operations consume heterogeneous compute accelerators: local Apple Silicon Metal (MPS), local NVIDIA GPUs (CUDA), dedicated SSH hosts, and on-demand cloud clusters.

Without a sovereign resource arbiter:
1. **Accelerated Contention & Crash (OOM):** An interactive notebook session and a background fine-tuning run (e.g. Unsloth/LoRA) launch concurrently on the same GPU, exhausting VRAM and triggering unhandled segmentation faults or CUDA OOM.
2. **Dual Launch on Exclusive Hardware:** Multiple workers simultaneously schedule training runs on the same physical accelerator, corrupting optimizer snapshots and interleaving forward/backward passes.
3. **Zombie Allocations on Crash:** An interactive notebook kernel or local inference server terminates or resets, but underlying GPU memory and lease registrations remain locked indefinitely.
4. **Silent Cloud Fallback Violations:** When local GPUs are saturated, naive systems silently route confidential tensors or training data to third-party cloud APIs without user consent.

Phase 12 (GPU-A / GPU-B) establishes sovereign compute admission, mutual exclusion, and clean lifecycle teardown:
> **GPU Admission Invariant:**  
> *"Two workloads claim the same managed exclusive device; one is rejected or queued — NO dual launch on exclusive accelerators."*

---

## 2. Core Entities & Layer Boundaries

Following the inviolable **Layer 0 Zero-I/O Invariant**, all domain representations in `custos-domain` remain 100% pure, deterministic, and free of runtime/I/O dependencies:

```
┌────────────────────────────────────────────────────────────────────────┐
│ Layer 0: Pure Domain (`custos-domain`)                                 │
│  - ComputeDeviceRef: Logical descriptor (CPU, CUDA, MPS, ROCm)         │
│  - ComputeWorkloadKind: NotebookKernel, TrainingRun, EvalRun, Infer    │
│  - WorkloadLease: Sovereign lease (lease_id, device_id, owner_id)      │
│  - WorkloadAdmissionRequest & WorkloadAdmissionVerdict                │
└────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Layer 1: Core Contracts (`custos-core`)                                │
│  - WorkloadAdmissionPort: discover_devices(), admit_workload(),         │
│    release_lease(), list_active_leases()                               │
└────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Layer 2: Runtime Arbitration (`custos-runtime`)                        │
│  - WorkloadAdmissionCoordinator: In-memory slot & queue arbitration    │
│  - PythonKernelCoordinator: Integrated device binding & probe          │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Mutual Exclusion & Anti-Dual-Launch Arbitration

```
                        [ Workload Admission Request ]
                                      │
                                      ▼
                      [ Resolve Target Device ID ]
                                      │
                                      ▼
                      [ Device Currently Free? ]
                             /                 \
                           YES                  NO
                           /                     \
      [ Require Exclusive? ]             [ Device Exclusively Held? ]
            /          \                          /              \
          YES           NO                      YES               NO
          /               \                     /                  \
[ Admit Exclusive ] [ Admit Shared ]     [ Queue / Reject ]  [ Request Exclusive? ]
                                        (Anti-Dual-Launch)       /             \
                                                               YES              NO
                                                               /                 \
                                                    [ Queue Request ]     [ Admit Shared ]
```

### Contention Rules:
1. **Training Run (Exclusive):** An admitted training job holds exclusive reservation on the target accelerator. All subsequent incoming requests targeting that GPU (whether training, eval, or notebook) are queued or rejected with explicit reasons.
2. **Notebook Execution:** Interactive kernel executions request shared capacity by default. If the user explicitly selects a device held by an active training run, execution fails-fast with `DomainError::Conflict`.
3. **Multi-Notebook Coexistence:** Independent notebook kernels (e.g. across two tabs or sessions) targeting shared devices (CPU or MPS) are admitted concurrently with isolated leases.
4. **Orphan & Crash Recovery:** When a notebook kernel process is reset (`reset_scoped`) or interrupted (`interrupt_scoped`), its device lease is immediately released. `sweep_orphaned_leases(owner_id)` safely cleans any abandoned allocations without leaking GPU VRAM handles.

---

## 4. Verification & Golden Test Coverage

The implementation is verified by Golden Acceptance Test [`tests/e2e/tests/gpu_workload_admission_golden_test.rs`](file:///Users/mac/Project/AgentHub/Custos/tests/e2e/tests/gpu_workload_admission_golden_test.rs) (6/6 PASS):
- `test_scenario_1_device_inventory_discovery`: CPU, MPS, and CUDA hardware profiling.
- `test_scenario_2_exclusive_mutual_exclusion_anti_dual_launch`: Zero duplicate runs on exclusive accelerators.
- `test_scenario_3_training_holds_gpu_blocks_notebook_execution`: Fail-closed collision avoidance between training and notebook execution.
- `test_scenario_4_multi_notebook_shared_coexistence`: Multi-notebook shared concurrent execution.
- `test_scenario_5_crash_reset_recovery_zero_zombies`: Zero zombie allocations on kernel reset and crash recovery.
- `test_scenario_6_bounded_diagnostic_probe`: 5-second bounded Python accelerator environment diagnostic.
