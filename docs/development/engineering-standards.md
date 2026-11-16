# Engineering Standards, Observability & Supply Chain

> **Classification:** Normative Engineering Standards Specification  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Developer Hub](README.md).

Custos maintains strict engineering standards regarding external dependency adoption, license compliance, operational telemetry, privacy protection, and architectural integrity. Every component merged into the monorepo must withstand rigorous automated gates and adhere to the core principles of proof-carrying agentic computing.

---

## 1. The Five Golden Architectural Rules

To prevent code bloat, architectural decay, and runtime instability, all software written for Custos must comply with the 5 Golden Rules:

1. **Zero-I/O Domain Purity (INV-01):** The `platform/domain` crate contains only pure domain data structures, mathematical models, state machines, and error types. It is strictly forbidden to import `tokio`, perform filesystem I/O, make network calls, or generate random numbers inside domain types.
2. **Single-Writer SQLite WAL (INV-02):** Persistence is mediated through a single write connection with Write-Ahead Logging (WAL) enabled, strict `busy_timeout(5000)`, and immediate transactions. All state mutations must be serializable and atomic.
3. **Thin Daemon Transport Adapters (INV-03):** Transport layers (IPC sockets, HTTP servers, CLI parsers) must remain thin delegators. Business logic, state transitions, and authorization checks belong exclusively in `platform/core` and `platform/runtime`.
4. **Zero-Mock Policy in Production Releases (INV-11):** Production release binaries must remain 100% free of mocks, stubs, or synthetic fallbacks. All mock structures and test harnesses must reside behind `#[cfg(any(test, feature = "test-support"))]`.
5. **Strict File Line Ceiling ($\le 600$ Lines):** Every source file (`.rs`, `.ts`, `.tsx`, `.css`) across the entire repository must strictly not exceed 600 physical lines. Monolithic files must be decomposed into cohesive submodules grouped by bounded context.

---

## 2. Open Source Adoption Philosophy

Custos follows a disciplined open source strategy: **Own the semantic core, avoid indiscriminate copy-pasting, and integrate high-quality libraries exclusively across strict architectural boundaries.**

### The Five Adoption Modes

| Mode | Definition | Architectural Policy | Examples |
|---|---|---|---|
| **Direct Dependency** | Added directly into `Cargo.toml`. | Standardized, foundational libraries with permissive licenses. | `tokio`, `serde`, `rusqlite`, `tree-sitter` |
| **Clean Integration** | Integrated via a dedicated adapter. | Interfaced strictly through abstract traits; swappable without modifying core logic. | `modelcontextprotocol/rust-sdk`, `opentelemetry` |
| **Reference / Borrow**| Architectural study re-implemented to Custos specs. | Learn structural patterns; reproduce them with Custos-native unit tests. | FSM persistence patterns from Temporal; Outbox from Restate |
| **Shadow / Evaluation**| Run in parallel evaluation mode. | Zero impact on main execution paths; used purely for metric comparisons. | Local SLM inference evaluation backends |
| **Reject / No-Adopt** | Explicitly forbidden. | Bloated frameworks, monolithic chat agent wrappers, or copyleft licenses. | Monolithic AI wrappers (LangChain, CrewAI), GPL/AGPL code |

### Curated Upstream Library Map

```text
┌─────────────────────────────────────────────────────────────┐
│                    CURATED REPOSITORY MAP                   │
├─────────────────────────┬───────────────────────────────────┤
│ Foundation & AST Parsing│ tree-sitter/tree-sitter           │
│                         │ ast-grep/ast-grep                 │
│                         │ BurntSushi/ripgrep                │
├─────────────────────────┼───────────────────────────────────┤
│ Security & Policy       │ cedar-policy/cedar                │
│                         │ containers/bubblewrap             │
├─────────────────────────┼───────────────────────────────────┤
│ Protocols & Standards   │ modelcontextprotocol/rust-sdk     │
│                         │ open-telemetry/opentelemetry-rust │
├─────────────────────────┼───────────────────────────────────┤
│ Provider SDKs & Adapters│ openai/codex                      │
│                         │ anthropics/claude-agent-sdk-*     │
└─────────────────────────┴───────────────────────────────────┘
```

