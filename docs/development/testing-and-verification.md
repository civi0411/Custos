# Testing Architecture, Acceptance Gates & Verification Standards

> **Classification:** Normative Quality Assurance Specification  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Developer Hub](README.md).

Custos enforces a rigorous, multi-tiered testing discipline designed to guarantee the correctness, durability, and safety of an autonomous runtime that interacts with and modifies production codebases.

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
2. **Contract & Integration Tests:** Verification of wire schemas (`schemas/`), SQLite database migrations, and adapter traits against deterministic mock servers (`tests/contract`).
3. **Chaos & Crash Matrix:** Injection of simulated SIGKILL power loss, socket abrupt terminations, and full-disk conditions to test the recovery of the SQLite WAL and Git worktrees.
4. **E2E Repo Fixture Tests:** Multi-process daemon and CLI runs on offline canonical repositories (`tests/e2e`), testing end-to-end task decomposition, patch application, and evidence verification.
5. **Live Provider Canaries:** Periodic, budget-capped executions against real upstream vendor APIs (Anthropic, OpenAI) to detect upstream protocol drifts early.

---

## 2. The Eight Product Acceptance Gates

Before any architectural change or feature is merged, it must satisfy the 8 Product Acceptance Gates authoritatively defined in `Custos.md` (Part 18):

| Acceptance Gate | Gate Name | Core Verification Requirement | Target Package / Suite |
|---|---|---|---|
| **GATE A** | **Local Process E2E** | Run from CLI through Daemon, invoking a local model to explain a repo under the `LocalOnly` flag. | `tests/e2e` |
| **GATE B** | **Worktree Isolation** | Prove all filesystem mutations take place exclusively inside `isolated_worktree`, leaving the main working branch untouched. | `custos-adapters` |
| **GATE C** | **Crash Resilience** | Simulate SIGKILL between outbox staging and physical execution; verify the daemon restarts into state `Uncertain` with automatic reconciliation. | `tests/contract` |
| **GATE D** | **Security Taint Test** | Inject prompt injection payloads into untrusted inputs (PDF, web scraping); verify 100% of untrusted data is quarantined from operational policy. | `custos-core/fixtures` |
| **GATE E** | **SWE-Bench Baseline** | Resolve a sample set of standard SWE-bench Lite issues autonomously without human code intervention. | `evals/swe-bench` |
| **GATE F** | **Research Reproducibility** | Reproduce a scientific benchmark algorithm from paper citations and generate a complete `ReproducibilityBundle`. | `custos-packs` |
| **GATE G** | **LongMemEval Temporal** | Verify long-term memory across sessions; prove the system distinguishes between outdated facts and active facts without hallucination. | `evals/memory` |
| **GATE OPT** | **Economic Non-Inferiority** | Measure actual `CostPerAcceptedTask`; prove System 1 routing reduces costs by $\ge 20\%$ compared to direct frontier LLM execution. | `evals/calibration` |

---

## 3. Critical Verification Matrices

### 3.1 Crash Matrix
Every state transition point is tested against abrupt process termination scenarios:
- **Pre-Commit Crash:** Abrupt SIGKILL immediately prior to committing an SQLite transaction. State must roll back cleanly.
- **Post-Sandbox Crash:** Process killed after tool execution finishes but before persisting the receipt. System must mark attempt as `Uncertain` upon restart and reconcile idempotently.
- **Mid-Stream Disconnect:** Abrupt network drop during token streaming. Partial state is safely parked; no corrupt tokens are committed.
- **Out of Disk Storage:** Disk reaches 100% capacity during step write. System aborts with an atomic error without corrupting the WAL journal.

### 3.2 Path Normalization Matrix
Guarantees immunity against path traversal and symlink escaping:
- Symlinks pointing outside the repository root are strictly rejected.
- Traversal sequences (`../`, non-canonical Unicode representations, Windows backslashes on Unix) are trapped by the capability sandbox.
- Race conditions where target files are deleted while worker processes are reading them are handled deterministically.

### 3.3 Anti-Tampering Gate
If an agent or worker attempts to modify or delete existing test fixtures in an effort to artificially make a test suite pass, the diff analyzer triggers an immediate hard failure: **Task cancelled with status `Failed(TestTamperingDetected)`**.

---

## 4. Status Claim Taxonomy

To maintain absolute engineering truthfulness, all repository documentation and pull requests must adhere to the following claim taxonomy:

| Label | Definition | Verification Requirement |
|---|---|---|
| **Designed** | Formal specification, contract, or state machine is documented. | Reviewed specification in `Custos.md` or active documentation. |
| **Implemented** | Source code exists in the workspace. | Compiles cleanly under `cargo check --workspace`. |
| **Wired** | Connected through the daemon composition root or CLI. | Production composition root invokes the component during normal execution. |
| **Verified** | Proven end-to-end under real conditions, including crash and failure paths. | Passing automated integration or end-to-end test in `tests/`. |

> [!IMPORTANT]
> Unverified performance metrics (latency benchmarks, token savings percentages, memory footprints) must remain unstated or explicitly marked as `[Pending Empirical Verification]` until repeatable automated tests prove them.
