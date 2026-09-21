# Evidence & Verification Architecture

> **Status:** Canonical Baseline v4.0  
> **Source:** Part IV (§13-14) & Part V (§24) Canonical Specification

Custos operates under the core invariant: **Reject ungrounded AI assertions; all accepted outcomes must carry objective, verifiable evidence.**

---

## 1. Formal Definition: Evidence-Carrying Action (ECA)

An Evidence-Carrying Action $ECA$ is defined as a formal 6-tuple:

$$ECA = \langle Intent, Proposal, Authority, Execution, Evidence, Verification \rangle$$

Where:
- $Intent$: Human principal goal or subtask objective.
- $Proposal$: Execution plan and projected diff proposed by the worker.
- $Authority$: Valid authorization granted via `ExecutionPermit`.
- $Execution$: Sandboxed execution producing artifacts and execution traces.
- $Evidence$: Objective receipts and observations collected from the environment.
- $Verification$: Independent evaluation by the Verifier confirming $Evidence \models Intent$.

---

## 2. Six Evidence Classes

Custos categorizes verification evidence into 6 hierarchical classes:

| Class | Name | Evidence Category | Concrete Examples |
|---|---|---|---|
| **L0** | **Static Evidence** | Syntax & static structure validation | AST parse succeeds, Linter emits 0 warnings, JSON schema valid. |
| **L1** | **Execution Evidence** | Process execution receipt | Exit code $= 0$, clean stdout/stderr logs, execution completed within timeout. |
| **L2** | **Deterministic Test** | Independent automated test suites | `cargo test` passes 100%, code coverage meets required threshold. |
| **L3** | **Environmental Evidence** | Environmental state verification | File hashes match expectation, Git worktree clean, service returns HTTP 200. |
| **L4** | **Human Attestation** | Explicit human sign-off | Human inspects preview diff and signs approval for handover. |
| **L5** | **Cryptographic Proof** | Digital signatures & content addressing | Ed25519 signature from Kernel, CAS content hashes verified. |

---

## 3. Verifiable Outcome Bundle

Upon Task completion, the runtime outputs a structured **Verifiable Outcome Bundle** formatted in canonical YAML/JSON:

```yaml
bundle_version: "custos.outcome.v1"
task_id: "tsk_01J8N6Z8K9M0P1Q2R3S4T5U6V7"
completed_at: "2026-09-21T21:30:00Z"

contract_fulfillment:
  intent: "Fix issue #102: JWT token expiration handling"
  status: "FULLY_SATISFIED"
  completion_gate_passed: true

artifacts:
  - path: "crates/auth/src/token.rs"
    content_hash: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    diff_stat: "+24 -6 lines"

evidence_ledger:
  - class: "L0_STATIC"
    tool: "cargo-clippy"
    exit_code: 0
    receipt_id: "rcpt_clippy_01"
  - class: "L2_DETERMINISTIC_TEST"
    tool: "cargo-test"
    tests_run: 32
    tests_passed: 32
    receipt_id: "rcpt_test_02"
    log_artifact_hash: "sha256:8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4"

cost_accounting:
  total_tokens: 14250
  total_cost_usd: 0.0428
  wall_clock_seconds: 18.4

signatures:
  kernel_signature: "ed25519:3b9ac97d519b93821a..."
```
