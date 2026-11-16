# Testing Architecture, Acceptance Gates & Verification Standards

> **Classification:** Normative Quality Assurance Specification  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Developer Hub](README.md).

Custos enforces a rigorous, multi-tiered testing discipline designed to guarantee the correctness, durability, and safety of an autonomous runtime that interacts with and modifies production codebases. Every line of code merged into the monorepo must withstand an exhaustive battery of deterministic tests, chaos injection, invariant verifications, and compile-time gates.

---

## 1. The Five-Tier Testing Pyramid

```text
              ▲
             / \     [ Tier 1: Live Provider Canaries ] (~2% of suites)
            /   \    [ Tier 2: E2E Repo Fixture Tests ] (~8% of suites)
           /     \   [ Tier 3: Chaos & Crash Matrix   ] (~15% of suites)
          /       \  [ Tier 4: Contract & Integration ] (~25% of suites)
         /         \ [ Tier 5: Pure Unit & Property   ] (~50% of suites)
        ─────────────
```

1. **Pure Unit & Property Tests:** Fast, in-memory validation of domain entities, task state machine transitions, path normalization invariants, and budget accounting algorithms using `proptest`.
2. **Contract & Integration Tests:** Verification of wire schemas, SQLite database migrations, and adapter traits against deterministic test fixtures (`tests/contract`). Must be executed with `--lib --tests` to prevent Darwin dyld hanging on doc-tests.
3. **Chaos & Crash Matrix:** Injection of simulated SIGKILL power loss, socket abrupt terminations, and full-disk conditions to test the recovery of the SQLite WAL and Git worktrees.
4. **E2E Repo Fixture Tests:** Multi-process daemon and CLI runs on offline canonical repositories (`tests/e2e`), testing end-to-end task decomposition, patch application, and evidence verification.
5. **Live Provider Canaries:** Periodic, budget-capped executions against real upstream vendor APIs (Anthropic, OpenAI, Gemini) to detect upstream protocol drifts early.

---

## 2. Automated Continuous Quality Gates (`cargo xtask gate`)

All changes must satisfy the 5 continuous gates executed by `cargo xtask gate`:

| Gate | Name | Command & Mechanism | Purpose |
|---|---|---|---|
| **Gate 1** | **Manifests & Descriptors** | `xtask check-boundaries` | Validates all 4 workbench manifests (`Assistant`, `Coding`, `Research`, `Quant`) and 8 capability descriptors. |
| **Gate 2** | **Desktop UI Build** | `npm --prefix ui/desktop run build` | Enforces TypeScript strict typechecking and Vite production asset bundling. |
| **Gate 3** | **Canvas Smoke Invariants** | `node ui/desktop/scripts/canvas-smoke.mjs` | Executes 51 rigorous headless browser invariants verifying DOM stability, canvas rendering, and zero mock leaks. |
| **Gate 4** | **Architecture Invariants** | `cargo test -p custos-testkit -p custos-tests-contract --lib --tests` | Enforces naming conventions, layering rules, dependency graphs, and the $\le 600$ line file ceiling. |
| **Gate 5** | **Workspace Cargo Check** | `cargo check --workspace --all-targets --offline` | Validates compilation across all 20 packages in the workspace. |

---

## 3. The Eight Product Acceptance Gates

Before any architectural change or feature is merged, it must satisfy the 8 Product Acceptance Gates authoritatively defined in `Custos.md` (Part 18):

| Acceptance Gate | Gate Name | Core Verification Requirement | Target Package / Suite |
|---|---|---|---|
| **GATE A** | **Local Process E2E** | Run from CLI through Daemon, invoking a local model to explain a repo under the `LocalOnly` flag. | `tests/e2e` |
| **GATE B** | **Worktree Isolation** | Prove all filesystem mutations take place exclusively inside `isolated_worktree`, leaving the main working branch untouched. | `platform/adapters`, `platform/runtime` |
| **GATE C** | **Crash Resilience** | Simulate SIGKILL between outbox staging and physical execution; verify the daemon restarts into state `Uncertain` with automatic reconciliation. | `tests/contract` |
| **GATE D** | **Security Taint Test** | Inject prompt injection payloads into untrusted inputs (PDF, web scraping); verify 100% of untrusted data is quarantined from operational policy. | `platform/core`, `tests/contract`, `tests/testkit` |
| **GATE E** | **SWE-Bench Baseline** | Resolve a sample set of standard SWE-bench Lite issues autonomously without human code intervention. | `evals/` |
| **GATE F** | **Research Reproducibility** | Reproduce a scientific benchmark algorithm from paper citations and generate a complete `ReproducibilityBundle`. | `workbenches/research`, `platform/components` |
| **GATE G** | **LongMemEval Temporal** | Verify long-term memory across sessions; prove the system distinguishes between outdated facts and active facts without hallucination. | `evals/memory` |
| **GATE OPT** | **Economic Non-Inferiority** | Measure actual `CostPerAcceptedTask`; prove System 1 routing reduces costs by $\ge 20\%$ compared to direct frontier LLM execution. | `evals/calibration`, `evals/oi` |

---

## 4. Critical Verification Matrices

### 4.1 Crash & Recovery Matrix (INV-02, INV-13)
Every state transition point is tested against abrupt process termination scenarios:
- **Pre-Commit Crash:** Abrupt SIGKILL immediately prior to committing an SQLite transaction. State must roll back cleanly.
- **Post-Sandbox Crash:** Process killed after tool execution finishes but before persisting the receipt. System must mark attempt as `Uncertain` upon restart and reconcile idempotently.
- **Mid-Stream Disconnect:** Abrupt network drop during token streaming. Partial state is safely parked; no corrupt tokens are committed.
- **Batch Transaction Rollback (INV-13):** If any task in a batch fails to schedule, all staged tasks in that batch are rolled back atomically.

### 4.2 Path Normalization & Symlink Containment (INV-04)
Guarantees absolute immunity against directory traversal and symlink escaping:
- Symlinks pointing outside the repository root are strictly rejected at the domain boundary.
- Traversal sequences (`../`, non-canonical Unicode representations, Windows backslashes on Unix) are trapped by the capability sandbox.
- Race conditions where target files are deleted or replaced during reads are handled deterministically.

### 4.3 Anti-Tampering Gate (INV-07)
If an agent or worker attempts to modify or delete existing test fixtures in an effort to artificially make a test suite pass, the diff analyzer triggers an immediate hard failure: **Task cancelled with status `Failed(TestTamperingDetected)`**.

### 4.4 Process Drop Guard Verification (INV-12)
Spawns child processes in isolated process groups (`setpgid(0)`) and asserts that dropping the guard sends SIGKILL to `-pid`, terminating the entire group and leaving zero zombie processes.

---

## 5. Status Claim Taxonomy

To maintain absolute engineering truthfulness, all repository documentation and pull requests must adhere to the following claim taxonomy:

| Label | Definition | Verification Requirement |
|---|---|---|
| **Designed** | Formal specification, contract, or state machine is documented. | Reviewed specification in `Custos.md` or active documentation. |
| **Implemented** | Source code exists in the workspace. | Compiles cleanly under `cargo check --workspace`. |
| **Wired** | Connected through the daemon composition root or CLI. | Production composition root invokes the component during normal execution. |
| **Verified** | Proven end-to-end under real conditions, including crash and failure paths. | Passing automated integration or end-to-end test in `tests/`. |

> [!IMPORTANT]
> Unverified performance metrics (latency benchmarks, token savings percentages, memory footprints) must remain unstated or explicitly marked as `[Pending Empirical Verification]` until repeatable automated tests prove them.
