# Desktop UI & Frontend Architecture Standards
## Custos Desktop Application Protocol (React 19, TypeScript, Vite, Tailwind CSS, Tauri v2)

> **Scope:** All AI Coding Assistants (Cursor, Claude Code, Antigravity, Codex, Copilot, Windsurf) developing, modifying, or refactoring code within `Custos/ui/desktop`.
> **Inviolable Principle:** Any change to the desktop UI must strictly adhere to this specification before touching source code.

---

### 1. Data Integrity & Zero-Mock Policy

1. **Zero Mock Data in Production Components:**
   - Never hardcode synthetic branches, dummy tasks, fake status lines, or simulated agent activities (e.g. no `"Custos Mobile"`, `"gpt-5.6-sol"`, or placeholder prompt outputs).
   - Every single displayed item (Workspaces, Repositories, Worktrees, Git Status, Dirty Counts, Tasks, Sessions) must reflect 100% real data queried from `daemonClient` RPC (`v1.workspaces.*`, `v1.tasks.*`, `v1.sessions.*`, `v1.workflow.*`).
   - When data is not yet present, always render an honest, clean, and actionable **Empty State** (e.g. *"No projects open" accompanied by an "Open Project Folder" button*).

2. **Single Source of Truth (SSOT):**
   - **`AppContext`**: Authoritative owner of global `workspaces`, `activeWorkspaceId`, `tasks`, `currentSessions`, `activeDetection`, and daemon connectivity status.
   - **`ScreenContext`**: Authoritative owner of worktree-anchored working screens (`worktree screens`), multi-screen tab layout, split ratios, and view zoom.
   - Never introduce unmanaged local `useState` copies of workspaces or worktrees that cause state drift.

3. **Worktree-Anchored Execution Invariant:**
   - Every conversation session, agent task, terminal instance, file viewer, and diff inspector **must be explicitly anchored to a physical worktree path and `workspace_id`**.
   - No commands or file edits may run in an abstract floating environment without a concrete checkout directory.
   - Switching worktrees must atomically update `activeWorkspaceId` and invoke `revealWorktreeScreen`.

---

### 2. 3-Tier Hierarchy Model

Every directory exploration and navigation flow across the desktop app must strictly implement this 3-tier hierarchy:

1. **Tier 1 — Project Group:**
   - Represents the high-level project container (e.g. `AgentHub` multi-repo ecosystem, or a standalone project opened via `⌘O`).
   - Icon: `GitFork` or `Boxes`.
   - Elements: Project title (bold), repository count badge (`10 repos`), expand/collapse chevron, and quick `+` button to add repositories.

2. **Tier 2 — Workspace / Repository:**
   - Represents a physical Git repository within the project (`Custos`, `opencode`, `agentgateway`, etc.).
   - Visual hierarchy: Indented with subtle tree branch guide (`border-l border-white/8`).
   - Elements: `FolderGit2` icon (or GitHub repo avatar), repository name (medium weight), worktree count pill (`(2)`), and hover `+` button to spawn a new isolated worktree.

3. **Tier 3 — Worktree (Physical Execution & Checkout Unit):**
   - Represents the concrete checkout directory where code runs, agents edit files, and terminals execute.
   - **Classification**:
     - Main checkout must display the compact `primary` chip (`text-[9px] font-mono border border-white/12 bg-white/[0.04] px-1`).
     - Task/secondary worktrees display the `GitBranch` icon.
   - **Active State Card**: Highlighted container (`rounded-xl bg-white/[0.08] border border-white/14 shadow-sm`) with a crisp white `Check` icon.
   - **Standard Status Indicators**:
     - 🟢 **Emerald (`bg-emerald-400`)**: Clean, ready, fully in-sync with Git index.
     - 🟡 **Amber (`bg-amber-400`)**: Dirty, contains uncommitted modified/untracked files (`*12 files`).
     - ⚪ **Pulsing White / Cyan**: Active AI agent or task executing in this worktree.
     - 🛡️ **Amber Shield (`Shield text-amber-300`)**: Blocked, waiting for human diff approval or permission.
   - **Metadata Sub-line**: Displays the relative directory path in font-mono (`Custos/`), followed by live agent activity if an active session exists (`✓ [Task Title] · [Model]`).

