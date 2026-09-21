# CUSTOS — FINAL PROJECT DIRECTION, ARCHITECTURE & LOGIC

> **Status:** Canonical product direction
> **Version:** 1.0
> **Date Locked:** 2026-09-21
> **Language:** English (Technical contract names preserved)

## 0. How to Use This Document

This document fully answers five core questions:
1. What is Custos fundamentally?
2. What problems does it solve and how does it differ from existing agents?
3. What are all the components of the final architecture?
4. How does a request flow through the system logically?
5. How must each component be built, tested, and integrated?

This document describes the destination of the product. It does not mean every module must be built immediately. 

Three levels to distinguish:
- **Product Direction:** Long-term value and positioning of Custos.
- **Target Architecture:** The stable architecture the product aims for.
- **MVP Slice:** The smallest slice proving the architecture works.

**Related Documents:**
- `README.md`: Public, concise introduction.
- `docs/development/roadmap.md`: MVP framework assembly blueprint.
- `dev_docs/README.md`: Role division and team coordination.
- **This Document:** The authoritative source for understanding the entire product direction and logical flow.

## 1. One-Paragraph Summary

Custos is a local-first, human-governed runtime for work involving AI agents. It does not replace Codex, Claude, local models, MCP servers, IDEs, Obsidian, or Notion. Custos sits in the middle to turn these fragmented tools into a continuous workflow: goals are kept in a durable Task; context is intentionally selected; multiple providers can continue each other's work; every side-effecting action requires permission; every important outcome becomes an artifact and receipt; tasks only complete when evidence is present; humans always retain final decision authority over risky or irreversible actions. Custos does not attempt to create a single all-knowing super-agent. It provides the operating environment where humans, specialized agents, fast models, tools, and evidence can collaborate coherently.

## 2. Core Problem Custos Solves

### 2.1. The Current Ecosystem is Powerful but Fragmented
A developer might simultaneously use:
- Codex to edit code
- Claude to read architecture
- A local model to classify sensitive data
- MCP to invoke tools
- Terminal to run tests
- GitHub to manage changes
- Obsidian/Notion to store knowledge
- Browser/Search to research

Each tool is powerful, but they do not share a common source of truth. Consequences:
- Each chat holds a fragment of context.
- Switching providers requires repeating context.
- Agents do not know which actions actually executed.
- Chat history is conflated with memory.
- No unified permission model.
- Models may claim "tests passed" without receipts.
- Closing the terminal or crashing loses state.
- Over-sending context increases tokens and reduces accuracy.
- Humans must manually stitch results across applications.

### 2.2. Common Pitfalls
Agent systems often gravitate toward two extremes:
- **Chat Wrappers:** Easy to use but weak on state, permissions, and evidence.
- **Autonomous Swarms:** Multi-agent but hard to control, debug, and expensive.

Custos chooses a third path: A durable, governed, task-centric runtime using agents as replaceable capabilities.

### 2.3. Differentiating Value

| Pain Point | Custos Solution |
|---|---|
| Context isolated in chats | Context is compiled from Tasks, workspaces, and artifacts |
| Provider lock-in | Provider adapters and neutral continuations |
| Uncontrolled tool execution | Capability, Grant, Permit, and policy |
| Agents self-reporting success | Verifiers and evidence closure |
| Crashes lose progress | Durable state, event logs, outbox, and recovery |
| High token costs | Context selection, System One, and budget controls |
| Disjointed multi-agent setups | Shared Task state and typed handoffs |
| Humans excluded from the loop | Approvals, steering, pausing, cancellation, and overrides |
| Cluttered memory | Memory categorized by scope, provenance, and lifecycle |

## 3. Product Positioning

### 3.1. What is Custos?
- A local runtime on the user's machine.
- A task operating layer for AI-assisted work.
- A coordination kernel between humans, providers, tools, and evidence.
- A governance boundary for side effects.
- A continuity layer across models, sessions, and interfaces.
- A platform for Domain Packs (Engineering, Research, Personal Assistant).

