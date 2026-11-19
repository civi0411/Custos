# Supervised Experiment Execution & Checkpoint Recovery

> **Classification:** Research Subsystem Architecture Specification 
> **Source of Truth:** Authoritatively governed by [Custos Master Specification](../../Custos.md). 
> **Principles:** Durable Checkpointing, Supervised Execution, Transparent Honest Recovery.

---

## 1. Product Identity & Core Invariants

### 1.1 Purpose
**Supervised Experiment Execution & Recovery** coordinates scientific experiments and machine learning model training across distributed accelerators:
1. **Preflight & Environment:** Validate dependencies, VRAM, and storage credentials.
2. **Submit & Allocate:** Provision compute instances on Colab, Kaggle, or SSH GPU fleets.
3. **Observe & Reconcile:** Audit telemetry streams and handle intermittent network drops.
4. **Durable Checkpoint Commitment:** Atomic multi-phase commit of optimizer states and model weights.
5. **Recovery & Handoff:** Resume execution on failover targets from verified checkpoints.

The system serves three decoupled execution lifecycles:
- **Interactive Notebook:** Data exploration, cell execution, plotting, and iterative debugging.
- **Background Experiment / Training:** Long-running batch executions identified by stable `job_id`, metric telemetry, artifacts, and held-out evaluation.
- **Recovery / Handoff:** Launching a new `Attempt` linked to a parent checkpoint when the source node is interrupted or migrated.

> [!IMPORTANT]
> An interactive notebook may trigger background training runs, but long-running training jobs **must never terminate when a notebook tab or desktop window closes**.

### 1.2 Invalidation of Fragile Assumptions
- **No Live VRAM Memory Migration:** Custos guarantees safe, reproducible resumption from the latest verified external checkpoint (`latest_protected_step`), not process state cloning across GPU architectures.
- **Storage-Level Fencing:** Incrementing generation numbers locally in SQLite is insufficient; fencing must be enforced at the storage prefix/namespace level to prevent split-brain writes.
- **Integrity vs. Provenance:** SHA-256 digests prove byte integrity, not cryptographic creator provenance.
- **Zero Heavy Tensor Streaming Through Desktop:** Large multithreaded checkpoint bundles (gigabytes of weights) upload directly from the compute node to remote storage, never passing through the desktop UI memory.

---

## 2. Separation of Concerns & Architecture Layering

```
Research UI / Notebook / Agent (apps/desktop/ui)
 
 SDK + Local API (sdk/protocol, sdk/client-rust)
 
 Daemon Composition Root (apps/daemon)
 
Authority / Budget / Admission (platform/engine/src/authority/)
 
Runtime Experiment Coordinator (platform/engine/src/orchestration/)
 Compute Target Adapter (platform/integrations/src/compute/)
 Training Engine Adapter (workbenches/research/src/)
 Checkpoint Storage Port (platform/storage/src/)
 Shared Observation Pipeline
 
 Relational Tables & CAS Artifacts (platform/storage)
```

- **Supervision Nature:** Supervision is implemented via state machines and policy contracts, **not** via continuous LLM polling. S1 agents only explain logs or diagnose errors; they never self-grant budget or self-certify completions.

---

## 3. Seven Discrete Operational Phases

All compute adapters must strictly distinguish the following 7 lifecycle operations:

| Operation | Technical Semantics |
|---|---|
| **1. Preflight** | Verify engine compatibility, credentials, dataset paths, storage, and budget limits. |
| **2. Submit** | Request attempt creation on target backend; success does not imply immediate running state. |
| **3. Observe / Reconcile** | Query actual status for specific `job_id` / `attempt_id`; reconcile upon disconnection. |
| **4. Request Checkpoint** | Signal engine to take a clean snapshot at the next safe optimizer-step boundary. |
| **5. Stop Job** | Terminate the training process for the specified execution attempt. |
| **6. Release Compute** | Deprovision VM, pod, or GPU allocation to stop billing. |
| **7. Collect** | Gather verified artifact references, metrics, and checkpoint manifests. |

> [!WARNING]
> **Stop Job and Release Compute are separate operations.** Halting training execution does not release compute resources; stream disconnection does not imply remote instances have terminated.

