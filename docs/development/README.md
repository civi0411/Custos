# Custos Developer Hub & Engineering Manual

> **Classification:** Normative Developer Hub & Engineering Manual  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Repository Documentation Hub:** See [Custos Documentation Overview](../README.md).

Welcome to the Custos developer engineering portal. Custos is a local-first, human-governed runtime for proof-carrying agentic AI work. This directory serves as the technical manual for systems engineers, AI cognitive architects, runtime developers, and UI contributors operating within the Custos monorepo.

---

## 1. Architectural Philosophy: The SADE Imperative

Mainstream multi-agent systems suffer from systemic fragility: ephemeral conversational state, hallucinated completion, unchecked file mutations, orphaned background processes, and unconstrained token expenditures. Custos fundamentally resolves these failure modes through **SADE (Sovereign Agent Development Environment)**:

1. **Deterministic State Continuity:** All tasks, steps, tool permits, and runs are persistently journaled into an ACID SQLite Write-Ahead Log (WAL). A sudden power loss or process kill returns the system cleanly to `Uncertain` with automatic idempotent reconciliation.
2. **Two-Phase Kernel Mediation:** LLMs and agent workers propose `ActionIntent`s; only the cryptographically signed, capability-governed `AuthorityEngine` can issue single-use `ExecutionPermit`s.
3. **Independent Evidence Verification:** Agents can never mark their own work completed. The `CompletionGate` requires empirical receipts (compiler digests, test exit codes, AST proofs) validated by isolated verifiers.
4. **Economic & Latency Optimization (OI):** The Measured Orchestration Intelligence engine dynamically selects execution topologies (from pinned local heuristics to multi-agent DAGs) balancing quality, cost, and latency using Pareto multi-objective optimization.
5. **Zero Zombie Leaks:** Every subprocess is spawned in an isolated process group (`setpgid(0)`) governed by `ProcessDropGuard` sending SIGKILL to `-pid` on termination.

---

## 2. Monorepo Topology & Crate Layout

The Custos repository is organized as a unified Cargo workspace containing **20 Rust packages** structured into three concentric rings, complemented by a modern Tauri/Vite desktop UI:

```text
Custos/
├── platform/                 # Ring 0: Foundational Platform Subsystems
│   ├── domain/               # Pure mathematical types, state machines, zero I/O (INV-01)
│   ├── core/                 # Abstract contracts, ports, TaskService, supervisor traits
│   ├── storage/              # SQLite WAL persistence, partition engine, migrations
│   ├── ai/                   # ModelPort router, provider adapters, local inference engine
│   ├── runtime/              # TaskRuntime, DAG coordinators, OI engine, FastPathFilter
│   ├── adapters/             # Sandbox, PTY terminal, MCP federation hub, Git worktrees
│   └── components/           # Reusable capability components (reviews, diffs, canvas)
├── workbenches/              # Ring 1: Specialized Vertical Workbenches
│   ├── assistant/            # Personal copilot, drafts, outbox pattern approvals
│   ├── coding/               # Software engineering, worktree branching, patch verification
│   ├── research/             # Scientific research, paper citations, literature graph
│   └── quant/                # Quantitative trading, backtesting, portfolio risk analysis
├── sdk/                      # Developer SDK & IPC Wire Protocols
│   ├── protocol/             # JSON-RPC wire envelopes, event streams, schema types
│   └── client-rust/          # Asynchronous Rust client library for Custos daemon
├── apps/                     # Ring 2: Application Entrypoints
│   ├── daemon/               # Headless custosd background daemon, LocalApiDispatcher
│   ├── cli/                  # Command-line interface utility (custos)
│   └── desktop/native/       # Native desktop application host
├── ui/desktop/               # TypeScript, React 19, Tailwind CSS, Vite frontend
├── tests/                    # Comprehensive Integration, Contract & E2E Suites
│   ├── testkit/              # Deterministic test fixtures, mock servers, assertions
│   ├── contract/             # Wire contract schemas & migration resilience tests
│   └── e2e/                  # End-to-end task decomposition & canary topologies
└── xtask/                    # Engineering workflow automation & automated quality gates
```

---

## 3. Thematic Engineering Guide Index

All developer documentation, baselines, and implementation specifications map directly to the **10 Master Thematic Pillars**:

### Theme 1: SADE Product Vision & Agent Lake Ecosystem
- **[Codebase Physical Architecture Catalog](codebase-architecture.md):** Complete catalog of all 1,085 source files, exported traits, and line counts across 20 packages.
- **[Upstream Source Map](upstream-source-map.md):** Audit of adopted architectural patterns and upstream provenance.

### Theme 2: Task Kernel & State Machine Engineering
- **[Runtime Composition Architecture](runtime-composition-baseline.md):** Complete 5-phase daemon bootstrap dependency injection graph.
- **[Task State Machine Reference](../reference/system-invariants.md):** Formal invariants governing task lifecycle transitions (`Created` $\rightarrow$ `Running` $\rightarrow$ `Evaluating` $\rightarrow$ `Succeeded` / `Failed`).

