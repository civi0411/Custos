# RFC 002: Agent Capability, Native Harnesses & Effect Mediation

**Status:** Draft  
**Target Crates:** `custos-provider`, `custos-adapters`  

## 1. Problem Statement
Treating native coding agents (like Claude Code, Codex, Goose) as simple LLMs (via `ModelPort`) is fundamentally flawed. These agents have their own internal loops, workspace managers, and effect generators. Conversely, treating them strictly as unconstrained systems bypasses Custos's security Kernel (`CapabilityPort`). We need a taxonomy that cleanly separates single-inference, agent loops, and physical effects.

## 2. Decoupling the Ports

Custos defines three distinct interfaces for interacting with external intelligence and the OS:

### A. ModelPort
- **Scope:** Single inference attempt.
- **Responsibilities:** Takes a prompt/context, streams tokens, returns tool-call proposals, handles cancellation at the inference level, and reports usage per attempt.
- **Use Case:** Custom loops written entirely in `custos-runtime` (e.g., S1 Judges, simple Assist streams).

### B. AgentRuntimePort
- **Scope:** A native harness with its own internal ReAct/execution loop (e.g., Claude Code, Goose).
- **Responsibilities:** Start, attach, steer, cancel. It emits events, generates artifacts, requests native approvals, and manages its own workspace ownership.
- **Constraint:** We do NOT force the harness to provide checkpointed hidden states if its native API doesn't support it. We only expose what the harness natively supports.

### C. CapabilityPort (Now SandboxPort / Gateway)
- **Scope:** Physical effects (File I/O, Shell, Network).
- **Responsibilities:** Takes an `ActionIntent` and `ExecutionPermit`. Returns an `ExecutionReceipt`. Includes reconciliation protocols.

## 3. HarnessProfile & Assurance

Every native adapter must publish a `HarnessProfile`. This profile defines the adapter's real capabilities:
- **Tool Mediation:** Does it allow Custos to intercept and block tool calls?
- **Worktree Ownership:** Does it operate on the live tree or an isolated worktree?
- **Event Coverage:** Does it stream thought processes or just final outputs?
- **Cancellation/Resume:** Can it be paused and resumed cleanly?
- **Cost Visibility:** Does it report token usage accurately?

When a native harness bypasses Custos's Capability Gateway (because it runs its own shell commands that we cannot intercept), the resulting effect MUST be labeled with an appropriate **assurance** (e.g., `provider-governed` or `observe-only`), preventing Custos from fraudulently claiming `custos-mediated`.

## 4. Implementation Requirement
We will implement exactly **one** real harness adapter (e.g., Claude Code or Codex API) to prove this design. The Orchestration Intelligence (OI) will use the `HarnessProfile` to decide if the harness meets the Task's required capability.