---

## 4. Two Lifecycles & Three Progress Counters

### 4.1 Independent Lifecycles
1. **Invocation Lifecycle:** The transient HTTP or CLI request to the target backend.
2. **Execution Attempt Lifecycle:** The durable running job on the accelerator node, tracked via `attempt_id` linked to account, target, recipe revision, and parent checkpoint.

### 4.2 Three Progress Counters
The user interface renders three distinct progress counters without synthetic aggregation:
- **`Latest Observed Step`:** Most recent training step observed via telemetry logs.
- **`Latest Locally Saved Step`:** Most recent step committed to the compute node's local disk.
- **`Latest Protected Step`:** Most recent step uploaded, hashed, and verified in external durable storage.

---

## 5. Four Resume Tiers

| Resume Tier | Classification | Required Assets | Technical Meaning |
|---|---|---|---|
| **Tier 1** | **State Resume Supported** | Full weights + optimizer + scheduler + engine state + data cursor + RNG | Resumes training sequence on verified hardware profile |
| **Tier 2** | **Adapted Continuation** | Recipe or hardware altered (e.g. lower VRAM requires micro-batch adjustment) | Resumes execution, but optimizer dynamics may vary; tracked as adapted lineage |
| **Tier 3** | **Weights-Only Warm Start** | Model weights / LoRA adapter only, missing optimizer or scheduler states | Initializes a new training lineage; resets learning rate and data epoch |
| **Tier 4** | **Unsupported / Unknown** | Missing required files, incompatible schema, or unverified engine | Rejects continuation to prevent wasted compute spend |

---

## 6. Target Handoff Protocols

### 6.1 Planned Handoff
1. Preflight destination quotas, credentials, and hardware compatibility.
2. Signal source engine to take a clean snapshot at the next optimizer boundary.
3. Upload and verify checkpoint bundle in external durable storage.
4. Quiesce source execution process.
5. Revoke checkpoint promotion rights from source attempt.
6. Spawn destination attempt linked to parent checkpoint ID.
7. Destination verifies hashes and pre-loads checkpoint state.
8. Monitor destination telemetry until normal running progress is observed.
9. Deprovision and release source compute resources.

### 6.2 Unplanned Interruption Recovery
1. Detect heartbeat loss or timeout; mark attempt status as `Uncertain`.
2. Reconcile with provider API to verify if the node died or network disconnected.
3. Select the most recent checkpoint verified as `CommittedProtected` (`latest_protected_step`).
4. Avoid overwriting output directories; create an isolated recovery attempt on a new namespace.

---

## 7. Subsystem Mapping & Physical Owners

| Responsibility | Defining Crate | Physical Path | Key Invariant |
|---|---|---|---|
| Recipe, Checkpoint & Attempt Records | `custos-contracts` | `platform/contracts/src/execution/` | Pure domain types, Zero-I/O |
| Authority, Budget, Admission & Contracts | `custos-contracts` & `custos-engine` | `platform/engine/src/authority/` | Two-phase mediation, signed permits |
| Experiment & Recovery Coordinator | `custos-engine` | `platform/engine/src/orchestration/` | Deterministic FSM, timeout reconciliation |
| Target CLI/API Compute Adapters | `custos-integrations` | `platform/integrations/src/compute/` | Colab, Kaggle, SSH GPU fleet drivers |
| Checkpoint Hashing & Verification | `custos-integrations` | `platform/integrations/src/compute/checkpoint.rs` | Bounded SHA-256 validation |
| Canonical Records & Outbox Migrations | `custos-storage` | `platform/storage/src/` | SQLite WAL single-writer transactions |
| Application Use Cases & Local API | `custos-daemon` | `apps/daemon/src/` | Thin transport adapter |
| Desktop UI & Progress Transparency | `apps/desktop/ui` | `apps/desktop/ui/src/` | Strict zero-mock data integrity |

---

## 8. Automated Verification Commands

```bash
# 1. Verify compute and execution contract tests
cargo test --package tests-contract --lib --offline

# 2. Verify platform engine state machines
cargo test --package custos-engine --lib --offline

# 3. Verify architecture boundary compliance
cargo xtask check-boundaries
```