### 3.2. What Custos is Not
- Not a new foundation model.
- Not a new chatbot.
- Not a fork of Codex or Claude.
- Not a generic multi-agent framework.
- Not a permanent persona agent system.
- Not a database that dumps chat history and calls it memory.
- Not autonomous software with full machine authority.
- Not a vague "AI OS" without clear semantics.
- Not a replacement for Git, IDEs, Obsidian, Notion, or MCP.

### 3.3. Target User
Primary user: Developer or research-minded engineer who:
- Works across multiple repositories.
- Uses multiple coding/reasoning providers.
- Needs context preserved across sessions.
- Wants control over cost and data.
- Demands evidence-backed results.
- Wants to retain decision authority, not cede total control.

### 3.4. Product Promise
Assign a goal to Custos, select automation level, observe, and intervene when needed; Custos maintains continuous work across agents, controls all side effects, and only presents completion when traceable evidence is provided.

## 4. Invariants
These invariants supersede any specific framework:
- **Task-centered:** The Task, not the chat, is the unit of work.
- **Human-governed:** Humans own intent, constraints, and commitments.
- **Local-first:** State, policy, artifacts, and sensitive knowledge prioritize local storage.
- **Provider-neutral:** Providers contribute reasoning but do not own the Task.
- **Capability-based:** Side effects only occur via scoped permissions.
- **Evidence-driven:** Model outputs are not automatically evidence.
- **Durable-by-design:** Critical transitions must persist.
- **Selective context:** Send exact context, not entire histories.
- **Separation of intelligence:** Fast judgment is separate from deep deliberation.
- **Replaceable integrations:** External protocols do not dictate domain models.
- **Explicit uncertainty:** When unsure, abstain or escalate.
- **Observable decisions:** Critical decisions must be explainable and auditable.

**Technical Invariants:**
- LLMs cannot grant themselves permissions.
- UIs cannot mutate the database directly.
- Adapters do not own workflow state.
- Side effects require valid ExecutionPermits.
- Retries require idempotency semantics.
- Tasks cannot transition to Succeeded without mandatory evidence.
- System One only returns judgments; Kernel decides transitions.
- Restarts must not lose committed transitions.

## 5. Mental Model for Software Engineers
Treat every AI provider as an external dependency:

| Characteristic | Design Implication |
|---|---|
| Nondeterministic | Use schema validation, fixtures, and evaluations |
| Expensive | Implement budgets, caching, and context selection |
| Slow | Support streaming, timeouts, and cancellation |
| Fallible | Require verifiers and fallbacks |
| Potentially unsafe | Deny direct authority |
| Provider-specific | Standardize via adapters |
| Externally stateful | Store external session refs, retain source of truth locally |

Custos operates closer to a durable local control system than an "AI chatbot".

## 6. Cognitive Model: Human + System One + System Two

### 6.1. Human
Provides goals, values, priorities, constraints, risk tolerance, irreversible decisions, context feedback, and final approval. Can approve, deny, steer, pause, resume, cancel, adjust budgets, swap providers, or demand more evidence.

### 6.2. System One — Judgment Fabric
Handles narrow, fast, cheap questions: relevant files, action risk, context sufficiency, secret detection, candidate reranking, loop detection, continuation decisions, and claim-evidence matching.
Backends: deterministic rules, lexical heuristics, local embeddings/rerankers, small local models, or remote classifiers (if policy allows).

### 6.3. System Two — Provider Fabric
Handles deep deliberation: understanding large repos, root cause analysis, planning, writing/reviewing code, synthesizing docs, researching, and architectural trade-offs.
Backends: Codex, Claude, OpenAI, Gemini, local models, or specialized agents.

### 6.4. Power Dynamics
- Human defines goals and authority.
- System One recommends or gates via policy.
- System Two analyzes and proposes.
- Kernel decides state transitions.
- Gateway decides action execution.
- Verifier decides if evidence meets conditions.

## 7. Overall Architecture

Custos is structured into seven logical planes. These planes are responsibility boundaries, not necessarily seven services.

| Plane | Role |
|---|---|
| **Experience Plane** | CLI, IDE extension, dashboard, and human interaction |
| **Task & Control Plane** | Task state, commands, events, policy, approvals, budgets |
| **Intelligence Plane** | Judgment backends and deep-reasoning providers |
| **Capability Plane** | Tools, execution, MCP, filesystem, Git, network, services |
| **Context & Knowledge Plane** | Repo intelligence, memory, search, context compilation |
| **Evidence Plane** | Artifacts, receipts, verifiers, completion rules |
| **Infrastructure Plane** | Persistence, process supervision, telemetry, secrets, config |

