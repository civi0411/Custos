# Three-Vertical Acceptance, Desktop & Cross-Pack Acceptance (F09)

> SADE Master Specification — Phase 8 / F09 Architectural Blueprint
> Governs: `custos-domain`, `custos-core`, `custos-adapters`, `custos-daemon`, `custos-persistence`, and Golden Acceptance Tests `G08`, `G09`, `G10`.

---

## 1. Context & Motivation

Local-first SADE (Sovereign Autonomous Development Environment) unifies three primary lenses into a single developer workbench:
1. **Coding Vertical:** Autonomous repository bugfixing, patch preview simulation, branch drift tracking, dirty worktree protection, and containment against malicious path traversal and agent-edited test tampering.
2. **Research Vertical:** Notebook document models, isolated Python kernel bindings, passage anchor citations, evidence synthesis, and immutable claim lineage.
3. **Assistant / Copilot Vertical:** Human-in-the-loop draft authoring, cryptographic payload locking, fail-closed outbox dispatching, crash recovery reconciliation to Uncertain, and double-dispatch fencing.

Phase 8 (F09) delivers the end-to-end acceptance suite proving that all three verticals operate seamlessly across the unified conversational journal without cross-tenant bleed, permission transfer, duplicate execution, or destruction of uncommitted user work.

---

## 2. Core Architecture & Verification Matrix

```
┌────────────────────────────────────────────────────────────────────────┐
│                        UNIFIED CONVERSATIONAL JOURNAL                  │
│                                                                        │
│  [ Assistant Lens ]       [ Coding Lens ]        [ Research Lens ]     │
│  - Drafts & Outbox        - Worktrees & Git      - Notebooks & Cells   │
│  - MCP / Webhook Calls    - Sandbox & Diff       - Synthesis & Claims  │
└────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                      SOVEREIGN GOVERNANCE & GATES                      │
│                                                                        │
│  Gate 1: Parameter & Path Traversal Containment (sandbox / chroot)     │
│  Gate 2: Capability Grants, Permits & Payload Lock Verification        │
│  Gate 3: Outbox Transactional CAS Fencing & Crash Reconciliation       │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Invariant Specification by Vertical

### 3.1 Three-Lens Continuity & Cross-Session Isolation (G08)
- **Unified Journal without Hallucination:** A single chat session viewed or continued across Assistant, Coding, and Research lenses retains a strictly unified journal. The system never fabricates phantom tasks, background runs, or synthetic tool calls when switching lenses.
- **Lens Switch Idempotency:** Switching between lenses does not transfer authority, capabilities, or execution permits across distinct contexts.
- **Draft Scoping & Isolation:** Assistant communication drafts (`AssistantDraft`) are strictly scoped to their originating `session_id` and `task_id`. Cross-session or cross-task modification attempts are rejected with `DomainError::Validation("Draft scope cannot be changed")`.
- **Draft Payload Lock:** Upon user review and approval, a cryptographic hash (`blake3`/`digest`) is sealed in `payload_lock`. Subsequent mutation attempts to locked drafts fail closed (`"Reviewed draft is locked; create a new draft to revise it"`).
- **Workspace Isolation:** Project and workspace switching guarantees context isolation, preventing cross-tenant bleed.

### 3.2 Coding Vertical: Dirty Repo, Base Drift & Containment (G09)
- **Normal Path Bugfix & Diff Parity:** Bugfixes in real Git repositories execute through `SovereignDeveloperAdapter`. Patch preview (`patch_preview`) simulates diffs with zero disk mutations. When write is authorized, applied changes match the git diff and `LocalWorkspaceProvider::inspect_dirty` accurately reports modified files.
- **User Bytes Preservation Invariant:** Uncommitted dirty user files (e.g. `src/user_scratch.rs`) and untracked notes (`user_todo.txt`) are strictly preserved during agent bugfix runs. The agent never deletes, overwrites, or reverts uncommitted user changes.
- **Base Drift Tracking:** Workspaces record the baseline commit hash (`base_commit_hash`) to accurately detect repository divergence.
- **Agent-Edited Tests Containment:** When an agent modifies test code to conceal a regression or bypass test failures, `EvidencePipeline` with trusted verifiers checks evidence bundle digests, preventing tampered test results from bypassing trusted acceptance.
- **Directory Traversal Denial:** Parameter validation and sandbox drivers reject `..` traversal paths (e.g. `../../outside.txt`), returning `ReceiptStatus::Failure` with zero mutations outside the workspace.

### 3.3 Assistant Vertical: Outbox, Timeout Reconciliation & CAS Fencing (G10)
- **Fail-Closed Outbox Lifecycle:**
  $$\text{Draft} \xrightarrow{\text{Approve}} \text{Locked} \xrightarrow{\text{Enqueue}} \text{Pending} \xrightarrow{\text{Approve}} \text{Approved} \xrightarrow{\text{CAS}} \text{Dispatching} \xrightarrow{\text{Execute}} \text{Receipted}$$
- **Timeout & Uncertain Handling (Gate 3):** Dropped connections or outbound call timeouts transition outbox entries immediately to `Uncertain`.
- **Startup Crash Reconciliation:** On daemon restart, `OutboxRepository::reconcile_on_startup()` recovers all in-flight effect attempts and dispatching outbox entries to `Uncertain`.
- **Zero Blind Retries:** `ActionLifecycleState::Uncertain.allows_blind_retry() == false`. Uncertain entries cannot be blindly retried or re-dispatched, preventing duplicate outbound messages and quota exhaustion.
- **Tampered Payload Rejection:** Outbox entries verify that `sha256(payload_json) == argument_digest`. Any in-place tampering blocks approval and dispatch.
- **Double-Dispatch CAS Fencing:** Transitioning from `Approved` to `Dispatching` uses an atomic Compare-And-Swap query (`UPDATE outbox_entries SET status = 'dispatching' WHERE id = ?1 AND status = 'approved'`). Concurrent dispatch workers conflict safely, ensuring that exactly one worker executes the outbound effect.

---

## 4. Test Suite Summary

| Golden Test | Target File | Test Scenarios | Status |
|:---|:---|:---|:---:|
| **G08** | `tests/e2e/tests/three_lenses_golden_test.rs` | Unified Journal, Lens Switch Idempotency, Draft Scoping, Payload Lock, Workspace Isolation | **5 / 5 PASS** |
| **G09** | `tests/e2e/tests/coding_vertical_golden_test.rs` | Bugfix Diff Parity, User Bytes Preserved, Base Drift, Agent-Edited Tests Fencing, Path Traversal Denial | **4 / 4 PASS** |
| **G10** | `tests/e2e/tests/assistant_vertical_golden_test.rs` | Normal Path Draft/Outbox/Receipt, Timeout & Crash Reconciliation, Tamper Defense, Double-Dispatch CAS Fencing | **4 / 4 PASS** |

---

## 5. Verification Protocol

Run the entire Phase 8 Golden Acceptance Suite:
```bash
cargo test -p custos-tests-e2e --test three_lenses_golden_test --test coding_vertical_golden_test --test assistant_vertical_golden_test
```
