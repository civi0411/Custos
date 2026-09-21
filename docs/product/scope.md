# Product Scope & Definition of Done (Scope & MVP DoD)

> **Status:** Canonical Baseline v4.0  
> **Source:** Part I (§2) & Part VI (§38) Canonical Specification

---

## 1. Scope Matrix

| Domain | In-Scope | Out-of-Scope |
|---|---|---|
| **Operational Environment** | Local workstation (macOS, Linux) running as a local daemon (`custosd`) | Centralized cloud multi-tenant SaaS platform in initial phases. |
| **State Management** | Durable SQLite + WAL, Outbox pattern, crash recovery | Distributed in-memory storage requiring external Redis/Kafka clusters. |
| **AI Interactions** | Multi-provider interop (Codex, Claude, Local models) via adapters | Pre-training foundation models from scratch. |
| **Action Governance** | Capability Gateway, Worktree isolation, Exact-payload approvals | Bypassing security boundaries or running commands with root privileges. |
| **Client Interfaces** | Robust CLI and official VS Code Extension (*First-class clients*) | Mobile applications or complex web dashboards. |

---

## 2. Release Horizons

```text
+-------------------------------------------------------------+
| Horizon 1 (H1): Local Engineering Core                     |
| - Local daemon + SQLite + CLI                               |
| - Engineering Domain Pack v1 (Bug-fix & Feature slice)     |
| - Worktree isolation + Tree-sitter + Ripgrep                |
| - Single ProviderPort (Codex or Claude)                     |
| - Capability Gateway + Exact-payload approval               |
+------------------------------+------------------------------+
                               |
+------------------------------v------------------------------+
| Horizon 2 (H2): Local-First Multi-Domain & Portability      |
| - Pluggable System One (Deterministic + Local SLM + Jev)    |
| - Multi-provider switching (Codex <-> Claude <-> Local)     |
| - Official VS Code Extension                                |
| - Research Domain Pack (Claim-Evidence matrix, Obsidian)    |
| - Personal Operations Domain Pack (Attention budget)        |
+------------------------------+------------------------------+
                               |
+------------------------------v------------------------------+
| Horizon 3 (H3): Workstation Mesh & Team Federation          |
| - Peer-to-peer workspace synchronization                    |
| - Selective cross-machine artifact sharing                  |
| - Remote federation via A2A protocol                        |
+-------------------------------------------------------------+
```

---

## 3. MVP Definition of Done (MVP DoD)

Before releasing an MVP build for operational use, the system must satisfy 100% of the following criteria:

### Core MVP DoD (Mandatory for Alpha)
- [ ] **Crash Resilience:** Tasks survive daemon termination (`kill -9`) in any resumable state and resume deterministically upon restart.
- [ ] **End-to-End Task Lifecycle:** A bug-fix workflow executes from goal definition to an automatically verified patch with test pass receipts.
- [ ] **Complete Action Governance:** Zero file mutations, shell commands, or network connections occur outside the Capability Gateway.
- [ ] **Exact-Payload Approval:** Approval binds cryptographically to the payload; altering payload content invalidates prior permits immediately.
- [ ] **Worktree Isolation:** All source mutations occur in isolated Git worktrees, never mutating the user's active branch directly.
- [ ] **Snapshot Provenance:** Source extraction binds to specific commit snapshots; stale snapshots trigger rejection and re-synchronization.
- [ ] **Compliant ContextPacks:** Context sent to models includes provenance labels, sensitivity controls, and strictly respects token budgets.
- [ ] **Provider Fault Independence:** Provider rate limits or 500 errors never corrupt or lose Task state.
- [ ] **Comprehensive OutcomeBundles:** Output bundles contain complete file diffs, test receipts, audit evidence, total costs, and residual risks.
- [ ] **Disaster Recovery Validation:** SQLite backup, restore, and schema migration scenarios are automatically tested in CI.
- [ ] **Transparent Documentation:** Threat models and operational runbooks are fully documented.
- [ ] **Engineering Honesty:** No marketing or capability claims exceed measured benchmark evidence.

### Cognitive Beta DoD (Supplementary for Cognitive Evaluation)
- [ ] Standard Question Packs run in shadow/advisory mode.
- [ ] Model versions and Question Pack definitions are pinned and fully logged.
- [ ] Separate calibration and thresholds for judgment types (Boolean, Choice, Score).
- [ ] Out-of-distribution (OOD) cases and judgment disagreements escalate to human principals promptly.
- [ ] Data transmitted to external judgment services is strictly redacted.
- [ ] No logical code path allows System One to grant an `ExecutionPermit` autonomously.
- [ ] Cost and task success metrics are transparently benchmarked against non-System-One baselines.
