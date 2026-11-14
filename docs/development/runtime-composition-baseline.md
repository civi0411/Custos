# Runtime Composition Architecture Baseline (`apps/daemon/src/runtime.rs`)

> **Classification:** Normative Runtime Composition Specification  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Developer Hub](README.md).

This document specifies the exact dependency injection graph, lifecycle phases, and service composition of the Custos Daemon runtime (`apps/daemon/src/runtime.rs` and `apps/daemon/src/bootstrap/`). 

Custos strictly enforces the **Single Composition Root** architectural pattern: all concrete storage repositories, model adapters, sandbox drivers, and coordinators are instantiated exclusively at the application boundary during startup, injected into pure abstract contracts (`platform/core/src/contracts/`), and passed down to the operational pipeline.

---

## 1. The Five-Phase Bootstrap Pipeline

The daemon startup decomposes into five strictly ordered, verifiable bootstrap phases:

```text
┌────────────────────────────────────────────────────────────────────────┐
│                      DAEMON BOOTSTRAP LIFECYCLE                        │
├────────────────────────────────────────────────────────────────────────┤
│ Phase 1: Storage Crash Recovery & Reconciliation                       │
│          reconcile_storage_on_startup() (Gate 3, INV-02)               │
├────────────────────────────────────────────────────────────────────────┤
│ Phase 2: Background Sweepers & Reconcilers                             │
│          spawn_background_sweepers() (OAuth & Inflight Reconcilers)    │
├────────────────────────────────────────────────────────────────────────┤
│ Phase 3: Kernel & Isolation Services                                   │
│          init_kernel_services() (TrustedKernel, Sandbox, Supervision)  │
├────────────────────────────────────────────────────────────────────────┤
│ Phase 4: Workbenches, Packs & Governed Harness Setup                   │
│          init_workbenches_and_harness() (Inventory, MCP Hub, Runner)   │
├────────────────────────────────────────────────────────────────────────┤
│ Phase 5: Coordinators, TaskRuntime & LocalApiDispatcher                │
│          init_coordinators_and_api() (T5LeadWorker, 197 RPC Routes)    │
└────────────────────────────────────────────────────────────────────────┘
```

### Phase 1: Storage Crash Recovery & Reconciliation (`bootstrap/storage.rs`)
- **Outbox Reconcile:** Scans the transactional outbox (`store.outbox().reconcile_on_startup()`) and transitions any abandoned `InFlight` side effects into `Uncertain`.
- **Run Reconcile:** Scans active runs (`store.reconcile_runs_on_startup()`) and marks orphaned running tasks as `Failed(InterruptedByRestart)`.
- **Command Ledger Reconcile:** Identifies orphaned command attempts exceeding 24 hours and marks them failed.

### Phase 2: Background Sweepers (`bootstrap/background.rs`)
- **OAuth Token Sweeper:** Spawns a background task running every 5 minutes to refresh expiring OAuth 2.1 tokens across external providers (GitHub, Google, Anthropic).
- **Uncertain Runs Sweeper:** Periodically interrogates physical adapters to reconcile `Uncertain` effect attempts.

### Phase 3: Kernel & Isolation Services (`bootstrap/kernel.rs`)
- **`TrustedKernel`:** Core state machine validator enforcing two-phase mediation and terminal state invariants.
- **`SovereignDeveloperAdapter`:** Sandbox capability boundary enforcing worktree path isolation and permit verification.
- **`SupervisionCoordinator`:** Real-time event sink routing step telemetry to registered observers.
- **`LiveRuns`:** In-memory registry tracking concurrent running tasks across the daemon.

### Phase 4: Workbenches, Packs & Governed Harness Setup (`bootstrap/workbenches.rs`)
- **Package Inventory Discovery:** Probes `~/.custos/packages` via `scan_local_packages()` and populates dynamic component inventory.
- **Workbench Registration:** Checks SQLite package inventory and registers installed workbenches (`Assistant`, `Coding`, `Research`, `Quant`) into `ModuleRegistry`.
- **`McpFederationHub`:** Initializes the Model Context Protocol aggregator for streaming external tool calls.
- **`SupervisedAgentRunner`:** Wraps the `GovernedAgentRuntime` with OpenTelemetry step spans, anomaly detection, and `ProcessDropGuard` zombie elimination.

### Phase 5: Coordinators, TaskRuntime & LocalApiDispatcher (`bootstrap/coordinators.rs`)
- **`WorkspaceCoordinator` & `WorkspaceLeaseManager`:** Manages isolated Git worktree checkouts and branch sandboxes.
- **`WorkloadAdmissionCoordinator`:** Dynamically allocates GPU memory and compute queues across training and inference.
- **`TaskRuntime`:** Composes the workflow dispatcher, active claims restoration, `T5LeadWorker` DAG orchestrator, and `FastPathFilter`.
- **`TerminalCoordinator` & `PythonKernelCoordinator`:** PTY session driver and Jupyter/IPython kernel manager.
- **`LocalApiDispatcher`:** Exposes the 197+ JSON-RPC methods across 4 cohesive contexts.

