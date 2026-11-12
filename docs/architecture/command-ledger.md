# Command Ledger & Idempotency Fence Specification

> **Scope**: `apps/daemon/src/api/` & `platform/storage/src/repositories/command_ledger.rs`  
> **Classification**: Architecture Reference  
> **Status**: Canonical Specification (Phase 1 / F03)

---

## 1. Idempotency Fence Model

Custos exposes a Local JSON-RPC API across desktop, CLI, and extensions. Mutation requests require a unique `command_id` to guarantee exactly-once execution semantics across process restarts and network retries.

The execution lifecycle consists of three distinct phases:
1. **Admission (`admit`)**: Records the command as `in_flight`. Replays prior results if already completed; rejects concurrent duplicates if currently in flight.
2. **Application Dispatch**: Executes the domain mutation against core services and SQLite stores.
3. **Settlement (`complete`)**: Records the mutation output and marks the command as completed.

---

## 2. Settlement Retry & Recovery

### 2.1 Exponential Backoff Retry Loop
Between domain mutation execution and settlement persistence, transient database lock contention (e.g. SQLite `busy`) could orphan commands in `in_flight` state.
The daemon implements an exponential retry loop:
- Retries: 3 attempts.
- Delays: `5ms`, `25ms`, `125ms`.
- If settlement fails after 3 retries, the daemon fences the command and returns an honest error prompting state reconciliation rather than blind client retry.

### 2.2 Daemon Startup In-Flight Reconciliation
Upon startup, `runtime.rs` invokes:
```rust
store.command_ledger().reconcile_orphaned_inflight_commands(24);
```
Any command left `in_flight` exceeding the threshold (24 hours) is transitioned to failed, clearing the admission fence for future attempts.
