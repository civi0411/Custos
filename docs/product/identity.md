# Product Identity & Thesis

> **Status:** Canonical Baseline v4.0  
> **Source:** Part I (§0-1) Canonical Specification

---

## 1. What is Custos?

**Custos is a Human-Centered Agentic Work Runtime — local-first and model-agnostic — that turns human intent into durable, resumable, cross-provider, and evidence-backed work.**

Custos is **not**:
- Not a self-proclaimed "super-agent".
- Not an API model chat aggregator or wrapper.
- Not a bloated generic framework.

Custos is a **specialized runtime layer** positioned between:
- **The Human Principal:** Holding intent, ethics, constraints, and ultimate approval sovereignty.
- **AI Reasoning Providers:** OpenAI Codex, Anthropic Claude, Google Antigravity, and local SLMs/LLMs.
- **Workspace Knowledge:** Git repositories, AST indexes, notes, and local files.
- **Execution Tools & OS:** Shell environments, linters, compilers, test runners, and external MCP tools.
- **Judgment Fabric (System One):** Fast, deterministic reflection, invariant checks, and risk triage.

---

## 2. Product Thesis & System Formula

> ### Product Thesis
> **Custos turns human intent into controlled, resumable, cross-provider and evidence-backed work.**

### The Core Operational Unit: Task
The fundamental operational unit is the **Durable Task** — never an ephemeral chat session, model context window, or isolated tool call.

```text
Human intent
--> Task contract
--> Decision cases and workflow
--> Ephemeral workers + tiered sandboxes + model providers
--> Content-addressed artifacts + independent evidence
--> Human-governed verified outcome bundle
```

### System Formula
$$\text{Custos} = \text{Durable Task Control} + \text{Cognitive Control Fabric} + \text{Capability-Governed Execution} + \text{Knowledge/Evidence Fabric} + \text{Human Sovereignty}$$

---

## 3. The Three Pillars

| Pillar | Technical Mechanism | User Value |
|---|---|---|
| **Model-Agnostic** | Task memory & state isolation | Switch seamlessly between Claude, Codex, or local models without breaking execution progress or losing context history. |
| **Evidence-Carrying** | Evidence-carrying actions & outcome bundles | Eliminates reliance on ungrounded AI assertions; outcomes require verifiable test receipts, clean git diffs, and proof artifacts. |
| **Judgment Infrastructure** | First-class pluggable System One | Isolates fast deterministic reflection (risk triage, classifier checks) from deep LLM deliberation, slashing 60-80% of token latency and cost. |

---

## 4. Architecture in Six Sentences

1. **The Kernel owns the truth:** The Task Kernel owns state, authority, budget, and commits; models are merely ephemeral reasoning compute.
2. **Workers are disposable:** Role-based workers are instantiated on demand for specific subtasks and terminated immediately upon completion, holding no persistent state.
3. **Judgment governs reasoning:** System One (Judgment Fabric) envelopes and constrains System Two (Deliberation LLMs) through explicit decision contracts.
4. **No implicit capabilities:** No shell command or API request executes without an explicit `ExecutionPermit` validated by the Capability Gateway.
5. **Acceptance is evidence-backed:** A task only transitions to `Completed` when it satisfies the objective completion gates defined in the Task Contract.
6. **Human sovereignty is absolute:** Humans hold attention budgets and approve high-risk actions through cryptographically bound exact-payload diffs.

---

## 5. Target Personas & Jobs-To-Be-Done (JTBD)

### Target Personas
- **Staff+ Software Engineers / Tech Leads:** Need multi-hour complex refactorings on large repos with worktree isolation, strict test verification, and zero risk to active branches.
- **Deep Researchers / Analysts:** Need to synthesize dozens of technical documents into verifiable claim-evidence matrices with strict provenance and zero hallucination.
- **Technical Operators / Power Users:** Need deterministic daily workflow automation (email triage, schedule synthesis, ticket updates) with absolute guarantees against unauthorized data mutation or egress.

### Jobs-To-Be-Done (JTBD)
- When I start an overnight complex technical task, I want the runtime to execute autonomously, retry recoverable errors, or pause safely for approval without losing context or state.
- When I delegate a bug fix to AI, I want a clean patch accompanied by verified test and lint receipts, rather than unverified assertions in a chat window.
- When API costs spike, I want to route routine subtasks to smaller models or local SLMs without rebuilding my pipeline.

---

## 6. Non-Goals

To maintain uncompromising discipline, Custos explicitly defines the following non-goals:
- **No Conversational Chatbots:** Custos is not designed for open-ended conversation, entertainment, or casual search.
- **No Non-Technical Drag-and-Drop Builders:** Custos targets software engineers, technical operators, and knowledge professionals.
- **No Unconstrained Autonomy:** Custos rejects unrestricted agents that operate without human approval gates or financial budgets.
- **No Proprietary Cloud Lock-In:** Custos is not a centralized SaaS requiring source code egress to external cloud storage.