---

## 2. Dependency Injection Matrix

| Component / Port | Concrete Implementation | Injection Target | Architectural Contract |
|---|---|---|---|
| `store` | `SqliteTaskStore` | Kernel, Workflow, LocalApi, SessionManager | `StoragePort`, `RunPort`, `OutboxPort` |
| `credentials` | `ProviderCredentials` | LocalApi, Sweeper, Resolver, OAuthServer | `CredentialVault` |
| `availability` | `ModelAvailability` | Resolver, LocalApi | Dynamic model catalog cache |
| `resolver` | `DaemonCredentialResolver` | `RouterModelProvider` | Model credential resolution |
| `model` | `RouterModelProvider` | TaskRuntime, GovernedRuntime | `ModelPort` (OpenAI, Anthropic, Gemini, Local) |
| `kernel` | `TrustedKernel` | TaskRuntime, LocalApi | `KernelPort` (Task state transitions) |
| `sandbox` | `SovereignDeveloperAdapter` | TaskRuntime, GovernedRuntime | `SandboxPort` (Filesystem containment) |
| `supervision` | `SupervisionCoordinator` | TaskRuntime, GovernedRuntime | `SupervisionTraceSink` |
| `mcp_hub` | `McpFederationHub` | Workbenches, GovernedRuntime | External MCP server federation |
| `harnesses` | `HarnessRegistry` | LocalApi, TaskRuntime | `AgentRuntimePort` (`governed`, `claude-code`) |
| `workflow` | `TaskRuntime` | LocalApi | `WorkflowPort` (DAG execution engine) |
| `lease_manager` | `WorkspaceLeaseManager` | CustosRuntime, LocalApi | Worktree lease locking |
| `workspace_coordinator` | `WorkspaceCoordinator` | LocalApi, TaskRuntime | Workspace provider & Git operations |
| `terminal_coordinator` | `TerminalCoordinator` | LocalApi | Native PTY driver |
| `python_coordinator` | `PythonKernelCoordinator` (opt) | LocalApi | Jupyter kernel execution |
| `pack_registry` | `DomainPackRegistry` | LocalApi, TaskRuntime | Domain pack workflows & verifiers |

---

## 3. Runtime Architecture Diagram

```mermaid
graph TD
    Store[SqliteTaskStore (WAL)] --> Kernel[TrustedKernel]
    Store --> TaskSvc[TaskService]
    Store --> SessMgr[SessionManager]
    Store --> Resolver[DaemonCredentialResolver]
    Credentials[ProviderCredentials] --> Resolver
    Resolver --> Router[RouterModelProvider]
    Router --> Model[Arc<dyn ModelPort>]

    DevAdapter[SovereignDeveloperAdapter] --> Sandbox[Arc<dyn SandboxPort>]
    Supervision[SupervisionCoordinator] --> Runner[SupervisedAgentRunner]
    Model --> Governed[GovernedAgentRuntime]
    Sandbox --> Governed
    Runner --> Governed
    Governed --> HarnessReg[HarnessRegistry]

    Kernel --> TaskRuntime[TaskRuntime (T5LeadWorker + FastPath)]
    Model --> TaskRuntime
    Sandbox --> TaskRuntime
    Store --> TaskRuntime
    SessMgr --> TaskRuntime
    Supervision --> TaskRuntime
    WsCoord[WorkspaceCoordinator] --> TaskRuntime
    Admission[WorkloadAdmissionCoordinator] --> TaskRuntime

    TaskSvc --> Bridge[BridgeService]
    SessMgr --> Bridge

    TaskSvc --> Dispatcher[LocalApiDispatcher (197 RPC Routes)]
    SessMgr --> Dispatcher
    Bridge --> Dispatcher
    TaskRuntime --> Dispatcher
    WsCoord --> Dispatcher
    HarnessReg --> Dispatcher
    Admission --> Dispatcher
```

---

## 4. Verification & Testing

The runtime composition graph is verified by:
- **Unit & Property Tests:** `apps/daemon/src/api/tests/` (verifying isolation across assistant, coding, research, quant).
- **Recovery Tests:** `tests/contract/` (verifying crash recovery and outbox reconciliation).
- **E2E Canary Slices:** `tests/e2e/tests/oi_topologies_slice.rs` (verifying full runtime execution across T1–T5 topologies).
