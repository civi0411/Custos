# Competitive Analysis and Moat

> **Status:** Canonical Baseline v4.0  
> **Source:** Part I (§1.4) & Part VI (§42) Canonical Specification

This document examines the architectural positioning of Custos within the contemporary agent ecosystem, contrasting its approach against alternative solutions and detailing its defensible product moat.

---

## 1. Detailed Ecosystem Comparison Matrix

| Evaluation Dimension | LangGraph / CrewAI | Temporal / Workflow Engines | Claude Code / Aider / Cursor | **Custos** |
|---|---|---|---|---|
| **Core Operational Unit** | Graph node / Agent loop | Deterministic Activity / Workflow | Chat session / Diff buffer | **Durable, Evidence-carrying Task** |
| **State Ownership** | Framework memory / Python process | Central Workflow Cluster | Model session context | **Local Kernel (SQLite + WAL + Outbox)** |
| **Provider Independence** | Bound to framework abstractions | Zero built-in AI reasoning | Hardwired to specific vendor model | **Model-Agnostic (Provider-neutral core)** |
| **Judgment Mechanism (System One)** | Prompt chain / LLM router | None | Basic client-side heuristic | **Multi-Backend Judgment Fabric** |
| **Security & Permissions** | Unchecked tool execution | Static worker permissions | Local prompt confirmation | **Capability-based ExecutionPermits** |
| **Completion Criteria** | Model-generated text | Activity exit status code | Accepted file diff | **Verifiable Outcome Bundle (Multi-tier evidence)** |
| **Runtime Topology** | In-memory library | Client-Server cluster | Interactive local CLI/IDE | **Local-first Daemon + Ephemeral Workers** |

---

## 2. Unique Architectural Fusion

The architectural differentiation of Custos does not derive from novel protocol branding, but from the **systematic synthesis of ten foundational design tenets**:

1. **Durable Task State Independent of Provider Sessions:** Task state is managed authoritatively by the local Kernel, allowing seamless transitions across Codex, Claude, and local open weights without workflow disruption.
2. **Domain Packs with Ephemeral Workers:** Eliminates heavyweight monolithic agents; workers are spawned on demand for bounded subtasks and destroyed immediately upon completion.
3. **System One Encapsulating System Two:** Fast, deterministic, low-cost judgment (System One) guards, routes, and offloads expensive LLM reasoning (System Two).
4. **Typed Cognitive Escalation and Commit:** All cognitive escalations and state transitions are strictly typed through the RDC (Request-Decision-Challenge) protocol.
5. **Capability-Based Execution:** Security follows capability-based authorization; no tool executes without a cryptographically signed `ExecutionPermit`.
6. **Context Compilation with Provenance:** Prompt context compilation ranks items by empirical relevance, deduplicates aggressively, and preserves provenance.
7. **Evidence-Based Completion:** Tasks conclude only when objective machine verifiers pass, rejecting unverified natural-language model assertions.
8. **Human Attention as a First-Class Budget:** Explicit limits on human interruptions; approvals are aggregated by risk tier to prevent cognitive fatigue.
9. **Local-First Knowledge Continuity:** Codebases, operational logs, and accumulated project memory reside locally under developer control, operating fully offline.
10. **Artifact-Based Cross-Provider Handoff:** Inter-agent handoffs occur via content-addressed storage (CAS) artifacts rather than conversational chat history.

---

## 3. Defensible Product Moat

Custos establishes long-term defensibility through operational telemetry and verified feedback loops:

- **Decision Ledger:** An audit ledger recording human decisions, rationales, risk assessments, and corrections over time.
- **Calibrated Question Packs:** Empirically calibrated question banks enabling System One to rapidly classify intent and surface ambiguity.
- **Failure & Recovery Corpus:** A growing corpus of operational failure modes and automated recovery receipts.
- **Provider Compatibility Knowledge:** Empirical benchmarks documenting performance boundaries, cost profiles, and edge cases across model families.
- **Verifier & Evidence Recipes:** Reusable libraries of automated verification suites tailored to specific tech stacks and domains.

---

## 4. North-Star Metric

> **Verified outcomes per unit of total cost**

Where **Total Cost** is formally evaluated as:
$$\text{Total Cost} = \text{Financial Expense (API Tokens)} + \text{Latency Overhead} + \text{Human Interruptions} + \text{Remediation Cost}$$
