# Vinh's Workspace — Agent Systems & Coordination Research Engineer

> **Role:** Agent Systems & Coordination Research Engineer  
> **Core Direction:** Worker lifecycle, structured handoffs, multi-agent scheduling, fault tolerance, protocol integration, observability, and empirical coordination evaluation.  
> **Language & Invariants:** 100% Rust & Formal Protocols. Zero prompt engineering or heuristic tuning. Focus strictly on systems software engineering: message atomicity, actor lifecycle, state synchronization, deadlock prevention, and crash resilience.

---

## 1. Core Focus Areas

1. **Worker Lifecycle Management:**
   - Implement formal state machine transitions for agents:
     `CREATED -> LEASED -> ACTIVE -> WAITING_TOOL -> PAUSED -> COMPLETED / FAILED / CANCELLED`
   - Enforce lifecycle correctness: leases, heartbeats, timeouts, cancellations, retries, orphan recovery, and idempotency.
2. **Multi-Agent Coordination Engine:**
   - Coordinate step assignments across workers and manage task DAG dependencies.
   - Resolve bounded parallel execution, worker availability tracking, and result aggregation.
   - Trigger handoffs and handle worker failures while reporting status back to the Task Kernel.
   - **Invariants:** The Coordinator cannot mutate canonical Task state directly, cannot mint capability execution permits, cannot decide task truth, and cannot invoke tools outside the Capability Gateway.
3. **Structured Handoff Protocols:**
   - Formalize provider-agnostic handoff envelopes (`objective`, `completed_work`, `artifacts`, `evidence`, `decisions`, `assumptions`, `unresolved_questions`, `capabilities_used`, `budget_consumed`, `recommended_next_step`).
   - Eliminate transcript pollution and measure information loss across handoffs.
4. **Scheduling, Concurrency & Deadlock Prevention:**
   - Sequential scheduling, DAG execution, bounded parallelism, and backpressure.
   - Priority queuing, cooperative cancellation propagation, and coordination budgets.
   - Enforce acyclic dependency validation to prevent deadlocks and starvation.
5. **Observability & Telemetry:**
   - Construct causal worker timelines, handoff traces, queue latency, and agent utilization metrics.
   - Build duplicate work detectors, failure/retry dashboards, and per-worker token/cost attribution.

---

## 2. Research & Academic Thesis Alignment

### Proposed Research Thesis
> **Engineering and Empirical Evaluation of Durable Multi-Agent Coordination for Developer Workflows**  
> *(Alternative: Cost-Aware and Fault-Tolerant Multi-Agent Coordination for Local-First Agentic Workflows)*

### Core Research Contributions
1. **Durable Worker Lifecycle:** Formal state machine with crash resilience and atomic recovery.
2. **Structured Handoff Protocol:** Quantitative proof of token reduction and context preservation versus raw transcript exchange.
3. **Budget-Aware Coordination:** Runtime enforcement of coordination, latency, and token budgets.
4. **Topology Benchmarks:** Systematic comparison between single-worker, sequential-role, and parallel-explorer topologies.
5. **Fault-Injection Framework:** Empirical resilience evaluation under crashes, network partitions, and stalled sidecars.
6. **Empirical Metric Suite:** Standardized instrumentation for coordination latency, handoff overhead, and duplicate work.

### Empirical Research Questions
- *Does structured handoff reduce token usage and duplicated work compared to full-context forwarding?*
- *Can a durable coordinator reliably reconstruct execution state after abrupt process termination?*
- *Do parallel multi-agent topologies reduce overall task latency enough to offset coordination overhead?*
- *Does adaptive dynamic worker spawning outperform fixed multi-agent graphs for developer tasks?*
- *Which coordination topology maximizes verified task success for specific software engineering workflows?*

---

## 3. Code Ownership & Repository Layout

```text
crates/
└── workflow-runtime/
    ├── coordination/           # Multi-agent coordination algorithms and DAG resolution
    ├── worker_lifecycle/       # Worker state machine, leases, heartbeats, and timeouts
    ├── scheduler/              # Sequential, DAG, and parallel execution schedulers
    ├── handoff/                # Structured handoff protocols and serialization
    └── recovery/               # Worker crash detection, orphan recovery, and reconciliation

schemas/
├── worker-definition/          # Declarative worker capability and role schemas
├── worker-event/               # Worker lifecycle and telemetry event contracts
├── handoff/                    # Structured handoff envelope schemas
└── coordination/               # Topology, DAG, and coordination plan schemas

adapters/
└── protocols/
    ├── a2a/                    # Agent-to-Agent external protocol integration
    └── agentgateway/           # Agent Gateway boundary integration

evals/
└── multi-agent/
    ├── baselines/              # Single-worker baseline benchmarks
    ├── workloads/              # Standard multi-agent test workloads
    ├── topologies/             # Sequential, parallel, and hierarchical benchmark suites
    ├── fault-injection/        # Crash injection, message loss, and timeout simulations
    └── reports/                # Benchmark output logs and performance matrices

lab/
├── multi-agent/                # Experimental coordination scripts
└── upstreams/                  # Reference investigations (LangGraph, AutoGen, CrewAI, A2A)

tests/
├── coordination/               # Unit and integration tests for multi-agent scheduling
├── concurrency/                # Loom and stress tests for race conditions and deadlocks
├── recovery/                   # Chaos and restart tests for worker durability
└── protocol/                   # Serialization and protocol compatibility suites
```

---

## 4. Workspace Purpose & Directory Structure

This space (`dev_docs/vinh/`) serves two distinct purposes:
1. **Domain Architecture (`notes/`)**: Localized technical specifications, protocol designs, actor schemas, and empirical research notes.
2. **Development Reports (`reports/`)**: Chronological record of daily sprint progress and integration syncs committed to branch `vinh`.

---

## 5. Standard Daily Report Template (`reports/YYYY-MM-DD.md`)

When committing daily progress, write your report to `reports/YYYY-MM-DD.md` in this directory, commit directly to the `vinh` branch alongside your code, and open a Pull Request to `dev`:

```markdown
# Vinh Progress Report — YYYY-MM-DD

## 1. Accomplished Today
- [x] Description (Module: `workflow-runtime/coordination`)
- [x] Description (Schema: `schemas/handoff`)

## 2. Tests & Verification
- Unit / concurrency tests added: `cargo test -p custos_workflow_runtime`
- Benchmark verification: Ran empirical coordination latency harness.

## 3. In-Flight Work & Next Steps
- Currently implementing: Worker heartbeat timeout monitor.
- Next: Wire structured handoff schema into SQLite event log.

## 4. Blockers & Questions for Vi / Truong
- Question for Vi on handoff semantic fields.
- Question for Truong on persistence transaction boundaries.
```