### 7.1. Why is the Kernel Central?
The Kernel is the only component with sufficient information to:
- Know the current Task state.
- Validate commands.
- Manage budgets.
- Select the next step.
- Wait for human input.
- Recover from crashes.
- Compare evidence against acceptance conditions.

The Kernel does not reason on behalf of models and does not execute tools itself.

### 7.2. Control Plane vs. Data Plane
- **Control Plane:** Task commands, policy, approval, scheduling, transitions.
- **Data Plane:** Context, model streams, tool input/output, artifacts, receipts.

Large payloads are not pushed into the event log. The Event Log holds identity and metadata; the Artifact Store holds large content.

## 8. Components and Ownership Boundaries

| Component | Owns | Does Not Own |
|---|---|---|
| **Core Domain** | Entities, value objects, invariants, ports | Database, provider SDKs |
| **Runtime/Kernel** | Workflow, transitions, recovery, budgets | Model reasoning |
| **Provider Adapter** | External sessions, event normalization | Task authority |
| **Judgment Adapter** | Structured judgment | Side effects |
| **Capability Gateway** | Authorization, execution dispatch | Product intent |
| **Workspace Engine** | Inventory, symbols, search, snapshots | Workflow state |
| **Context Compiler** | Selecting/packaging context | Long-term source of truth |
| **Memory Service** | Memory items, scope, provenance, lifecycle | Raw chat dumps |
| **Artifact Store** | Immutable content, identity | Completion decisions |
| **Verifier** | Claim/acceptance checks | Altering output to pass itself |
| **Store** | Atomic persistence | Business policy |
| **UI/CLI** | Projection, commands | Direct state mutation |

## 9. Core Domain Model

### 9.1. Main Entities
- **Task:** User-level goal and aggregate root.
- **Run:** A single attempt to execute a Task.
- **Step:** A workflow unit defined by a Pack.
- **ActionIntent:** Proposal to create a side effect.
- **ApprovalRequest:** Request for human/policy decision.
- **CapabilityGrant:** General permission within a scope.
- **ExecutionPermit:** One-time permission bound to a specific action.
- **Receipt:** Proof of an executed action.
- **Artifact:** Immutable content with provenance.
- **EvidenceItem:** Accepted artifact/receipt for a claim.
- **ProviderSession:** Reference to an external session.
- **Judgment:** Evaluation result with schema, confidence, and reason.
- **Continuation:** State package for resuming or handoff.
- **MemoryItem:** Knowledge with scope, provenance, and lifecycle.
- **WorkspaceSnapshot:** Logical snapshot of a repo/doc at a specific time.
- **DomainPack:** Workflow, policy, and verifiers for a domain.

### 9.2. Task is Not a Simple Row
The Task is an aggregate protecting invariants. Changes occur via:
`Command → validate(current state, policy) → Domain Events → new state`
State cannot be arbitrarily modified by controllers, adapters, or SQL scripts.

### 9.3. Proposed Task State Machine
`Draft` → `Ready` (define scope) → `Running` (start run)
`Running` can transition to:
- `WaitingHuman` (needs input)
- `WaitingApproval` (risky action)
- `Verifying` (candidate outcome)
- `Paused` (pause)
- `Cancelled` (cancel)
- `Failed` (unrecoverable)

`Verifying` transitions to `Succeeded` if closure conditions are met, or back to `Running` if evidence is insufficient.

### 9.4. Terminal States
- **Succeeded:** Acceptance conditions proven by evidence.
- **Failed:** Cannot continue within policy/budget.
- **Cancelled:** Human or system intentionally stopped.
"Model replied" does not equal Succeeded.

