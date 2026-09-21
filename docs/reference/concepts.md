# Core Concepts Glossary

> **Status:** Canonical Baseline v4.0  
> **Scope:** Entire Custos System

This document provides definitive, consistent specifications for foundational concepts used across Custos source code, specifications, protocols, and APIs.

---

## 1. Core Entities

### Task
The central unit of work in Custos. A Task maintains durable state, is defined by a binding Task Contract, executes across multiple steps via specialized workers, and concludes with verified artifacts and empirical evidence.

### Task Contract
A binding specification between the user and the runtime defining work parameters, including:
- **Intent:** The core objective to be achieved.
- **Scope:** The explicit boundary of files, directories, or external resources allowed to be mutated.
- **Constraints:** Resource limitations (token budget, maximum duration, autonomy tier).
- **Verification Criteria:** Objective acceptance criteria that must be satisfied.

### Run
A concrete execution session of a Task. A Task can span multiple Runs (e.g., Run 1 fails verification; the user refines requirements, triggering Run 2).

### Step
A discrete execution step within a Run, associated with a single reasoning action or tool execution, including pre/post state captures and execution receipts.

### Artifact
Output products generated or mutated during Task execution (source code, documentation files, analysis reports, test receipts). Large artifacts are stored in the Content-Addressable Storage (CAS) system.

---

## 2. Cognitive Fabric and Judgment

### System One (Judgment Fabric)
Fast, local, low-latency (millisecond-scale) judgment infrastructure. Functions as reflex checks: intent classification, ambiguity detection, risk tiering, invariant enforcement, and human escalation gating.

### System Two (Deliberation Fabric)
Deep, computationally intensive reasoning infrastructure executed by frontier Large Language Models (OpenAI Codex, Claude, DeepSeek, etc.). Responsible for complex multi-step planning, code generation, and knowledge synthesis.

### RDC Protocol (Request-Decision-Challenge)
The standardized exchange protocol across cognitive tiers:
- **Request:** Context-bearing evaluation or action request.
- **Decision:** Structured evaluation output including confidence score and rationale.
- **Challenge:** Reflexive counter-evaluation triggered when contradictions or latent risks are detected.

### Question Registry
A versioned repository of standardized evaluation questions used to query System One consistently for structured decisions instead of ad-hoc prompt formatting.

---

## 3. Execution and Security

### Capability Gateway
The single, authoritative gateway controlling all external side-effect operations. No worker or agent process is permitted to execute system commands or external APIs without passing through this gateway.

### ExecutionPermit
A cryptographically signed token issued by the Kernel confirming that an action has been validated against permissions, satisfies security policies, and has received required Human Exact-Payload Approval. Features strict TTL and explicit resource scopes.

### Exact-Payload Approval
Transparent authorization principle: humans never issue blanket approvals (e.g., "allow arbitrary bash commands"). Users are presented with exact command arguments, file diffs, or API payloads prior to signing off.

### Worktree Isolation
Environment isolation mechanism provisioning an independent `git worktree` per Task/Run. Workers operate strictly inside this isolated worktree; the repository `main` branch is updated only after all verification gates succeed.

---

## 4. Evidence and Verification

### Evidence-Carrying Action (ECA)
Every mutating system action must generate verifiable evidence demonstrating correctness and audit justification for why it was permitted.

### Verifiable Outcome Bundle
The canonical deliverable package produced at task completion, comprising:
- All generated or modified artifacts.
- Complete execution trace log.
- Collected verification receipts (test results, linter receipts, build logs).
- Cryptographic signatures and content hashes.

### Completion Gate
An automated verification gate evaluated prior to marking a Task as completed: validates delivered outcomes against acceptance criteria in the Task Contract. Missing or invalid evidence prevents state transition to `Completed`.

---

## 5. Data and Communication

### ContextPack
An optimized, compiled context payload prepared before dispatching to an AI model, ranked via relevance scoring and tagged with explicit data provenance.

### ContinuationPacket
A provider-agnostic state payload packaging runtime state, environment variables, and contracts enabling seamless provider switching or crash recovery.

### Star Topology
Strict star-shaped communication architecture: all coordination between workers routes through the central Kernel coordinator; direct unmonitored peer-to-peer agent chat is prohibited.

### Human Attention Budget
A quantitative limit on user interruptions. The runtime optimizes execution to minimize interruptions, reserving escalation exclusively for high-risk decisions.