---

### 3. Visual Aesthetics (Graphite on Paper / JetBlack Luxury)

1. **Color Tokens**:
   - Deep Canvas: `#0D0E11` or `#101113`.
   - Sidebar & Cards: `#141518` or `#18181b`.
   - Translucent Surfaces: `bg-white/[0.04]`, hover `bg-white/[0.08]`.
   - Borders:
     - Structural Dividers: `border-white/6`.
     - Normal Card Borders: `border-white/10`.
     - Focused / Active Card Borders: `border-white/14` to `border-white/20`.
   - Typography Contrast:
     - Primary Titles / Active Items: `text-white` or `text-white/90`.
     - Secondary Labels: `text-white/60`.
     - Annotations / Keyboard Shortcuts / Paths: `text-white/35` to `text-white/45`.

2. **Typography Standards**:
   - UI chrome, headers, buttons, descriptions: Sans-serif (`Inter`, `font-sans`).
   - Git branch names, file paths, commit hashes, dirty file counters, terminal text, shortcut badges: Monospace (`JetBrains Mono`, `font-mono text-[11px]`).

3. **Zero Layout Jitter**:
   - All icons, status dots, and badges must specify `shrink-0`.
   - Long labels must specify `truncate min-w-0 flex-1`.
   - All hover, focus, and active transitions must be swift and smooth (`transition duration-150 ease-out`).

---

### 4. Component Architecture & Directory Organization

1. **Directory Boundaries**:
   - `src/components/shell/`: Core application framing (`AppWorkspaceShell`, `WorkspaceSidebar`, `ActivityRail`, `AppTopBar`, `AppStatusBar`, `CommandPalette`).
   - `src/components/workbenches/`: Domain-specific workbenches (`OrcaTabbedContainer`, `engineering/`, `research/`, `copilot/`).
   - `src/components/chat/`: Conversation and agent dispatch interfaces (`ConversationPane`, `ComposerPlusMenu`, `ModelSelector`).
   - `src/components/modals/`: Global dialog overlays (`SettingsModal`, `AiSetupModal`, `CreateTaskModal`, `GitHubAuthModal`).
   - `src/components/viewer/`: Dedicated content viewers (`MarkdownViewer`, `PdfViewer`, `CompanionFileViewer`).

2. **File Size Limit**:
   - No single component file may exceed **600 lines**.
   - If a component grows beyond this threshold, decompose its internal sub-views (rows, cards, modal panes) into discrete child components or custom hooks within the same module directory.

---

### 5. Native Tauri v2 Safety & Web Fallback

1. **Safe Native Invocation**:
   - Every invocation of `@tauri-apps/api` (window, webview, event, dialog) must check `isTauri` or be wrapped in a `try...catch` block.
   - The desktop bundle must never crash when viewed in a standard web browser environment.

2. **Daemon RPC Communication**:
   - Use `daemonClient.request<T>(method, params)` in `src/api/daemon_client.ts` as the sole bridge to the daemon.
   - Raw untyped `fetch` calls bypassing `DaemonClient` are prohibited.

---

### 6. Mandatory Quality Gates

Every frontend edit must pass both quality gates prior to declaring completion:

1. **TypeScript & Vite Compilation Gate**:
   ```bash
   npm --prefix Custos/ui/desktop run build
   ```
   *Requirement:* Exit code **0** with zero `tsc` type errors or Vite bundling warnings.

2. **Canvas Smoke Test Suite Gate**:
   ```bash
   node Custos/tools/canvas-smoke.mjs
   ```
   *Requirement:* All **26/26 tests must pass** (`pass 26, fail 0`). Never break tab tree operations, split-pane mechanics, draft persistence, or workspace recovery.