## 10. Standard Coordination Loop
1. Human Intent
2. Create/Update Task
3. Resolve Domain Pack
4. Inspect Task and Workspace State
5. Compile Context
6. Ask System One or System Two
7. Validate Structured Output
8. Convert Proposed Tool Use into ActionIntent
9. Evaluate Policy and Risk
10. Request Human Approval when required
11. Mint ExecutionPermit
12. Execute Capability
13. Persist Receipt and Artifacts
14. Verify Claims and Acceptance Conditions
15. Continue / Ask Human / Switch Provider / Complete

### 10.1. Crucial Transaction Boundaries
Before dispatching an external effect, Custos persists: intended action, idempotency key, permit identity, expected state, outbox item.
After effect returns, Custos persists: execution result, receipt, artifact references, resulting events, next state.

### 10.2. No Infinite Agent Loops
Each loop has: step budget, token/cost budget, wall-clock deadline, repeated-action detector, cancellation token, escalation conditions, max tool retries.

## 11. End-to-End Illustrated Flow
**Request:** "Find the cause of the payment test failure, fix it in an isolated branch, run related tests, and give me evidence before committing."

1. Kernel creates `bug_fix` Task and Run.
2. Workspace Engine captures Git HEAD, dirty state, inventory.
3. Context Compiler selects manifest, symbols, tests, history.
4. Provider analyzes; output validated via schema.
5. Edits/commands convert to ActionIntents.
6. Risk engine evaluates path/command/network/reversibility.
7. Human approves scope.
8. Gateway mints permit bound to task, action hash, scope, TTL.
9. Executor runs in Git worktree.
10. Diff, stdout/stderr, exit code, duration become artifacts/receipts.
11. Verifier runs test profile independent of provider's claims.
12. Closure rules check for sufficient evidence.
13. Human decides to keep patch, commit, revise, or cancel.

## 12. Provider Fabric and Multi-Model Continuity

### 12.1. ProviderPort
Providers must support common semantics where possible: `start`, `resume`, `send_control`, `cancel`, `health`, `capabilities`. Adapters declare capabilities (e.g., supports resume, structured output, local execution).

### 12.2. Normalized Provider Events
Standard events: SessionStarted, TextDelta, ReasoningSummary, PlanProposed, ToolIntentProposed, ArtifactProposed, UsageReported, ProviderWarning, SessionCompleted, SessionFailed.

### 12.3. ContinuationContract
When handing off (e.g., Codex to Claude), Custos creates a typed continuation instead of copying the whole chat. It includes: goal, constraints, current state, completed steps, open questions, decisions (with rationale refs), artifacts, pending actions, budgets. Goal: sufficient to continue, shorter than transcript, provider-neutral.

### 12.4. Provider Selection
Selected via policy: task type, required capability, privacy class, cost/latency budget, provider health.

### 12.5. Handoff is Not Agent-to-Agent Chat
Agent A does not message Agent B. Flow: Provider A output → normalize → persist artifacts/decisions → build continuation → Kernel selects Provider B → Provider B receives bounded context.

## 13. Judgment Fabric — Replaceable System One

### 13.1. JudgmentPort
Request includes: question_type, typed input, policy context, latency/cost constraints.
Result includes: decision/candidates, confidence, reason codes, evidence refs.

### 13.2. Judgment Categories
Routing, Screening (secrets/injections), Relevance, Risk, Quality (completeness), Control (continue/stop/escalate), Deduplication, Compression.

### 13.3. Backend Ladder
Prioritize cheap/deterministic backends: Rules → Lexical scorer → Local embedding/reranker → Small local model → Deep provider (only when necessary).

### 13.4. Conformance over Hard Dependency
Jev/Layla are just adapters. All backends must pass standard tests (schema, timeout, invalid payload, confidence range).

### 13.5. Shadow Mode
New backends run in shadow mode first to compare against human/label outcomes without affecting real decisions.

## 14. Capability Gateway and Permission Model

### 14.1. What is a Capability?
Powers that create side effects: read/write files, search repo, run commands, create worktrees, call MCP tools, network access.

### 14.2. Grant vs. Permit
- **CapabilityGrant:** General permission (who, which task, resource scope, constraints, expiry).
- **ExecutionPermit:** Narrow, one-time token for a specific action (action payload hash, path/network scope, allowed tool, TTL).

### 14.3. Action Pipeline
Tool proposal → normalize ActionIntent → validate schema → calculate risk → match policy/grant → request approval if needed → mint permit → execute → record receipt → consume permit.

