# Capability Map & Feature Taxonomy

> **Status:** Canonical Baseline v4.0  
> **Source:** Part I (§3) & Part VI (§31) Canonical Specification

---

## 1. Six-Domain Capability Map

The Custos architecture encompasses 6 core capability domains:

```text
+-----------------------------------------------------------------------------+
|                             CUSTOS CAPABILITY MAP                           |
+----------------------+----------------------+-------------------------------+
| 1. TASK GOVERNANCE   | 2. COGNITIVE CONTROL | 3. EXECUTION & SAFETY         |
| - Contract definition| - Pluggable System 1 | - Capability Gateway          |
| - State transitions  | - System 2 adapters  | - Tiered sandboxing           |
| - Budget enforcement | - RDC protocol       | - Exact-payload approvals     |
| - Resumable runs     | - Question Registry  | - Git worktree isolation      |
+----------------------+----------------------+-------------------------------+
| 4. CONTEXT & MEMORY  | 5. INTEROPERABILITY  | 6. EVIDENCE & AUDIT           |
| - ContextPack scoring| - ProviderPort       | - 6 Evidence classes          |
| - Provenance tracking| - ContinuationPacket | - Verifiable Outcome Bundle   |
| - 5 Memory layers    | - Safe model switch  | - Decision Ledger             |
| - Relational graph   | - Official MCP client| - Tamper-evident receipts     |
+----------------------+----------------------+-------------------------------+
```

### 1.1 Task Governance
- Task Contract validation before task execution.
- Durable state machine governing full task lifecycles: Draft -> Ready -> Running -> Suspended -> Completed / Failed.
- Budget constraints: token cost caps and step limits enforced by the kernel.
- Deterministic resumption following unexpected interruptions or daemon restarts.

### 1.2 Cognitive Control
- Clear cognitive bifurcation: System One (fast reflection) and System Two (deep deliberation).
- Standardized RDC (Request-Decision-Challenge) protocol.
- Versioned Question Registry management.
- Reflexive challenges to identify flawed assumptions before executing costly operations.

### 1.3 Execution & Safety
- Unified Capability Gateway arbitrating all OS and network interactions.
- Fine-grained capability grants via `ExecutionPermit` with cryptographically bound hashes and expiration.
- Complete mutation isolation inside dedicated Git worktrees.
- Sandboxed command execution via macOS Seatbelt and Linux Bubblewrap.

### 1.4 Context & Memory
- ContextPack relevance scoring and token allocation.
- Provenance tagging and sensitivity tier labeling for all ingested data.
- 5-tier memory model: Working, Task Episodic, Workspace Semantic, Procedural, and Human Preference.
- Memory promotion and eviction pipeline governed by human feedback.

### 1.5 Interoperability
- `ProviderPort` abstraction decoupling the kernel from provider-specific APIs.
- `ContinuationPacket` format enabling clean task hand-offs across models at designated checkpoints.
- Model Context Protocol (MCP) client integration at outer boundaries.

### 1.6 Evidence & Audit
- Systematic ingestion of 6 objective evidence tiers from the environment.
- Verifiable Outcome Bundles containing diffs, execution logs, and signed receipts.
- Append-only Decision Ledger recording all technical choices with rationale.

---

## 2. Feature Taxonomy & Priorities

### 2.1 Must — Alpha
1. Local workspace and repository registration.
2. Task Contract initialization and validation.
3. Durable Task/Step state persistence in SQLite.
4. CLI binary for full task lifecycle management.
5. Snapshot indexing and fast codebase search (Git + Ripgrep).
6. Basic ContextPack compiler with token budget enforcement.
7. First ProviderPort adapter (Codex or Claude).
8. Git worktree mutation isolation.
9. Capability Gateway governing patch applications and basic shell commands.
10. Exact-payload approval gates for high-risk actions.
11. Verifier executing automated test commands.
12. Crash recovery and deterministic pause/resume loops.
13. Verifiable Outcome Bundle export.
14. Local token usage and cost accounting.

### 2.2 Must — Cognitive Beta
1. Formal DecisionCase / RDC schema implementation.
2. Versioned Question Registry.
3. JudgmentPort adapter connecting System One backends.
4. Shadow/Advisory mode to measure decision calibration.
5. Context-Triage Question Pack.
6. Task-Intake Question Pack.
7. Patch-Challenge Question Pack.
8. Confidence calibration reports.
9. Structured decision logging to Decision Ledger.
10. Sensitive data redaction prior to external judgment evaluation.

### 2.3 Should — Portability & DX
1. Second ProviderPort adapter (seamless dual-provider switching).
2. Automated capability probes for model evaluation.
3. ContinuationPacket serialization and transfer.
4. Safe switching checkpoints during active execution.
5. VS Code extension client for Task inspection and artifact review.
6. Provider event record-and-replay suite for regression testing.
7. Provider-specific prompt and syntax compilation.

### 2.4 Later — Research Domain Pack
1. Document ingestion and parsing (PDF, HTML).
2. Claim-Evidence relational graph model.
3. Citation graph and cross-document link mapping.
4. Automated source verification and retraction detection.
5. Synthesis report export in Markdown and Obsidian Vault formats.
6. Zotero reference library connector.

### 2.5 Later — Personal Operations Pack
1. Secure connectors for Email, Calendar, and Tasks.
2. Recipient identity resolution and safety bounds.
3. 3-stage execution pipeline: Draft -> Review Preview -> Commit.
4. Personalized attention policies per user.
5. Notification batching to protect human focus.
6. Cross-domain daily summary reports.

### 2.6 Explicitly Deferred
- Centralized multi-tenant cloud team servers.
- Public agent/plugin marketplaces.
- Decentralized Agent-to-Agent (A2A) network federation.
- Automated unconstrained deployment to production environments.
- Standalone external graph database clusters.
- Native mobile client applications.