---

## 3. Supply Chain & License Compliance

- **Automated License Auditing:** Automated verification via `cargo-deny` in continuous integration. Only **MIT, Apache-2.0, BSD-2-Clause, and BSD-3-Clause** licenses are permitted.
- **Copyleft Prohibition:** GPL, AGPL, and SSPL dependencies are strictly prohibited in all binary releases.
- **Pinned Dependencies:** The root `Cargo.lock` is committed to version control; dependency upgrades must be isolated in dedicated pull requests accompanied by regression test verification.

---

## 4. Observability & Supervision Architecture

Custos implements the **OpenTelemetry** standard across all crates, providing comprehensive visibility into local operational internals while preserving user privacy and providing real-time supervision.

### 4.1 Trace Span Hierarchy

All operations within a Task are structured as an explicit hierarchical span tree:

```text
Workspace
└── Task Trace (task_id: "tsk_01J8N6...")
    ├── Step Spans (step: 1, role: "engineering.explorer")
    │   ├── Context Compilation Span (structural scoring & token budget)
    │   ├── Provider Call Span (model: "codex", streaming)
    │   └── Decision Case Span (action proposal evaluation)
    ├── Step Spans (step: 2, role: "engineering.patcher")
    │   ├── Tool Execution Span (tool: "apply_patch", worktree sandbox)
    │   └── Verifier Span (tool: "cargo_test", receipt_id)
    └── Human Approval Span (waiting for user exact-payload authorization)
```

### 4.2 Supervised Agent Runner & Telemetry Sink

The `SupervisedAgentRunner` decorator (`platform/runtime/src/agent/runner.rs`) instruments every agent turn:
- Opens an instrumented `tracing::info_span!("agent_step")`.
- Dispatches execution events to `SupervisionTraceSink` (`platform/core/src/contracts/supervision.rs`).
- Alerts `SupervisedObserver` for anomaly detection (e.g. repeated tool failure loops, run budget exhaustion).

### 4.3 Standard Metric Instruments

- **`custos_task_total`:** Total tasks executed, broken down by terminal status (`Succeeded`, `Failed`, `Cancelled`).
- **`custos_step_duration_seconds`:** Execution latency per step broken down by worker role.
- **`custos_tokens_consumed_total`:** Input, output, and cached tokens categorized by provider and model.
- **`custos_system1_routing_ratio`:** Ratio of execution decisions handled by System 1 fast heuristics versus System 2 frontier models.
- **`custos_human_interruptions_total`:** Count of human interventions and approval prompts required.
- **`custos_verifier_pass_ratio`:** Percentage of task attempts passing automated verification on the first attempt.

---

## 5. Process Lifecycle & Zombie Elimination (INV-12)

Every external subprocess spawned by Custos (CLI harnesses, test runners, local model engines) must be governed by `ProcessDropGuard` (`platform/adapters/src/harnesses/stream/process.rs`):
- Spawned into an isolated process group via `setpgid(0)`.
- On drop, SIGKILL is sent to the entire process group (`-pid`) to guarantee that no orphaned or zombie background processes leak onto the developer workstation.

---

## 6. Privacy & Pre-Log Redaction Standards

- **Pre-Log Secret Redaction:** All logging through Rust's `tracing` framework automatically strips API keys, OAuth bearer tokens, and detected private keys prior to writing to stdout or disk journals.
- **Raw Prompt Safeguard:** Raw LLM prompts and model responses are withheld from production logs by default. Developers may enable local prompt logging strictly for offline debugging using `CUSTOS_LOG_PROMPTS=1`.