### 14.4. Risk Classes
- **R0 Read-only:** Auto if in scope.
- **R1 Reversible local:** Edit in worktree (auto based on user mode).
- **R2 External/restricted:** Network, package install (requires approval).
- **R3 Sensitive:** Secrets, private data (explicit approval + redaction).
- **R4 Irreversible:** Push, deploy, delete (mandatory human confirmation).

### 14.5. MCP
MCP is an integration protocol, not an internal event bus or authority model. Custos must: discover tools via MCP, normalize schemas, apply its own policy, mint permits before invocation, isolate MCP server lifecycles.

## 15. Context, Workspace Intelligence, and Token Economy

### 15.1. Context != Memory
Memory is knowledge persisting across tasks. Context is the specific data bundle selected for a single inference call.

### 15.2. Repo Intelligence Pipeline
Repository → Git metadata → inventory → symbol extraction → dependency mapping → structural index → task-aware retrieval → context bundle.

### 15.3. Context Compiler
Takes task goals, step, budgets, workspace snapshots, memory candidates, and artifact refs to produce a ContextBundle (instructions, summaries, selected ranges, symbols, prior decisions).

### 15.4. Context Recipes by Phase
- **Triage:** Manifest, errors, changed files.
- **Diagnose:** Relevant symbols, tests, logs.
- **Plan:** Constraints, architecture rules.
- **Implement:** Exact file ranges, style rules.
- **Review:** Diff, requirements, risks.

### 15.5. Token Reduction Strategy
Inventory over content, retrieve by symbol not file, progressive disclosure, cache by content hash, use continuations instead of transcripts, deduplicate chunks.

## 16. Memory Architecture

### 16.1. Memory Types
- **Task Memory:** Single task (decisions, hypotheses).
- **Workspace Memory:** Single repo (architecture rules, build commands).
- **User Memory:** Single person (preferences, approval style).
- **Domain Memory:** Single Pack (research rubrics).
- **Episodic Memory:** Single run/session (memorable outcomes).
- **Evidence Memory:** Content-addressed (receipts).

### 16.2. Minimal MemoryItem
Includes scope, kind (fact/preference/decision), content, provenance refs, confidence, validity periods.

### 16.3. Memory Write Policy
Do not save all chat logs. Write memory only when it has reuse value, defined scope, provenance, and respects privacy policies.

### 16.4. Obsidian/Notion Integration
Note apps are external knowledge surfaces. Import/export Markdown, index by identity, retain backlink URIs. Mutations require capabilities and approvals.

## 17. Artifact, Receipt, Evidence, and Verification

### 17.1. Distinctions
- **Output:** Model claims "tests passed".
- **Artifact:** Immutable file/patch stored safely.
- **Receipt:** Record of a test command that actually executed.
- **Evidence:** Receipt/artifact accepted by a verifier for a claim.

### 17.2. Artifact Model
Stored in content-addressed store. Database holds metadata (hash, media, producer, task, integrity status).

### 17.3. ExecutionReceipt
Contains action/permit IDs, executable, sanitized args, start/duration, exit status, stdout/stderr refs, redacted env fingerprint.

### 17.4. Verifier Profiles
Defines what command to run, expected exit codes, required output patterns, and which claims are supported.

### 17.5. Closure Rules
A `bug_fix` task only Succeeds if: patch artifact exists, targeted tests have passing receipts, regression profile passes, no unresolved high-risk findings, and human approval is present for risky changes.

## 18. Durable Runtime, Eventing, and Recovery

### 18.1. Persistence Model
Local-first SQLite containing tasks, runs, events, outbox, approvals, permits, artifacts, receipts, leases, budgets, memory.

### 18.2. Atomic Writes
State-changing transactions write aggregate state, events, outbox messages, and idempotency records simultaneously. Roll back completely on failure.

### 18.3. Outbox/Inbox
Outbox ensures committed intents are dispatched. Inbox prevents duplicate effects. Handlers must be idempotent.

### 18.4. Leases
Workers hold leases with owner ID, heartbeat, expiry, and fencing tokens to prevent split-brain commits.

