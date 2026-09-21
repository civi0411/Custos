# Design Philosophy, Invariants, and Architecture Decisions

> **Status:** Canonical Baseline v4.0  
> **Source:** Part II (§6-8) & Part VI (§43) Canonical Specification

This document codifies the **architectural guardrails** of Custos: inviolable system commitments, non-negotiable principles, and vetted strategic design decisions.

---

## 1. Ten Irreversible Decisions

These decisions form the architectural foundation and cannot be modified without a formal Major Architecture Review:

1. **Kernel Owns Authority:** The Kernel (not AI models or external agents) owns state, authority, budget, and commit logic.
2. **State Independent of Provider Session:** Task state is fully decoupled from AI provider sessions; switching providers does not lose task progress.
3. **Domain Packs Over Monolithic Agents:** Modular, domain-scoped packs replace monolithic "super-agents."
4. **Ephemeral Role-Based Workers:** Workers are provisioned for specific subtask roles and terminated immediately upon completion, maintaining zero permanent state.
5. **System One is a Multi-Backend Judgment Fabric:** Fast judgment (System One) is architecturally decoupled from deep deliberation (System Two); System One is pluggable and provider-neutral.
6. **Confidence Never Generates Capability:** High confidence scores never synthesize authority; capabilities originate solely from explicit Human Grants.
7. **Internal Typed Contracts:** Internal subsystems communicate exclusively via typed commands, events, and artifacts; open protocols like MCP are restricted to integration boundaries.
8. **All Side Effects Route Through Capability Gateway:** Every environment-mutating action routes through the Capability Gateway, validates an `ExecutionPermit`, and generates receipts.
9. **Completion Requires Evidence:** Task completion mandates verifiable evidence fulfilling the Task Contract; model assertions alone are rejected.
10. **Self-Measured Benchmarks:** All performance and token efficiency claims must derive from reproducible, empirical Custos benchmarks.

---

## 2. Seven Product Principles

| # | Principle | Detailed Definition |
|---|---|---|
| **1** | **Task-Centered** | The foundational operational unit is a durable **Task** with an explicit contract, not an ephemeral chat session or prompt stream. |
| **2** | **Human-Governed** | Humans retain sovereign authority: humans define intents, constraints, values, and approve high-risk operations. |
| **3** | **Local-First** | Sensitive data, task state, history, and source code remain on the local machine; zero data egress without explicit policy consent. |
| **4** | **Provider-Neutral** | Seamless interoperation across OpenAI Codex, Anthropic Claude, Google Antigravity, and Local LLMs (Ollama, llama.cpp, vLLM) without state loss. |
| **5** | **Evidence-Driven** | Work is recognized as complete solely through empirical verification: unit test passes, clean builds, static analysis receipts, and verified hashes. |
| **6** | **Capability-Based** | Security is enforced via capability grants: tools cannot execute outside permissions defined in signed `ExecutionPermit` tokens. |
| **7** | **Durable by Design** | State transitions persist reliably via SQLite WAL and CAS; power losses, crashes, and network disconnections resume deterministically. |

---

## 3. Eight System Invariants

These invariants are validated through formal verification and integration test suites:

| ID | Invariant | Formal Definition | Description |
|---|---|---|---|
| **I1** | **Authority Invariant** | `dispatch(a) → valid(Auth_a, t_dispatch)` | An action $a$ is dispatched only if valid authority exists at the dispatch timestamp. |
| **I2** | **Provenance Invariant** | `context ⊬ authority` | The appearance of information within prompt context never implies authorization to act. |
| **I3** | **Budget Invariant** | `reserve + settle ≤ ceiling` | Total settled expenditure plus active reservations must not exceed the defined task ceiling. |
| **I4** | **Evidence Invariant** | `SUCCEEDED(a) → receipt(a) ∧ verifier_passed(a)` | An action is marked successful only if it possesses an execution receipt and passes verification. |
| **I5** | **Continuation Invariant** | `resume(a) → StateValid ∧ AuthValid ∧ PreValid ∧ EffectsResolved` | Task resumption requires valid state, unexpired authorization, satisfied preconditions, and reconciled side effects. |
| **I6** | **Fencing Invariant** | `publish(result) → lease_epoch(result) = current_epoch` | Worker output is accepted only if the submission epoch matches the task's active lease epoch (split-brain prevention). |
| **I7** | **Privacy Invariant** | `secret ∉ C_a ∧ (egress(C_a) → privacy_gate(C_a))` | Secrets must not enter context payload $C_a$; all outbound traffic must pass privacy evaluation gates. |
| **I8** | **Fallback Invariant** | `fallback(model) → privacy_gate ∧ authority_gate` | Switching to fallback models requires re-evaluating privacy gates and capability authorization. |

---

## 4. Strategic Decisions: Retain, Modify, Defer, Reject (§43)

Through the evolution to Canonical Architecture v4.0, strategic decisions were categorized:

| Original Proposed Concept | Final Decision | Architectural Rationale & Direction |
|---|---|---|
| **Local-First** | **RETAIN** | Core foundation for privacy, security, and developer data sovereignty. |
| **Human-In-The-Loop** | **MODIFY** | Upgraded from passive confirmation prompts to **Human Sovereignty & Attention Budget**. |
| **Jev** | **MODIFY** | Prevent proprietary lock-in; retain Jev as a **first-class adapter** within the pluggable Judgment Fabric. |
| **LLM Agents** | **RETAIN** | Confined to the Deliberation Fabric for deep reasoning, supervised by System One and the Kernel. |
| **Three Monolithic Agents** | **MODIFY** | Replaced with modular **Domain Packs** (Engineering, Research, Personal) spawning ephemeral roles. |
| **Unrestricted Agent Chat** | **REJECT** | Eliminated free-form agent dialogue to prevent loops and hallucinations; replaced by **typed events and verified artifacts**. |
| **Ubiquitous Internal MCP** | **REJECT** | MCP introduces runtime overhead and lacks internal compile-time safety; restricted to **external tool/resource boundaries**. |
| **A2A Protocol** | **DEFER** | Deferred to the H3 Federation Mesh milestone; excluded from MVP core. |
| **Custom Terminology (JEP, UJE, CP)** | **REJECT** | Standardized on industry-aligned typed schemas and the **RDC (Request-Decision-Challenge)** protocol. |
| **Complex Memory Graph from Day 1** | **DEFER** | Begin with SQLite relational relations and FTS5; defer dedicated graph engines until empirically required. |
| **Full Research/Personal in MVP** | **DEFER** | Focus exclusively on delivering a robust **Engineering Domain Pack v1** in H1 before broadening scope. |
| **Autonomous Git Push / Deploy** | **REJECT FROM MVP** | All high-impact mutating side effects mandate explicit Human Exact-Payload Approval. |
| **Complete Repo Understanding Claim** | **MODIFY** | Replaced with **Measured Coverage** audited via ripgrep and tree-sitter AST queries. |
| **Academic Papers as Primary Goal** | **REJECT** | Prioritize **durable, verified, production-grade software** over speculative theoretical literature. |
