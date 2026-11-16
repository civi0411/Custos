# Governed Agent Loop, Budget & Gateway Ownership (F08)

> SADE Master Specification — Phase 7 / F08 Architectural Blueprint  
> Governs: `custos-domain`, `custos-core`, `custos-runtime`, `custos-adapters`, `custos-daemon`, `custos-persistence`, and Golden Acceptance Test `G07`.

---

## 1. Context & Motivation

Unconstrained agent loops in legacy frameworks suffer from severe structural liabilities:
1. **Unchecked Tool Dispatch & Side-Effect Pollution:** Tools are executed directly without kernel policy verification, cryptographic execution permits, or transactional ledgers. Failed, aborted, or retried actions can cause double writes, silent mutations, and data corruption.
2. **Infinite Loops & Runaway Cost:** Loops lacking hard iteration bounds or budget guards consume unbounded API tokens and memory when models fail to converge or get stuck in repetitive tool-calling cycles.
3. **Double Execution Across Layers:** When a governed agent runtime intercepts and executes tools, downstream orchestration layers often receive returned intents and erroneously re-dispatch them, causing duplicate physical writes on disk or duplicate network calls.
4. **Leaked Handlers & Poor Cancellation Responsiveness:** Runs canceled by users often finish in-flight turns, run additional tools, or leave dangling cancellation/steering listeners in memory.
5. **Orphaned Subprocesses & Zombie Leaks:** Terminated CLI agent processes leave orphaned background child processes that continue consuming CPU and holding locks on disk files.

Phase 7 (F08) and the SuperAgent Refactoring implement the governed multi-turn agent loop with two-phase authority checks, transactional outbox admission, duplicate execution prevention, hard budget limits, deterministic cancellation, and the `SupervisedAgentRunner` decorator with process-group kill guards.

---

## 2. Core Architecture & Workflow

```
[ Model API / Provider / CLI Agent ]
                  │ (turn / tool calls)
                  ▼
┌────────────────────────────────────────────────────────┐
│ SupervisedAgentRunner (custos-runtime::agent::runner)  │
│                                                        │
│ 1. Decorates AgentDriver / AgentRuntimePort            │
│ 2. Emits hierarchical StepSpan telemetry               │
│ 3. Records execution to SupervisionTraceSink           │
│ 4. Notifies SupervisedObserver of state anomalies      │
└────────────────────────────────────────────────────────┘
                  │
                  ▼
┌────────────────────────────────────────────────────────┐
│ GovernedAgentRuntime (custos-runtime)                  │
│                                                        │
│ 1. Intercept into ActionIntent (Assurance::Mediated)   │
│ 2. Check cancellation flag before & during turn        │
│ 3. Bound iterations (max_iterations, default 10)       │
│ 4. Bound tool outputs (LimitedWriter, default 64 KiB)  │
└────────────────────────────────────────────────────────┘
                  │ (authorize / execute)
                  ▼
┌────────────────────────────────────────────────────────┐
│ DaemonGovernedToolExecutor (custos-daemon)             │
│                                                        │
│ Phase 1: authorize()                                   │
│  - Parameter safety: reject ".." traversal             │
│  - Gate 2 Policy: reject Critical-risk tools           │
│  - Capability verification: explicit active grant      │
│                                                        │
│ Phase 2: execute()                                     │
│  - request_permit() via TrustedKernel                  │
│  - consume_permit() [INV-04]                           │
│  - record_effect() [InFlight] in OutboxRepository      │
│  - enqueue() [Dispatching] in OutboxPort               │
│  - SovereignDeveloperAdapter / McpFederationHub        │
│  - update_effect_status() & mark_receipted()           │
│  - Atomic fallback: mark Uncertain if settlement fails │
└────────────────────────────────────────────────────────┘
                  │
                  ▼
┌────────────────────────────────────────────────────────┐
│ TaskRuntime Orchestration (custos-runtime)             │
│                                                        │
│ Line 392 Invariant:                                    │
│ if intent.lifecycle_state == ActionLifecycleState::Receipt {
│     continue; // Bypass re-dispatch! Single effect!    │
│ }                                                      │
└────────────────────────────────────────────────────────┘
```

---

## 3. Key Invariants & Guarantees

### 3.1 Two-Phase Tool Mediation & Parameter Safety
- **Path Traversal Protection:** All path-bearing tool parameters (`path`, `file`, `target_file`, `dir`) are validated by `DaemonGovernedToolExecutor::parse_tool_invocation`. Any occurrences of `..` are immediately rejected with `DomainError::Validation`, preventing directory traversal and file escape outside the authorized workspace sandbox.
- **Deterministic Risk Classification:** Tools proposing critical-risk actions (e.g. destructive disk partitioning like `mkfs` or unauthorized commands) are blocked by `authorize()`. Unapproved actions return `DomainError::Unauthorized` or `ToolResult::PendingApproval`, never modifying physical disk state.