### 18.5. Recovery
On restart: load non-terminal tasks, clear expired leases, reconcile outbox, check provider external state, do not blindly retry non-idempotent effects.

### 18.6. Crash Semantics
Promises state transitions are not lost post-commit, effects have identity, ambiguities are exposed, and dangerous actions are never retried blindly.

## 19. Domain Pack Architecture

Core contains no domain-specific logic. Packs define task types, workflows, roles, context recipes, capability policies, verifiers, and schemas.

### 19.1. Engineering Pack
MVP focuses on: `repo_explain`, `bug_fix`. Future: feature implementation, code review, refactoring, test generation.

### 19.2. Research Pack
Workflows: question decomposition, source discovery, claim extraction, synthesis. The Research Pack forbids creating citations without source artifacts.

### 19.3. Personal Assistant Pack
Workflows: synthesis, planning, emails. Gated heavily (sending email is R4). User memory requires privacy flows.

### 19.4. Pack Isolation
Packs cannot directly access DB, mint permits, bypass Kernel, declare model outputs as evidence, embed credentials, or alter core state machines.

## 20. Engineering Pack: Full Coding Flow

### 20.1. Repo Onboarding
Register workspace → validate Git state → create snapshot → detect tools → inventory files → extract symbols → create RepoProfile → human confirms critical commands.

### 20.2. repo_explain
Question → classify intent → retrieve symbols → ask provider (citations required) → validate citations → return explanation + evidence map.

### 20.3. bug_fix
Failure report → reproduce via receipt → retrieve code → diagnose → propose plan → approval → isolated edit → targeted verification → regression verification → evidence bundle.

### 20.4. Provider Specialization
Provider A for repo understanding, Provider B for implementation, Provider C for review. Do not default to "more models is better"; use additional providers only when expected information gain outweighs cost.

## 21. Human-in-the-Loop Modes
Selected per Task, not globally permanent.
- **Observe:** Read/analyze, no side effects.
- **Suggest:** Propose plans, human executes.
- **Guided:** Auto low-risk, ask for medium/high-risk.
- **Delegated:** Auto within narrow grant, pause at checkpoints.
- **Locked:** All side effects require approval.

## 22. Security and Privacy Model

### 22.1. Threat Assumptions
Do not trust: model outputs, repo contents, prompts embedded in documents, MCP servers, package install scripts, stale approvals.

### 22.2. Defense Layers
Deny-by-default, loopback binding, scoped filesystem, direct `argv` execution (no shell interpolation), environment allowlist, secret redaction, OS Keychain, worktree isolation, payload-bound permits.

### 22.3. Prompt Injection
Repo content is untrusted. Context Bundles must structurally separate: system policy, pack instructions, human instructions, and retrieved untrusted content.

### 22.4. Local-First != Offline-Only
Custos can call remote providers but must: know what is sent, redact secrets, show destination to user, respect data classifications, and retain control state locally.

## 23. Observability and Explainability

### 23.1. Three Layers of Observation
- **System:** Latency, errors, queues, DB.
- **Task:** State, step, budgets, pending approvals.
- **Intelligence:** Provider choices, context size, confidence.

### 23.2. Explain Next Step
At any point, Custos must answer: Where is the Task? Why? Who/what is it waiting for? What is the next action? What budget/permission will be used?

## 24. Cost, Latency, and Performance
Track tokens, model costs, wall time, retry counts. Optimize by resolving via rules first, using System One to filter, sending small/precise context, caching by hash, and stopping when marginal gain is low.

## 25. Process Topology and Local Deployment
**MVP Default:**
- `custosd`: Kernel, API, persistence, policy, gateway (Rust Daemon).
- `custos`: CLI client.
- Sidecars: Only when runtime/dependency barriers require it (e.g., Node bridge for Claude SDK).
Monolith-first control plane; no Kubernetes for local MVP.

## 26. Target Repository Architecture
```
custos/
├── apps/ (custosd, custos)
├── crates/ (core, runtime, providers, judgment, capabilities, workspace, verification)
├── adapters/ (Codex, Claude, local models, MCP, Obsidian)
├── sidecars/
├── packs/ (engineering, research, personal)
├── extensions/ (vscode)
└── tests/ evals/ docs/
```
Dependencies flow inward: `apps` → `adapters` → `runtime` → `core`. Core imports no provider SDKs or databases.

