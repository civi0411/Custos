# Desktop Panel and Control Source Index

> **Status:** Static Architectural Component Index  
> **Source Analysis:** Generated from static source declarations across desktop reference components (Orca and Open Science).  
> **Total Audited Scope:** 1,190 component declarations, covering 4,448 interactive surfaces and controls.

---

## 1. Executive Summary of Audited Component Surfaces

| Upstream Source | Primary Architecture | Audited Component Files | Total Declarations | Primary Functional Domain |
|---|---|:---:|:---:|---|
| **Orca Reference** (`src/renderer/src/`) | Electron / React / TypeScript | 1,108 files | 3,899 | Window shell, code editors, git worktrees, terminal PTYs, browser CDP, mobile emulator canvases, and provider cards. |
| **Open Science Reference** (`apps/desktop/src/`) | Tauri / React / TypeScript | 82 files | 549 | Scientific paper readers, citation cards, notebook canvases, execution run timelines, and figure annotators. |

---

## 2. Functional Subsystem Breakdown & Custos Architecture Mapping

### 2.1 App Shell, Window Chrome & Navigation

Orca defines 4 shell controllers with 13 top-level window declarations; Open Science defines 9 app container wrappers with 138 declarations.

| Subsystem Component | Declarations | Representative Controls | Custos Target Architecture Equivalent |
|---|:---:|---|---|
| `AppWorkspaceShell.tsx` | 2 | Primary layout tree, sidebar toggle, pane divider | `apps/desktop/ui/src/components/layout/` |
| `TitlebarLeftControls.tsx` | 5 | Application menu, app branding, sidebar toggle | `apps/desktop/ui/src/components/layout/TitleBar.tsx` |
| `TitlebarMainStrip.tsx` | 3 | Toggle right inspector, collapse pane | `apps/desktop/ui/src/components/layout/Header.tsx` |
| `WindowControls.tsx` | 3 | Minimize, Maximize, Close | Native window chrome via Tauri / Electron |
| `SidebarContainer.tsx` | 24 | Expand/collapse sidebar, project switcher, worktree tree | `apps/desktop/ui/src/components/workspaces/WorkspaceSidebar.tsx` |

### 2.2 Chat, Conversational Supervision & Decision Cards

142 component files provide conversational interaction, model routing, and approval cards.

| Subsystem Component | Declarations | Representative Controls | Custos Target Architecture Equivalent |
|---|:---:|---|---|
| `ChatHistory.tsx` | 18 | Message turn scroll, timestamp markers, actor badges | `apps/desktop/ui/src/components/conversation/` |
| `MessageComposer.tsx` | 14 | Model picker, token budget indicator, send/abort button | `apps/desktop/ui/src/components/conversation/Composer.tsx` |
| `DecisionApprovalCard.tsx` | 9 | Cryptographic permit approval, payload diff preview, reject button | `apps/desktop/ui/src/components/governance/OutboxFence.tsx` |
| `NativeChatResumeModal.tsx` | 6 | Reconnect session, dismiss orphan runs, retry attempt | `apps/desktop/ui/src/components/recovery/ReconcileModal.tsx` |
| `TokenUsageGauge.tsx` | 5 | Context window utilization, cost meter ($USD), token counters | `apps/desktop/ui/src/components/telemetry/TokenGauge.tsx` |

### 2.3 Worktrees, Git Lifecycle & Code Diff Canvas

215 component files handle repository checkouts, branch isolation, and diff reviews.

| Subsystem Component | Declarations | Representative Controls | Custos Target Architecture Equivalent |
|---|:---:|---|---|
| `GitBranchSwitcher.tsx` | 12 | Branch list, create worktree, fetch remote, checkout | `apps/desktop/ui/src/components/workbenches/coding/` |
| `WorktreeManager.tsx` | 16 | Add worktree, delete lease, dirty tree warning | `apps/desktop/ui/src/components/workspaces/WorktreeList.tsx` |
| `DiffViewer.tsx` | 28 | Side-by-side / inline diff, stage hunk, discard change | `apps/desktop/ui/src/components/editor/DiffEditor.tsx` |
| `FileTreeNavigator.tsx` | 22 | File/directory tree, new file, delete, rename | `apps/desktop/ui/src/components/files/FileTree.tsx` |
| `MergeConflictResolver.tsx` | 14 | Accept current, accept incoming, compare both | `apps/desktop/ui/src/components/editor/ConflictResolver.tsx` |