### Theme 3: Authority, Security & Sandboxing Gateway
- **[Engineering Standards & Observability](engineering-standards.md):** The Five Golden Rules, supply chain security (`cargo-deny`), and pre-log secret redaction.
- **[System Invariants Matrix](../reference/system-invariants.md):** Complete specifications for INV-01 through INV-14.

### Theme 4: Governed Agent Loop, Harnesses & Zombie Process Isolation
- **[Testing & Verification Standards](testing-and-verification.md):** 5-tier testing pyramid, chaos recovery matrix, and process drop guard verification.
- **[Supervision Architecture](engineering-standards.md#4-observability--supervision-architecture):** `SupervisedAgentRunner`, `SupervisionTraceSink`, and real-time step spans.

### Theme 5: Evidence Engine, Completion Gates & Verifier Contracts
- **[Testing Standards & Claim Taxonomy](testing-and-verification.md#5-status-claim-taxonomy):** Verification requirements for Designed vs Implemented vs Wired vs Verified claims.
- **[Anti-Tampering Enforcement](testing-and-verification.md#43-anti-tampering-gate-inv-07):** Protection against synthetic test tampering.

### Theme 6: Measured Orchestration Intelligence (OI) & Economic Topologies
- **[Runtime Composition Baseline](runtime-composition-baseline.md):** Integration of `T5LeadWorker` DAG coordinator and `FastPathFilter` zero-token routing.
- **[OI Economic Benchmark Contract](../../evals/oi/README.md):** Evaluation suite verifying Pareto utility optimization across models.

### Theme 7: Storage Architecture, Relational SQLite WAL & Partition Engine
- **[API Surface Baseline](api-surface-baseline.md):** Complete inventory of 197+ RPC endpoints and their underlying storage repositories.
- **[Storage Crash Recovery Matrix](testing-and-verification.md#41-crash--recovery-matrix-inv-02-inv-13):** Pre-commit, post-sandbox, and mid-stream crash resilience.

### Theme 8: Heterogeneous Compute, GPU Admission & Accelerator Relay
- **[Runtime Composition (Admission Coordinator)](runtime-composition-baseline.md):** Integration of `WorkloadAdmissionCoordinator` governing GPU memory.
- **[GPU Workload Components Plan](gpu-workload-components-plan.md):** Remote compute leasing, external accelerator relay, and checkpoint continuity.

### Theme 9: Vertical Specialist Workbenches (Assistant, Coding, Research, Quant)
- **[Modular Workbench Architecture](coding-conversation-and-research-workbench-model.md):** Multi-workbench domain models, state separation, and UI lenses.
- **[Workbench Controls & Panels](workbench-control-inventory-panels.md):** UI component layout and inspector surfaces across all 4 workbenches.

### Theme 10: Connectivity Hubs, Desktop UI & Operational Parity
- **[API Surface Baseline](api-surface-baseline.md):** Comprehensive JSON-RPC protocol specification for desktop UI and CLI.
- **[Delivery Blueprint & Release Gates](delivery-blueprint.md):** Vertical slice engineering phases and product acceptance gates.

---

## 4. Developer Quick-Start & Verification Commands

All developers must verify changes through automated workspace quality gates before submitting pull requests:

```bash
# Set path to include Cargo and Node.js
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"

# 1. Fast boundary and file line ceiling check (< 0.5s)
cargo xtask check-boundaries

# 2. Run the 5-Gate Quality Pipeline (manifests, UI build, canvas smoke, contracts, check)
cargo xtask gate

# 3. Fast incremental compilation check
cargo check --workspace --all-targets --offline

# 4. Pure unit and contract tests (using --lib --tests to avoid doc-test hangs on Darwin)
cargo test --workspace --lib --tests --offline

# 5. Strict linter check (zero warnings policy)
cargo clippy --workspace --all-targets --offline -- -D warnings

# 6. Check formatting compliance
cargo fmt --all -- --check
```

---

## 5. Architectural Invariants for Developers

When implementing new features or refactoring existing modules, adhere strictly to these principles:

1. **Never introduce I/O into `platform/domain`:** Any import of `tokio::fs`, `std::fs`, or `reqwest` in `platform/domain` will fail `cargo xtask check-boundaries`.
2. **Never exceed 600 lines per file:** Gate 4 automatically checks line counts. If your file approaches 600 lines, decompose it into submodules.
3. **Wrap external processes with `ProcessDropGuard`:** Never call `std::process::Command::spawn()` bare. Use `custos_adapters::harnesses::stream::process::spawn_guarded()` with `setpgid(0)`.
4. **Zero-Mock Policy in Release Binaries:** Never add mock structures to non-test code. Keep mocks strictly behind `#[cfg(any(test, feature = "test-support"))]`.
5. **Always update documentation:** Follow the Documentation Synchronization Triad (`Custos.md`, `docs/`, `codebase-architecture.md`) whenever adding or modifying public contracts.
