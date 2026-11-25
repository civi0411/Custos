# Sovereign Agent Execution Environment (AEE) Architecture

> Single Source of Truth (SSOT) defining the unified Agent Execution Environment lifecycle, honest assurance model, and invariant evaluation gates for Custos SADE.

---

## 1. Principles & Design Philosophy

The Agent Execution Environment (AEE) adheres to the **Anthropic Agent Philosophy** and **SADE Sovereign Boundary Protocols**:

1. **Anti-Null Environment Invariant**: An autonomous agent must never execute in a null, unanchored, or speculative environment (`sade-foundations.md`). Every mutation must be rooted in an explicit, verified environment handle.
2. **Honest Assurance**: The system must never inflate or hallucinate isolation guarantees. The reported assurance level (`measured_assurance`) must strictly reflect host-verified capabilities (`observe_only`, `provider_governed`, `custos_mediated`), never inferred merely from naming or configuration labels (Gate 4 / ENV-03).
3. **Fail-Closed Gate Evaluation**: If any required environment binding (e.g. Git worktree for Coding, corpus directory for Research, air-gapped network for Quant backtests) is missing or violated, the environment transitions to `Degraded` or `SetupFailed`, refusing execution.
4. **Hexagonal Boundary Cleanliness**: Zero I/O in the domain layer. All state transitions, specs, and snapshot pins live in pure domain aggregate structures. Orchestration is mediated via core ports and runtime coordinators.

---

## 2. Hexagonal Architectural Topology

```
+-----------------------------------------------------------------------------------+
|                                 Desktop UI / CLI                                  |
|   (Wire Models: ui/desktop/src/types/domain/environment.ts, RPC: v1.environment.*) |
+------------------------------------------+----------------------------------------+
                                           |
                                           v
+-----------------------------------------------------------------------------------+
|                        Composition Root: apps/daemon                              |
|   (Dispatcher: application/environment.rs -> EnvironmentCoordinator)              |
+------------------------------------------+----------------------------------------+
                                           |
                                           v
+-----------------------------------------------------------------------------------+
|                      Orchestration: platform/runtime                              |
|   EnvironmentCoordinator (platform/runtime/src/environment/coordinator.rs)        |
|     * Reuses WorkspaceCoordinator, TerminalPort, Sandbox drivers                  |
|     * Enforces fail-closed lifecycle: Initializing -> Ready / Degraded            |
+--------------------+-------------------------------------+------------------------+
                     |                                     |
                     v                                     v
+-------------------------------------+   +-----------------------------------------+
|     Contracts: platform/core        |   |         Domain: platform/domain         |
| EnvironmentPort (contracts/env.rs)  |   | EnvironmentSpec, Profile, State,       |
| EnvironmentGate (ENV-01 .. ENV-07)  |   | EnvSnapshot, EnvironmentAssurance       |
+-------------------------------------+   +-----------------------------------------+
```

---

## 3. Aggregate Domain Vocabulary (`platform/domain/src/environment/`)

| Type | Location | Architectural Responsibility |
|---|---|---|
| `EnvironmentProfile` | `profile.rs` | Lens-specific binding declaration: `Coding{worktree}`, `Assistant{logical}`, `Research{corpus,kernel}`, `Quant{partition,live}`. |
| `EnvironmentSpec` | `spec.rs` | Immutable specification: session binding, target assurance, policies, compute lane. |
| `EnvironmentState` | `state.rs` | Explicit lifecycle state: `Initializing`, `Ready`, `Degraded(reason)`, `SetupFailed(err)`, `TornDown`. |
| `EnvSnapshot` | `snapshot.rs` | Universal reproducibility pin: lockfile hash, random seed, dataset hash, tool versions. |
| `EnvironmentAssurance` | `assurance.rs` | Host-observed isolation metrics: filesystem confinement, network restriction, process isolation. |

---

## 4. Invariant Gates (`platform/core/src/environment/gate.rs`)

The environment subsystem evaluates 7 canonical, deterministic invariant gates prior to task execution:

- **ENV-01 (Ready State)**: No task may execute when `EnvironmentState != Ready` (or verified degraded fallback).
- **ENV-02 (Coding Worktree Isolation)**: The Coding lens strictly requires an existing physical Git worktree, a declared non-empty branch, and filesystem confinement.
- **ENV-03 (Honest Assurance)**: Target assurance declared in `EnvironmentSpec` must not exceed observed host capabilities. Promising `CustosMediated` under unconfined or observe-only hosts is rejected fail-closed.
- **ENV-04 (Research Corpus Binding)**: Research environments require a validated local document corpus path and reproducible snapshot binding.
- **ENV-05 (Quant Air-Gap)**: Backtest execution requires `network_restricted = true` (zero live sockets permitted).
- **ENV-06 (Idempotent Teardown & Orphan Reconcile)**: Teardown operations are safe to repeat; orphaned worktrees, PTYs, and kernel sessions are reconciled at daemon startup.
- **ENV-07 (Explicit Degradation Reason)**: Any degraded state transition must declare an explicit, human-auditable reason string.

---

## 5. Local API Wire Protocol (`v1.environment.*`)

The daemon exposes five sovereign RPC endpoints conforming to the JSON-RPC local protocol:

1. `v1.environment.provision`: Validates spec, allocates physical/logical workspace, probes host assurance, evaluates invariant gates, and registers the environment handle.
2. `v1.environment.get`: Retrieves handle details, current lifecycle state, and measured assurance.
3. `v1.environment.list`: Lists all active environment handles.
4. `v1.environment.verify`: Re-evaluates invariant gates against live observations and updates state to `Ready` or `Degraded`.
5. `v1.environment.teardown`: Releases bound terminal PTYs, tears down physical worktrees (if ephemeral), and transitions state to `TornDown`.

