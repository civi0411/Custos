# Custos Desktop UI

> The visual desktop frontend for the **Supervised Agent Development Environment (SADE)**, powered by **Tauri 2**, **Vite**, **React 19**, and **TypeScript**.

---

## 1. Architectural Philosophy & Standards

The Desktop UI strictly enforces the standards defined in [`.agents/rules/desktop_ui_architecture_standards.md`](../../../.agents/rules/desktop_ui_architecture_standards.md):

1. **Zero-Mock Policy:** The UI never renders synthetic, hallucinated, or placeholder state. All metrics, logs, artifacts, and diffs reflect authentic telemetry from the local Custos daemon (`apps/daemon`).
2. **Fail-Closed States:** When backend capabilities or transports are degraded, the interface transitions explicitly to `Degraded` or `Unavailable`, displaying human-auditable reasons rather than empty lists `[]` or silent fallbacks.
3. **3-Tier Spatial Hierarchy:**
   - **Tier 1 (Project):** Logical project scope, configuration, and global provider catalog.
   - **Tier 2 (Workspace):** Concrete environment anchoring, lens binding, and session orchestration.
   - **Tier 3 (Worktree):** Physical Git worktree isolation, active dirty manifests, and execution diffs.
4. **Graphite JetBlack Token System:** High-contrast, dark-mode design token architecture optimized for prolonged cognitive focus and reduced eye strain.

---

## 2. Four Specialized Workbenches

The interface provides dedicated cognitive environments for each of the four vertical domains:

- **Coding Workbench (`components/workbenches/coding`):**
  - Three-path workspaces (canonical, staging, sandbox).
  - Visual structured diffs, syntax inspection, and atomic patch bundles.
  - Worktree lease management and dirty state tracking.
- **Research Workbench (`components/workbenches/research`):**
  - Literature corpus explorer with cryptographic passage anchors.
  - 4-tier epistemic claim matrix (`L0` to `L3`) and ALCE citation coverage verifier.
  - Synthesis Studio and interactive Python notebook kernel viewer.
- **Assistant Workbench (`components/workbenches/assistant`):**
  - Outbox approval queue for human-in-the-loop mutation governance.
  - Sandboxed browser automation and terminal PTY sessions.
  - Standing delegation grants and persistent knowledge notes.
- **Quant Workbench (`components/workbenches/quant`):**
  - Strategy backtest simulation with air-gapped network indicators (`ENV-05`).
  - Empirical performance tearsheets (Sharpe, Sortino, drawdowns).
  - Historical market data partition manager.

---

## 3. Development & Verification Commands

```bash
# Install dependencies
pnpm install

# Run Vite development server
pnpm dev

# Build production bundle and run TypeScript typecheck (Mandatory Quality Gate 2/5)
npm run build

# Run headless canvas smoke invariants test suite (Mandatory Quality Gate 3/5)
node --test ../../tests/desktop/canvas-smoke.mjs ../../tests/desktop/model-catalog-smoke.mjs ../../xtask/scripts/checks-smoke.mjs
```