### 2.4 Terminal PTYs, Process Streams & Search

84 component files provide terminal multiplexing and command tracking.

| Subsystem Component | Declarations | Representative Controls | Custos Target Architecture Equivalent |
|---|:---:|---|---|
| `TerminalCanvas.tsx` | 16 | Xterm.js mount, PTY stream listener, resize observer | `apps/desktop/ui/src/components/terminal/TerminalPane.tsx` |
| `TerminalSearch.tsx` | 7 | Case sensitive, Regex search, match next/previous | `apps/desktop/ui/src/components/terminal/TerminalSearch.tsx` |
| `ProcessSupervisor.tsx` | 11 | Interrupt process (SIGINT), kill process, restart job | `apps/desktop/ui/src/components/terminal/ProcessControls.tsx` |
| `TerminalWorkspaceDialog.tsx` | 5 | Save session buffer, clear scrollback, copy selection | `apps/desktop/ui/src/components/terminal/TerminalDialogs.tsx` |

### 2.5 Browser Automation & Mobile Simulator

48 component files provide visual inspection canvases for coding agents.

| Subsystem Component | Declarations | Representative Controls | Custos Target Architecture Equivalent |
|---|:---:|---|---|
| `BrowserViewPane.tsx` | 15 | URL address bar, back/forward, reload, devtools toggle | `apps/desktop/ui/src/components/browser/BrowserPane.tsx` |
| `AccessibilityTreeWalk.tsx` | 9 | Inspect AX nodes, view selector paths, copy node ref | `apps/desktop/ui/src/components/browser/AXTreeInspector.tsx` |
| `MobileSimulatorCanvas.tsx` | 12 | Device frame (iPhone, Pixel, iPad), rotate, touch mode | `apps/desktop/ui/src/components/simulator/SimulatorPane.tsx` |
| `ScreenshotCompare.tsx` | 8 | Visual regression overlay, baseline vs. candidate slider | `apps/desktop/ui/src/components/simulator/VisualDiff.tsx` |

### 2.6 Scientific Inquiry, Notebooks & Citation Grounding (Open Science)

73 component files provide specialized research workbench capabilities.

| Subsystem Component | Declarations | Representative Controls | Custos Target Architecture Equivalent |
|---|:---:|---|---|
| `PaperReaderPane.tsx` | 24 | PDF page navigation, zoom, passage text selection | `apps/desktop/ui/src/components/workbenches/research/` |
| `ClaimInspector.tsx` | 16 | Grounding level badge, passage anchor link, verifier status | `apps/desktop/ui/src/components/workbenches/research/` |
| `NotebookCellEditor.tsx` | 32 | Execute cell, interrupt kernel, restart kernel, add cell | `apps/desktop/ui/src/components/workbenches/research/` |
| `FigureAnnotator.tsx` | 14 | Normalized bounding box pin, comment thread, export figure | `apps/desktop/ui/src/components/workbenches/research/` |
| `ProvenanceInspector.tsx` | 18 | Producing task hash, input dataset digest, execution receipt | `apps/desktop/ui/src/components/workbenches/research/` |

---

## 3. Architectural Invariants for Custos Component Realization

1. **Zero-Mock Rendering:** No UI component may render static placeholder data or mock success states. Unconnected or in-flight features must display honest `Unconnected`, `Admitting`, or `Degraded` states.
2. **Graphite JetBlack Tokens:** All realized components strictly consume design tokens defined in `packages/ui-tokens` and `.agents/rules/desktop_ui_architecture_standards.md`.
3. **Decoupled Presentation Identifiers:** Presentation tabs (`CanvasGroup` / `TabId`) are strictly separate from physical domain identities (`TaskId`, `SessionId`, `WorktreeId`). Closing a presentation tab never terminates background jobs or unmounts worktrees.
