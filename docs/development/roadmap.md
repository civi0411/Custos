# 16-Week Delivery Roadmap

> **Status:** Canonical Baseline v4.0  
> **Source:** Part XIV (§37-38) & Part VII (§56) Canonical Specification

Custos execution follows a strict **Vertical Slices** methodology: each milestone produces a fully testable, end-to-end runnable system rather than disconnected horizontal abstraction layers.

---

## 1. The Four Execution Phases

### Phase 0 — Weeks 1–2: Architecture Runway
- Initialize Cargo Monorepo and CI pipelines (`cargo clippy`, `cargo test`, `cargo deny`).
- Define core domain types (`TaskId`, `TaskContract`, `ActionProposal`, `ExecutionPermit`, `VerificationReceipt`).
- Construct Fake Provider and Fake Tool harnesses for deterministic testing.
- Author foundational ADRs (ADR-0001 through ADR-0010).
- Establish the baseline crash recovery test harness.

### Phase 1 — Weeks 3–5: Durable Local Kernel
- Implement SQLite schemas, migrations, and WAL mode.
- Build the Task & Step State Machine backed by event sourcing.
- Implement atomic transactions: Append Event + Projection Update + Outbox in a single transaction.
- Implement Content-Addressable Storage (CAS) for execution artifacts.
- Ship minimal CLI: `custos run`, `custos status`, `custos pause`, `custos resume`.
- Pass the automated Kill/Restart Crash Recovery Matrix.

### Phase 2 — Weeks 6–8: Read-Only Coding & Repo Intelligence
- Integrate Git commit snapshotting and workspace isolation.
- Integrate `ripgrep` and `tree-sitter` for sub-second symbol extraction and navigation.
- Implement the `ContextPack` compiler with relevance scoring and token budget checks.
- Build the first production `ProviderPort` adapter (OpenAI Codex or Claude).
- Deliver verified Code Explanation with snapshot-anchored citations.

### Phase 3 — Weeks 9–12: Controlled Mutation & Engineering Pack v1
- Implement the `Capability Gateway` and native OS sandboxing (macOS Seatbelt / Linux Bubblewrap).
- Enforce automated Git worktree isolation per Task/Run.
- Implement Exact-Payload Human Approval gates.
- Implement independent Verifier Runners for `cargo test`, `pytest`, and `ruff`.
- Export structured, signed `Verifiable Outcome Bundles`.
- Integrate the second Provider adapter with lossless switching via `ContinuationPacket`.

### Phase 4 — Weeks 13–16: Cognitive Control Fabric (System One Beta)
- Implement `JudgmentPort` and the Versioned Question Registry.
- Integrate deterministic Rules Engine and local SLM (ONNX / llama.cpp) backends.
- Integrate the TypeSafe Jev adapter in shadow/advisory mode.
- Standardize the 3 core Question Packs: Context-Triage, Risk-Classification, Completion-Challenge.
- Generate Calibration Reports quantifying token cost savings versus baseline execution.

---

## 2. 15-Point Acceptance Matrix

| # | Acceptance Criterion | Target Metric | Verification Method |
|---|---|---|---|
| 1 | Crash resilience (`kill -9`) | 100% deterministic resume | Automated Crash Test Harness |
| 2 | End-to-end bug fixing | Goal -> Verified Patch | Run across 20 SWE-bench benchmarks |
| 3 | Gateway enforcement | 0 unauthorized bypasses | Sandbox audit logs & seccomp traps |
| 4 | Approval revocation | Invalidate immediately on payload change | Test Approval Invalidation |
| 5 | Branch protection | 0 mutations on active working branch | Git commit hash comparison on HEAD |
| 6 | Code citations | 100% of citations bound to commit hash | Code Citation Verification |
| 7 | Budget limits | Strict ceiling adherence ($< +1\%$) | Token accounting unit tests |
| 8 | Network fault resilience | State preserved safely on provider 500/429 | Chaos test injection |
| 9 | Outcome Bundle validity | Schema-compliant diffs, receipts, costs | Bundle Schema Validator |
| 10 | Database disaster recovery | 100% successful backup and restore | SQLite Online Backup Verification |
| 11 | System One latency | $\ge 60\%$ reduction vs deep LLM reasoning | Benchmark vs LLM deliberation |
| 12 | API cost optimization | $\ge 50\%$ token savings on routing | Shadow Evaluation Report |
| 13 | CLI startup time | $< 30\text{ ms}$ | `hyperfine custos --version` |
| 14 | Idle memory footprint | $< 50\text{ MB}$ RSS | OS Process Monitoring |
| 15 | Zero secret leakage | 0 credentials in logs or DB | Secret Scanning CI Gate |
