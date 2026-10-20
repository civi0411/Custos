import type { CanvasState, ResourceRef } from './canvas';

/**
 * Descriptor of a tab inside a Screen pane (Tab Group).
 * Maps a visual tabId to concrete domain data (ResourceRef).
 */
export interface TabDescriptor {
  id: string;               // Unique Tab ID (e.g. 'tab_diff_1', 'tab_app_tsx')
  title: string;            // Tab title (e.g. 'App.tsx', 'wt-ui: Diff')
  icon?: string;            // Icon identifier
  ref: ResourceRef;         // Stable reference to workspace, file, diff, session, notebook
  isPinned?: boolean;       // Pinned tab
  isDirty?: boolean;        // Has unsaved edits
  closeable?: boolean;      // Can be closed
}

/**
 * Anchor definition: Identifies the worktree, task, or experiment this screen is centered on.
 * Used for automatic resolution and reveal without forcing an extra navigation tier.
 */
export interface ScreenAnchor {
  kind: 'worktree' | 'task' | 'experiment';
  id: string;               // Unique ID: workspaceId, taskId, or experimentId
  label: string;            // Human-readable label (e.g. 'feature/auth' or 'Custos @ main')
  path?: string;            // Filesystem checkout path for worktrees
  repoName?: string;        // Owning repository name
  branch?: string;          // Git branch name
}

/**
 * Screen: A named, persistent spatial layout configuration (LayoutGroup).
 * Decoupled from activeSessionId and activeWorkspaceId.
 */
export interface Screen {
  id: string;               // UUID of the screen (e.g. 'screen_dev_1', 'screen_compare_2')
  name: string;             // User-facing name (e.g. 'Standard Dev', 'Candidate Compare')
  icon?: string;            // Icon name (e.g. 'Code', 'GitCompare', 'Microscope')
  projectId: string;        // Project association for navigation
  homeWorkbench: 'engineering' | 'research' | 'copilot';
  
  // Natural anchor connecting this layout to a worktree or task
  anchor?: ScreenAnchor;
  
  // Mathematical recursive tree state (verified by canvas-smoke.mjs)
  canvasState: CanvasState;
  
  // Registry of all tabs active in this screen
  tabRegistry: Record<string, TabDescriptor>;
  
  // Screen-level workspace state
  defaultWorkspaceId?: string; // Default worktree/workspace context for newly opened tabs
  activeSessionId?: string;
  selectedTaskId?: string | null;
  sidebarView?: 'chats' | 'tasks';
  isWebTabOpen?: boolean;
  isWebTabExpanded?: boolean;

  // Lifecycle flags
  isPinned?: boolean;       // Pinned screens are protected from LRU eviction
  isEphemeral?: boolean;    // Ephemeral preview screen, promoted upon interaction
  createdAt: number;
  lastUsedAt: number;
}

export type WorkbenchId = Screen['homeWorkbench'];

/** Return only layouts owned by this workbench; legacy unscoped layouts belong to Engineering. */
export function screensForWorkbench(screens: Screen[], workbench: WorkbenchId): Screen[] {
  return screens.filter((screen) => screen.homeWorkbench === workbench || (!screen.homeWorkbench && workbench === 'engineering'));
}

/**
 * Standard layout presets for each workbench lens.
 */
export interface ScreenPreset {
  id: string;
  name: string;
  icon: string;
  workbench: 'engineering' | 'research' | 'copilot';
  description: string;
  initialTabs: Array<{ id: string; title: string; ref: ResourceRef }>;
  splitConfig?: {
    axis: 'horizontal' | 'vertical';
    ratio: number;
  };
}

export const WORKBENCH_SCREEN_PRESETS: ScreenPreset[] = [
  // Engineering Presets
  {
    id: 'preset_eng_dev',
    name: 'Dev & Edit',
    icon: 'Code',
    workbench: 'engineering',
    description: 'Conversation beside active code editor',
    initialTabs: [
      { id: 'conversation', title: 'Conversation', ref: { kind: 'conversation' } },
      { id: 'files', title: 'Code Editor', ref: { kind: 'file' } },
    ],
    splitConfig: { axis: 'horizontal', ratio: 45 },
  },
  {
    id: 'preset_eng_compare',
    name: 'Candidate Compare',
    icon: 'GitCompare',
    workbench: 'engineering',
    description: 'Compare diffs of multiple candidate worktrees side by side',
    initialTabs: [
      { id: 'conversation', title: 'Coordinator', ref: { kind: 'conversation' } },
      { id: 'changes', title: 'Candidate Diff', ref: { kind: 'diff' } },
    ],
    splitConfig: { axis: 'horizontal', ratio: 40 },
  },
  {
    id: 'preset_eng_terminal',
    name: 'Full Stack',
    icon: 'Terminal',
    workbench: 'engineering',
    description: 'Code editor with PTY terminal console below',
    initialTabs: [
      { id: 'files', title: 'Code Editor', ref: { kind: 'file' } },
      { id: 'terminal', title: 'Terminal', ref: { kind: 'terminal' } },
    ],
    splitConfig: { axis: 'vertical', ratio: 65 },
  },

  // Research Presets
  {
    id: 'preset_res_literature',
    name: 'Literature Study',
    icon: 'BookOpen',
    workbench: 'research',
    description: 'Research chat beside scientific paper & PDF reader',
    initialTabs: [
      { id: 'conversation', title: 'Research Chat', ref: { kind: 'conversation' } },
      { id: 'literature', title: 'Literature Sources', ref: { kind: 'source' } },
    ],
    splitConfig: { axis: 'horizontal', ratio: 40 },
  },
  {
    id: 'preset_res_lab',
    name: 'Lab & Compute',
    icon: 'Cpu',
    workbench: 'research',
    description: 'Interactive Jupyter Notebook experiment and 3D visualization',
    initialTabs: [
      { id: 'lab', title: 'Lab Notebook', ref: { kind: 'notebook' } },
      { id: 'artifacts', title: '3D Structure', ref: { kind: 'artifact' } },
    ],
    splitConfig: { axis: 'horizontal', ratio: 55 },
  },

  // Copilot Presets
  {
    id: 'preset_cop_ops',
    name: 'Ops & Outbox',
    icon: 'ShieldCheck',
    workbench: 'copilot',
    description: 'Executive chat beside calendar and approved Outbox queue',
    initialTabs: [
      { id: 'conversation', title: 'Executive Chat', ref: { kind: 'conversation' } },
      { id: 'outbox', title: 'Outbox Approval', ref: { kind: 'outbox' } },
    ],
    splitConfig: { axis: 'horizontal', ratio: 50 },
  },
];
