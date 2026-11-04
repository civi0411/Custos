# Remote Compute Adapters & Mock Mode Specification

> **Scope**: `crates/custos-adapters/src/compute/` (Colab, Kaggle, Rented GPU / SSH / Cloud Pods)  
> **Classification**: Architecture Reference  
> **Status**: Canonical Specification (Phase 2 / F06)

---

## 1. Core Principle: Fail-Closed Production Execution

Remote compute runtimes incur financial billing and allocate scarce GPU resources. Under no circumstance may a production adapter execute placeholder or stub code on a live target.

```
                  ┌────────────────────────┐
                  │ submit(attempt, key)   │
                  └───────────┬────────────┘
                              │
               Is self.mock_mode == true?
                              │
               ├─── YES ──────┴────── NO ───┐
               ▼                            ▼
      Write mock script            Check entrypoint/recipe
      Return simulated job ref              │
                                   ├─── VALID ─────────── INVALID ───┐
                                   ▼                                 ▼
                         Launch real job on host         Return DomainError::Validation
```

### 1.1 Invariants
1. **No Dummy Code in Production**: In live mode (`mock_mode == false`), all compute adapters (`ColabTargetAdapter`, `KaggleTargetAdapter`, `RentedGpuTargetAdapter`) MUST verify the existence of the execution script or recipe prior to execution. Missing artifacts fail with `DomainError::Validation`.
2. **Upfront Fail-Fast Validation**: Host format, username safety, and recipe existence must be validated prior to establishing any remote network connection or SSH channel.
3. **Pinned Versions**: Remote jobs must reference immutable external versions (`v1`, `pid_<N>`, pinned kernel version). The identifier `"latest"` is strictly forbidden as it permits mutable aliasing.
4. **Decoupled Stop vs. Release**:
   - `stop_job()` terminates the training process but leaves the GPU runtime allocated (`ResourceState::Held`) for inspection and artifact collection.
   - `release_compute()` deallocates the underlying cloud pod / kernel (`ResourceState::ReleaseConfirmed`) to halt billing.

---

## 2. Adapter Specific Contracts

### 2.1 Google Colab (`colab.rs`)
- Uses `google-colab-cli` to govern Colab hosted runtimes.
- Staging directory: `<work_dir>/colab/<attempt_id>/run.py`.
- If `run.py` does not exist in live mode, submission aborts before CLI execution.

### 2.2 Kaggle Kernels (`kaggle.rs`)
- Uses Kaggle CLI / API (`kaggle kernels push`, `kaggle kernels status`).
- Staging directory: `<work_dir>/kaggle/<attempt_id>/`.
- Requires `kernel-metadata.json` with pinned versioning.
- Live mode requires verified credentials (`KAGGLE_USERNAME` and `KAGGLE_KEY` or `~/.kaggle/kaggle.json`).

### 2.3 Rented GPU & Dedicated SSH (`rented_gpu.rs`)
- Supports RunPod, Vast.ai, and Dedicated SSH hosts.
- Enforces strict input validation on hostnames and usernames to prevent shell injection.
- Validates `recipe_id` before invoking SSH `mkdir` or background script execution.