### 3.2 Single Physical Effect Invariant (G07 Invariant)
- **Pre-execution Durable Ledger:** Before executing physical side-effects on disk, `DaemonGovernedToolExecutor` atomically records an `InFlight` attempt in `OutboxRepository` and enqueues a `Dispatching` entry in `OutboxPort`.
- **Receipt Marking:** Upon successful execution via `SovereignDeveloperAdapter`, the receipt is settled and the returned `ActionIntent` is assigned `ActionLifecycleState::Receipt`.
- **Downstream Re-dispatch Prevention:** `TaskRuntime::execute_node` explicitly inspects each intent:
  ```rust
  if intent.lifecycle_state == custos_domain::ActionLifecycleState::Receipt {
      continue;
  }
  ```
  This guarantees that already-executed tool calls are never re-evaluated or double-dispatched by downstream orchestration. Exactly one physical effect occurs on disk, and exactly one receipted entry is stored in the outbox.

### 3.3 Loop Convergence & Budget Limits
- **Bounded Iterations:** Governed agent turns are constrained by `max_iterations` (default 10).
- **Status: Limited:** If `max_iterations` is exhausted without the model completing cleanly (i.e. returning empty tool calls), the turn terminates and unconditionally returns:
  ```rust
  DomainError::BudgetExceeded(format!(
      "Agent loop iteration limit of {} exceeded without completion (status: limited)",
      self.max_iterations
  ))
  ```
  Unfinished loops are never reported as successful operations.

### 3.4 Cancellation Responsiveness & Zero Memory Leaks
- When `cancel_run(&run_id)` is invoked, `GovernedAgentRuntime` detects the cancellation token before or during the loop and breaks immediately.
- The turn returns `DomainError::Validation("Run <id> was canceled during agent loop execution")`.
- On exit, cancellation tokens and steering notes for `effective_run_id` are removed from the shared concurrency maps, eliminating memory leaks.

---

## 4. Supervised Agent Runner & Zombie Process Elimination

### 4.1 SupervisedAgentRunner Decorator
Implemented in `platform/runtime/src/agent/runner.rs` (265 lines):
- Wraps any implementation of `AgentDriver` / `AgentRuntimePort`.
- Automatically measures turn duration, token usage, and status transitions.
- Records hierarchical `StepSpan` spans and publishes them to `SupervisionTraceSink` for real-time observability.
- Monitors step duration and failure rates, notifying `SupervisedObserver` when anomalies occur.
- Enforces transparent supervision without modifying the inner agent driver logic.

### 4.2 ProcessDropGuard & Zombie Elimination
Implemented in `platform/adapters/src/harnesses/stream/process.rs`:
- External CLI agents (e.g. Claude Code, Aider, Codex) run as OS subprocesses.
- To prevent orphaned processes when tasks are aborted or run objects are dropped:
  ```rust
  #[cfg(unix)]
  {
      command.process_group(0); // Spawns in isolated process group
  }
  ```
- The inner process handle implements `Drop`:
  ```rust
  #[cfg(unix)]
  if let Some(pid) = self.pid {
      unsafe {
          libc::kill(-(pid as i32), libc::SIGKILL); // Kill whole process group!
      }
  }
  ```
- This guarantees that background compiler runs, dev servers, or agent loops started by child processes are cleanly and immediately terminated.

### 4.3 Canonical Naming & Zero-Mock Policy
- **Typed Interface:** Legacy untyped "harness" terminology is superseded by the `AgentDriver` trait in `platform/core/src/contracts/agent.rs` and `CliAgentAdapter` in `platform/adapters/src/harnesses/adapter.rs`.
- **Zero-Mock Policy:** Production code contains zero mock implementations. All test harnesses, double adapters, and state stubs are strictly isolated behind:
  ```rust
  #[cfg(any(test, feature = "test-support"))]
  ```

---

## 5. Golden Acceptance Test G07

Golden Acceptance Test G07 (`tests/e2e/tests/governed_loop_golden_test.rs`) validates the end-to-end integration:

| Test Case | Invariant Validated | Assertions |
|---|---|---|
| `test_g07_composed_path_admitted_tool_to_real_disk_and_single_receipt` | End-to-end execution path | Real file created in tempdir, 1 receipted outbox entry, 1 succeeded effect attempt. |
| `test_g07_prevention_of_duplicate_execution_in_task_runtime` | Prevention of double-dispatch | `TaskRuntime` skips already receipted intents; exactly 1 write and 1 outbox entry. |
| `test_g07_security_kernel_critical_risk_rejection_no_disk_mutation` | Gate 2 Policy enforcement | Critical tools rejected with `ActionLifecycleState::Failed`; 0 disk mutations, 0 outbox entries. |
| `test_g07_path_traversal_denial_no_disk_mutation` | Path traversal protection | Relative `..` paths rejected with `DomainError::Validation`; zero disk files created outside workspace. |
| `test_g07_bounded_iteration_exhaustion_returns_limited_budget_exceeded` | Budget limits | Iteration limit hit returns `BudgetExceeded` with `"status: limited"`. |
| `test_g07_cancellation_during_governed_loop_cleans_handles_and_halts` | Cancellation & cleanup | Canceled run halts promptly with `Validation` error; steering/cancel maps pruned. |

---

## 6. Verification Commands

```bash
# Run Golden Acceptance Test G07
cargo test -p custos-tests-e2e --test governed_loop_golden_test

# Verify Process Drop Guard & Adapter Tests
cargo test -p custos-adapters --lib harnesses

# Workspace Quality Gates
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask gate
```
