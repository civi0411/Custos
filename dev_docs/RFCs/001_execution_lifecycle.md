# RFC 001: Execution Lifecycle & Workflow Robustness

**Status:** Draft  
**Target Crates:** `custos-domain`, `custos-core`, `custos-runtime`  

## 1. Problem Statement
The current execution model conflates Tasks, Runs, and Agent Attempts. In addition, the `WorkflowPort` serves too many distinct purposes (planning vs lifecycle command), making it hard to build robust crash recovery and cancel/resume semantics. Without a strict lifecycle distinction, effects cannot be correctly reconciled after a SIGKILL.

## 2. Definitive Terminology (The 5 Layers)

To resolve state confusion, Custos enforces five distinct layers of execution lifecycle:

1. **Task:** The overarching human goal. Associated with a `TaskContractV1` and a `TaskRevision`. Exists across multiple sessions.
2. **Run (Workflow Run):** A single execution instance of a Task/Workflow. A `Run` is scoped to a specific `WorkflowRevision` (the compiled plan).
3. **WorkerRun (Node Run):** An instance of a specific node within the Workflow DAG (e.g., an Engineer agent doing a refactor step). 
4. **NodeAttempt:** A retryable iteration of a `WorkerRun`. If a worker fails due to a flaky test, the scheduler launches a new `NodeAttempt`. 
5. **EffectAttempt:** The finest granularity. Represents a single physical mutation proposed by the worker (e.g., writing a file, sending an email). Bound to a `Permit`. State transitions include `Pending`, `InFlight`, `Succeeded`, `Failed`, and crucially, `Uncertain`.

## 3. Redesigning WorkflowPort

The `WorkflowPort` (in `custos-core::contracts::workflow`) should NOT be a planner. It is strictly a **Lifecycle Command Port**.

```rust
#[async_trait]
pub trait WorkflowPort: Send + Sync {
    /// Starts a run matching a pre-compiled workflow revision.
    async fn start_run(&self, command_id: &str, actor: &str, workflow_revision: &str) -> Result<RunDescriptor, DomainError>;
    
    /// Requests a graceful halt. Does NOT mean external effects are undone.
    async fn request_cancel(&self, run_id: &str, reason: &str) -> Result<(), DomainError>;
    
    /// Resumes a halted or crashed run. Validates source, approval, and effect states first.
    async fn resume_run(&self, run_id: &str) -> Result<EventCursor, DomainError>;
    
    /// Issues a durable checkpoint.
    async fn checkpoint(&self, run_id: &str) -> Result<ContinuationPacket, DomainError>;
}
```

## 4. Crash States & Resiliency
If the system crashes, `NodeAttempt` and `EffectAttempt` states will be left hanging. Upon reboot, the system MUST:
- Identify `InFlight` effects and transition them to `Uncertain`.
- Use an `IdempotencyKey` to trigger a **Reconcile** phase before allowing the `NodeAttempt` to resume.

## 5. Next Steps
1. Refactor `TaskContract` in `custos-domain/src/task.rs` to support `TaskRevision` and `Criterion ID`.
2. Build an E2E fake model (S1/S2 stub) and fake effect generator to validate this lifecycle without real external dependencies.