## 27. Contracts
Core contracts must stabilize early (Task events, ProviderPort, JudgmentPort, CapabilityPort). Schemas must be versioned, additive, with clear error models (category, retryability, redacted context).

## 28. Framework and OSS Integration Strategy
No arbitrary "clone and mix". Upstreams used via strict modes: Dependency, Sidecar, Reference-only, or Fork (rare). 
Adopt: Tokio, Axum, rusqlite, Serde, BLAKE3.
Study but do not blindly copy: large agent gateways, cloud-native wrappers.

## 29. MVP: Proving the Product
**Two Workflows Only:** `repo_explain` and `bug_fix`.
**One Required Loop:** create task → profile repo → compile context → invoke provider → propose action → approve → execute in worktree → collect receipt → verify → build evidence bundle → recover after restart → complete.
**Out of Scope for MVP:** Full Research/Personal packs, GraphRAG, A2A federation, Kubernetes sandboxing, heavy UI.

## 30. Testing and Evaluation
Test pyramid: Unit (invariants), Property (paths), Contract (ports), Integration (SQLite/processes), E2E (canonical workflows with FakeProvider), Recovery (crash injection), Security (symlinks, injections).
FakeProvider must simulate success, malformed outputs, timeouts, crashes, and partial streams without real API keys in core CI.

## 31. Definition of Done
A feature is Done when it has clear domain contracts, handles invalid cases, has tests, telemetry, error redaction, recovery semantics, and security reviews. A workflow is Done when it runs E2E with fixtures and produces an auditable evidence bundle.

## 32. Roles and Division of Labor
- **Vi (Product & AI/Research Lead):** Owns product vision, use cases, Domain Pack semantics, provider strategy, AI evaluations, prompt design.
- **SE Lead (Platform & Software Engineering):** Owns Rust workspace, core runtime, persistence, execution sandbox, capability gateway, CI/reliability.
- **Shared:** State machine semantics, ports, continuations, security boundaries, evidence closure.

## 33. First Vertical Slice
**Scenario:** `repo_explain` Task asking "Which module saves tasks and why can't UI access DB?"
Validates API, Kernel, Workspace Engine, Context Compiler, FakeProvider, verifier, Artifact Store, and restart resilience—without requiring real API keys or risky side effects.

## 34. Locked Architecture Decisions
- Task is central, not chat.
- Rust local daemon + SQLite.
- Capability Gateway for tool authority.
- Evidence-driven completion.
- Scoped memory with provenance.
- MCP as integration boundary, not internal bus.

## 35. Open Questions
VS Code vs. Desktop dashboard? Specific embedding engine? Which features require encrypted artifact stores? (Must be resolved via threat models, not just framework popularity).

## 36. Defensibility / Moat Criteria
Custos' uniqueness is not in new vocabulary, but coherent semantics: Durable tasks, typed continuations, evidence-backed closures, separated side-effect permits, and crash recovery as a product feature. If a feature does not increase coherence, control, continuity, evidence, or efficiency, it doesn't enter core.

## 37. New Member Checklist
Can you explain why Task > Chat? Difference between System 1 & 2? Grant vs. Permit? Artifact vs. Receipt vs. Evidence? How handoffs work? Crash recovery? If not, reread sections 6, 9, 10, 12, 14, 17, 18.

## 38. One-Page Summary
Custos is a local-first runtime orchestrating AI-assisted work. It sits above providers to ensure durability, tool control, and evidence-backed completion. Core runs locally (Rust/SQLite). Providers act through adapters. Work is governed by Domain Packs sharing a unified Kernel. Humans retain authority. The MVP proves the core loop for `repo_explain` and `bug_fix`.

## 39. Final Design Statement
Custos is a local-first, human-governed runtime that turns fragmented AI interactions into durable, transferable, capability-controlled, and evidence-backed work.
Custos does not build one autonomous super-agent. It provides the operating structure in which humans, specialized agents, judgment systems, tools, memory, and evidence can collaborate coherently.

**Custos — Guardian of Work.**
