# Agent Workspace & Desktop UI Architecture

> **Classification:** Desktop Frontend & Presentation Specification 
> **Source of Truth:** Authoritatively governed by [Custos Master Specification](../../Custos.md). 
> **Standards Reference:** [Desktop UI Architecture Standards](../../.agents/rules/desktop_ui_architecture_standards.md).

---

## 1. Product Identity & Design Principles

Custos provides a **Supervised Agent Development Environment (SADE)** with a local-first desktop workspace built with **React 19, TypeScript, Vite, Tailwind CSS, and Tauri v2**.

### Inviolable UI Invariants

1. **Strict Zero-Mock Policy:** Synthetic placeholders, dummy branches, or fabricated agent activities are prohibited in production components. Every displayed entity (Workspaces, Tasks, Sessions, Worktrees, Git Dirty Counts) reflects 100% genuine data queried from `DaemonClient` RPC (`v1.workspaces.*`, `v1.tasks.*`, `v1.sessions.*`).
2. **Worktree-Anchored Execution:** Every conversation session, terminal instance, diff viewer, and file editor is explicitly anchored to a physical worktree path and `workspace_id`.
3. **Decoupled Presentation & Authority:** Navigating between workbench screens or UI lenses is a read-only projection that never dispatches side-effects or grants execution permits.
4. **Single Source of Truth (SSOT):**
 - **`AppContext`**: Authoritative owner of global `workspaces`, `activeWorkspaceId`, `tasks`, and daemon connectivity status.
 - **`ScreenContext`**: Authoritative owner of worktree screens, multi-screen tab layouts, split ratios, and view zoom.

---

## 2. Distinct Architectural Concepts (Never Conflate)

| Concept | Meaning | Prohibited Misinterpretation |
|---|---|---|
| **Project Group** | High-level container of related repositories (e.g. `AgentHub`) | Not a single Git checkout |
| **Workspace / Repository** | Physical Git repository directory within the project | Not an abstract cloud workspace |
| **Worktree** | Concrete checkout directory where code runs and agents edit files | Not a detached virtual environment |
| **Task** | Goal, criteria, budget, revisions, and durable outcome state | Not an ephemeral chat tab |
| **Conversation Session** | Interaction turn sequence bound to a specific Task | Not execution authority |
| **Workbench Lens** | Specialized domain toolset (`assistant`, `coding`, `quant`, `research`) | Not a separate agent loop |
| **Run / WorkerRun** | Specific execution attempt in a task's graph | Not a new task |
| **Artifact** | Versioned output (patch, document, report, checkpoint) | Not an automated guarantee of task pass |

---

## 3. 3-Tier Navigation Hierarchy

The application chrome strictly enforces a 3-tier organizational model:

```

 Activity Rail (Home | Conversations | Tasks | Supervise)

 Tier 1: Project Group (AgentHub, 10 Repos) 
 
 Tier 2: Repository (Custos) 
 
 Tier 3: Worktree (dev [Primary Checkout]) 
 Tier 3: Worktree (feat/ui-overhaul) 
 
 Tier 2: Repository (AgentGateway) 
 Tier 3: Worktree (main [Primary Checkout]) 

```

1. **Tier 1 — Project Group:** Top-level project container opened via `⌘O`. Displays repository counts and global management controls.
2. **Tier 2 — Repository:** Physical Git repository with active branch tracking and worktree count badges.
3. **Tier 3 — Worktree:** The physical execution and checkout directory where agents run, compile, and edit files. Displays status chips (`primary`), dirty counts (`*12 files`), and live agent activity indicators.

---

## 4. Visual Design Tokens: Graphite JetBlack Aesthetic

The desktop UI adopts a dark luxury visual language:

- **Canvas Background:** `#0D0E11` / `#101113`
- **Sidebar & Card Surfaces:** `#141518` / `#18181b`
- **Translucent Overlays:** `bg-white/[0.04]`, hover: `bg-white/[0.08]`
- **Borders & Dividers:** `border-white/6` (structural), `border-white/10` (cards), `border-white/14` (active)
- **Typography:**
 - UI Chrome, Buttons, Body Prose: Inter (`font-sans`)
 - File Paths, Git Hashes, Terminal Text, Shortcuts: JetBrains Mono (`font-mono text-[11px]`)
- **Status Dots & Indicators:**
 - Emerald (`bg-emerald-400`): Clean, synchronized with Git index.
 - Amber (`bg-amber-400`): Dirty, uncommitted modified or untracked files.
 - Pulsing Cyan / White: Active AI agent task executing in worktree.
 - Amber Shield (`Shield text-amber-300`): Blocked, waiting for human approval.

---

## 5. Four Sovereign Workbench Lenses

The central workspace canvas renders 4 specialized lenses over shared resources:

1. **Assistant Workbench (`workbenches/assistant`):**
 - Personal productivity, email drafts, calendar synchronization, and contacts.
 - Outbox mediation view with two-phase human confirmation for external messages.
2. **Coding Workbench (`workbenches/coding`):**
 - Three-way file diffs, Git branch management, AST code navigators, and PTY terminal instances.
 - Isolated Git worktrees for candidate patch exploration.
3. **Research Workbench (`workbenches/research`):**
 - Literature exploration, interactive Jupyter notebook cells, claim synthesis, and passage anchoring.
 - 4-step ALCE verification pipeline view and experiment tracking.
4. **Quant Workbench (`workbenches/quant`):**
 - Financial market data charts (OHLCV), order ticket inspectors, backtesting tearsheets.
 - Statistical bias verifiers (`DeflatedSharpeVerifier`, `LookaheadBiasChecker`).

---

## 6. Component Directory Organization

All frontend components reside strictly within `apps/desktop/ui/src/`:

```text
apps/desktop/ui/src/
 api/ # Daemon RPC client, WebSocket streams, typed transports
 components/
 chat/ # Conversation pane, message cards, composer menu
 modals/ # Global overlays (settings, permissions, task creation)
 shell/ # App framing (ActivityRail, AppWorkspaceShell, TopBar)
 viewer/ # Dedicated viewers (Markdown, PDF, Diff, Companion)
 workbenches/ # Vertical lenses (assistant, coding, quant, research)
 context/ # AppContext, ScreenContext, ThemeContext
 lib/ # Shared utility functions, formatters, tokens
```

*File Ceiling Rule:* No single component file may exceed **600 lines**. Complex sub-views are decomposed into discrete child components within the module directory.

---

## 7. Mandatory Quality Gates

Every frontend modification must pass two automated quality gates prior to commit:

1. **TypeScript & Vite Compilation Gate:**
 ```bash
 npm --prefix apps/desktop/ui run build
 ```
 *Requirement:* Exit code 0 with zero TypeScript errors or bundling warnings.

2. **Canvas Smoke Test Suite Gate:**
 ```bash
 node tests/desktop/canvas-smoke.mjs
 ```
 *Requirement:* 100% of tests must pass (pass 43+, fail 0). Never break tab tree operations, split-pane mechanics, draft persistence, or workspace recovery.
