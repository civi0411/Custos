# SADE Platform Architecture: Living Commons & Four Canonical Blocks

> **Classification:** Platform Architecture Specification & Research Foundation 
> **Source of Truth:** Authoritatively governed by [Custos Master Specification](../../Custos.md). 
> **Documentation Triad:** Synchronized with [`codebase-architecture.md`](../development/codebase-architecture.md).

---

## 1. Core Philosophy: Anthropic Environment & The Living Commons

In Anthropic's agentic systems research, an agent is not an isolated prompt wrapper, but an organism operating within a well-defined execution environment:
- **Environment:** The Single Source of Truth (SSOT). Agents observe the world via typed Observations and mutate external state exclusively via Governed Actions.
- **Actuators & Tools:** Capabilities requiring explicit, signed `ExecutionPermit` tokens and producing tamper-evident `ExecutionReceipt` records.
- **The Living Commons (Agent Lake):** An open yet strictly governed multi-agent ecosystem where diverse agents (Claude Code, Codex, local scout models, specialized verifiers) collaborate symbiotically over Unified Resource References (URR) without stepping on each other's execution contexts.

```
 
 SADE LIVING COMMONS 
 (Agent Lake Substrate) 
 
 
 
 
 
 HETEROGENEOUS FLEET AQUATIC SUBSTRATE 
 - Claude Code / Codex - Universal Resource Reg 
 - Local S1 / S2 Scouts urr://[comp]/[kind]/id 
 - Specialized Verifiers - Extensible Lake Event 
 - Background Evaluators modality/topic bus 
 - Domain Specialists - CAS Immutable Storage 
 
 
 1. Emits Work & Products 3. Wakes Up & Refines 2. Content Addressed
 
 
 CROSS-DOMAIN PIPELINE 
 Research (Claim) Coding (AST/Patch) Quant (Factor/PBO) Assistant (Outbox) 
 
 
 4. Closed Feedback Loop
 
 T6 REPAIR & OPTIMIZE 
 Self-improving Ecosystem 
 
```

---

## 2. Four Canonical Platform Blocks

The shared platform is partitioned into four canonical blocks following Ports-and-Adapters (Hexagonal) architecture:

### 2.1. `platform/contracts` (`custos-contracts`) — Ring 0
- **Responsibility:** Pure domain models, port interfaces, state definitions, error taxonomy, and event schemas.
- **Inviolable Invariant:** Zero-I/O (INV-01). Prohibits `tokio`, `rusqlite`, `reqwest`, `std::fs`. Logic consists strictly of pure types, deterministic state transitions, and validation rules.
- **Core Modules:** `authority`, `context`, `contributions`, `conversation`, `evidence`, `execution`, `identity`, `inference`, `resources`, `task`.

### 2.2. `platform/storage` (`custos-storage`) — Ring 1
- **Responsibility:** Relational persistence under SQLite WAL mode and Content-Addressable Storage (CAS).
- **Core Architecture:** Single-writer connection pool with dedicated read-only connections, dynamic partition engine (`attach_partition` / `detach_partition`), Transactional Outbox pattern, and crash reconciliation.
- **Core Modules:** `artifacts`, `connection`, `recovery`, `repositories`, `store`, `transactions`.

### 2.3. `platform/integrations` (`custos-integrations`) — Ring 1
- **Responsibility:** External world interfaces, model providers, execution sandboxes, and hardware compute bridges.
- **Core Architecture:** Model provider codecs (OpenAI, Anthropic, Gemini, Groq, DeepSeek, Ollama), local `llama.cpp` inference, OS sandboxing (Seatbelt, Bubblewrap), PTY terminal manager, MCP Federation Hub, A2A Agent Cards, and remote compute relays (Colab, Kaggle, SSH).
- **Core Modules:** `compute`, `connectors`, `credentials`, `filesystem`, `harnesses`, `host`, `models`, `notebook`, `protocols`, `sandbox`, `terminal`.

### 2.4. `platform/engine` (`custos-engine`) — Ring 2
- **Responsibility:** Autonomous agent loop execution, cognitive coordination, task scheduling, and verification.
- **Core Architecture:** Governed execution loop with `ProcessDropGuard`, S1 FastPathFilter, S2 Replanner, Pareto OI Selector, T0–T6 Topologies, Completion Gate verification engine, and pack registry.
- **Core Modules:** `application`, `authority`, `components`, `context`, `contracts`, `conversation`, `evidence`, `execution`, `orchestration`, `resources`, `supervision`, `task`.

---

## 3. The SADE Living Commons Mechanics

### Mechanism 1: Universal Resource References (URR)
Every resource produced within the ecosystem adheres to the canonical URI structure:
$$\text{URR} = \texttt{urr://[component]/[kind]/[id]\#[sha256]}$$

Examples:
- `urr://science/claim/claim_8921#e3b0c442...` (Ground scientific claim with literature citation)
- `urr://engineering/patch/patch_3120#a1b2c3d4...` (Code patch verified against AST tests)
- `urr://quant/factor/vol_momentum#f9e8d7c6...` (Alpha factor verified against PBO criteria)
- `urr://assistant/brief/brief_0012#77aa88bb...` (Executive briefing ready for human outbox review)

Agents do not duplicate large binary artifacts; they exchange immutable URR pointers verified via CAS content digests.

### Mechanism 2: Extensible Lake Event Substrate
Agents broadcast discoveries and milestones via structured lake events:
```rust
let event = PipelineEvent::lake(
 "quant.alpha.discovered",
 "quant_researcher_agent",
 "quant",
 vec!["urr://science/claim/claim_8921#e3b0c442".to_string()],
 json!({ "sharpe": 2.45, "pbo": 0.04 }),
);
event_bus.publish(event);
```
Subscribers (such as risk evaluators or backtest runners) react asynchronously to advance the workflow.

### Mechanism 3: Cross-Domain Symbiosis
Workflows cascade smoothly across the four specialized workbenches:
1. **Research Agent:** Extracts literature claims and publishes verified claims with citation anchors.
2. **Coding Agent:** Consumes claim URRs, implements algorithms in an isolated Git worktree, and runs automated tests.
3. **Quant Agent:** Executes backtests against historical market data without lookahead bias.
4. **Assistant Agent:** Synthesizes results into executive drafts awaiting one-click human outbox approval.

---

## 4. Automated Verification Commands

```bash
# 1. Verify Rust workspace architecture boundaries
cargo xtask check-boundaries

# 2. Verify platform engine tests
cargo test -p custos-engine --lib --offline

# 3. Verify desktop canvas smoke suite
node tests/desktop/canvas-smoke.mjs
```
