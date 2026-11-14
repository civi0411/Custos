import { type Screen, type ScreenPreset } from '@/types/screen';
import { type CanvasState } from '@/types/canvas';
import type { OrcaTab } from '@/components/workbenches/OrcaTabbedContainer';

export {
  ScreenContext,
  ScreenProvider,
  useScreenContext,
} from './screen/ScreenProvider';
export * from './screen/anchors';

export interface ScreenContextType {
  screens: Screen[];
  activeScreenId: string;
  activeScreen: Screen | undefined;
  selectScreen: (screenId: string) => void;
  createScreen: (preset?: ScreenPreset) => void;
  closeScreen: (screenId: string) => void;
  togglePinScreen: (screenId: string) => void;
  renameScreen: (screenId: string, newName: string) => void;
  updateActiveScreenLayout: (canvasState: CanvasState) => void;
  updateActiveScreen: (updates: Partial<Screen>) => void;
  setScreenWorkbench: (screenId: string, wb: 'engineering' | 'research' | 'copilot') => void;
  
  // Worktree & Resource Anchored Materialization
  revealWorktreeScreen: (worktree: {
    id: string;
    name: string;
    branch?: string;
    path?: string;
    repoName?: string;
  }) => void;
  revealTaskScreen: (task: {
    id: string;
    title: string;
    packId?: string;
  }) => void;
  promoteEphemeralScreen: (screenId: string) => void;
  
  // Focused pane actions registered from active canvas
  splitRight?: () => void;
  splitDown?: () => void;
  toggleZoom?: () => void;
  isZoomed?: boolean;
  registerCanvasActions: (actions: {
    splitRight: () => void;
    splitDown: () => void;
    toggleZoom: () => void;
    isZoomed: boolean;
  } | null) => void;
}

/** Ignore identical layout observations to prevent canvas/context feedback loops. */
export function applyScreenCanvas(screens: Screen[], screenId: string, canvasState: CanvasState): Screen[] {
  const active = screens.find(screen => screen.id === screenId);
  if (!active || JSON.stringify(active.canvasState) === JSON.stringify(canvasState)) return screens;
  return screens.map(screen => screen.id === screenId ? { ...screen, canvasState } : screen);
}

export interface ScreenLayoutSnapshotV2 {
  version: 2;
  scope: string;
  screens: Screen[];
  activeScreenId: string;
  timestamp: number;
}

export function defaultTabsForScreen(screen: Partial<Screen>): OrcaTab[] {
  if (screen.anchor?.kind === 'worktree') {
    const wtId = screen.anchor.id;
    const label = screen.anchor.label || screen.name || 'Worktree';
    return [
      {
        id: `conv_${wtId}`,
        type: 'conversation',
        title: `${label}: Chat`,
        url: `custos://chat?workspace=${wtId}`,
        workspaceId: wtId,
      },
      {
        id: `terminal_${wtId}`,
        type: 'terminal',
        title: `${label}: Terminal`,
        url: `custos://terminal?workspace=${wtId}`,
        workspaceId: wtId,
      },
    ];
  }
  if (screen.anchor?.kind === 'task') {
    const taskId = screen.anchor.id;
    const label = screen.name || `Task: ${taskId}`;
    return [
      {
        id: `task_${taskId}`,
        type: 'tasks',
        title: label,
        url: `custos://task?id=${taskId}`,
        taskId,
      },
      {
        id: `conv_task_${taskId}`,
        type: 'conversation',
        title: `${label} - Chat`,
        url: `custos://chat?taskId=${taskId}`,
        taskId,
      },
    ];
  }
  const wb = screen.homeWorkbench || 'engineering';
  if (wb === 'research') {
    return [
      { id: 'conversation', type: 'conversation', title: 'AI Assistant', url: 'custos://chat' },
      { id: 'lab', type: 'lab', title: 'Lab', url: 'custos://resource/lab' },
    ];
  }
  if (wb === 'copilot') {
    return [
      { id: 'conversation', type: 'conversation', title: 'AI Assistant', url: 'custos://chat' },
      { id: 'drafts', type: 'drafts', title: 'Drafts', url: 'custos://resource/drafts' },
    ];
  }
  if (wb === 'quant') {
    return [
      { id: 'alpha', type: 'alpha', title: 'Alpha Studio', url: 'custos://resource/alpha' },
      { id: 'tearsheet', type: 'tearsheet', title: 'Tearsheet', url: 'custos://resource/tearsheet' },
    ];
  }
  return [
    { id: 'conversation', type: 'conversation', title: 'AI Assistant', url: 'custos://chat' },
    { id: 'terminal', type: 'terminal', title: 'Terminal', url: 'custos://resource/terminal' },
  ];
}

export function loadAndMigrateScreens(
  scope: string,
  currentProject: string | undefined,
  defaultScreens: Screen[]
): { screens: Screen[]; activeScreenId?: string; futureSnapshotPreserved?: boolean } {
  const v2Key = `custos.screen_layout.v2.${scope}`;
  const legacyScreensKey = `custos.screens.${scope}`;

  // Read legacy resource tabs if available
  let legacyTabsMap: Record<string, { tabs: OrcaTab[]; activeTabId?: string }> = {};
  try {
    const rawResourceTabs = localStorage.getItem('custos.resource-tabs.v1');
    if (rawResourceTabs) {
      const parsed = JSON.parse(rawResourceTabs);
      if (parsed && parsed.scopes && typeof parsed.scopes === 'object') {
        legacyTabsMap = parsed.scopes;
      }
    }
  } catch {}

  const resolveTabs = (screen: Partial<Screen>): OrcaTab[] => {
    if (Array.isArray(screen.tabs) && screen.tabs.length > 0) return screen.tabs;
    const scopeKeys = [
      screen.id ? JSON.stringify([currentProject, screen.id]) : '',
      screen.id ? JSON.stringify([scope, screen.id]) : '',
      screen.id || '',
    ].filter(Boolean);

    for (const key of scopeKeys) {
      const entry = legacyTabsMap[key];
      if (entry && Array.isArray(entry.tabs) && entry.tabs.length > 0) {
        return entry.tabs;
      }
    }

    if (typeof defaultTabsForScreen === 'function') {
      try {
        return defaultTabsForScreen(screen);
      } catch {}
    }

    const wb = screen.homeWorkbench || 'engineering';
    if (wb === 'research') {
      return [
        { id: 'conversation', type: 'conversation', title: 'AI Assistant', url: 'custos://chat' },
        { id: 'lab', type: 'lab', title: 'Lab', url: 'custos://resource/lab' },
      ];
    }
    if (wb === 'copilot') {
      return [
        { id: 'conversation', type: 'conversation', title: 'AI Assistant', url: 'custos://chat' },
        { id: 'drafts', type: 'drafts', title: 'Drafts', url: 'custos://resource/drafts' },
      ];
    }
    return [
      { id: 'conversation', type: 'conversation', title: 'AI Assistant', url: 'custos://chat' },
      { id: 'terminal', type: 'terminal', title: 'Terminal', url: 'custos://resource/terminal' },
    ];
  };

  // 1. Try reading unified V2 snapshot
  try {
    const rawV2 = localStorage.getItem(v2Key);
    if (rawV2) {
      const parsed = JSON.parse(rawV2);
      if (parsed && typeof parsed.version === 'number') {
        if (parsed.version > 2) {
          // Future snapshot version detected: preserve in backup and avoid downgrading overwrite
          try {
            localStorage.setItem(`${v2Key}.unrecognized_backup`, rawV2);
          } catch {}
          return {
            screens: Array.isArray(parsed.screens) && parsed.screens.length > 0 ? parsed.screens : defaultScreens,
            activeScreenId: parsed.activeScreenId || defaultScreens[0]?.id || 'screen-default',
            futureSnapshotPreserved: true,
          };
        }
        if (parsed.version === 2 && Array.isArray(parsed.screens) && parsed.screens.length > 0) {
          const screensWithTabs: Screen[] = parsed.screens.map((screen: Screen) => {
            const tabs = resolveTabs(screen);
            const activeTabId = screen.activeTabId || tabs[0]?.id || '';
            return {
              ...screen,
              tabs,
              activeTabId,
            };
          });
          const activeId = parsed.activeScreenId && screensWithTabs.some(s => s.id === parsed.activeScreenId)
            ? parsed.activeScreenId
            : screensWithTabs[0].id;
          return {
            screens: screensWithTabs,
            activeScreenId: activeId,
            futureSnapshotPreserved: false,
          };
        }
      }
    }
  } catch {}

  // 2. Migration: read legacy keys
  try {
    let screensToMigrate: Screen[] = [];
    const legacyRawScreens = localStorage.getItem(legacyScreensKey);
    if (legacyRawScreens) {
      const parsed = JSON.parse(legacyRawScreens);
      if (Array.isArray(parsed) && parsed.length > 0) {
        screensToMigrate = parsed;
      }
    }

    if (screensToMigrate.length === 0) {
      screensToMigrate = defaultScreens;
    }

    // Try reading legacy canvas state if a screen has empty or default canvas
    screensToMigrate = screensToMigrate.map((screen) => {
      const screenScope = JSON.stringify([currentProject, screen.id]);
      const legacyCanvasKey = `custos.canvas.v1:${screenScope}`;
      let canvasState = screen.canvasState;
      try {
        const rawCanvas = localStorage.getItem(legacyCanvasKey);
        if (rawCanvas && (!canvasState || !canvasState.root)) {
          const parsedCanvas = JSON.parse(rawCanvas);
          if (parsedCanvas && parsedCanvas.root) {
            canvasState = parsedCanvas;
          }
        }
      } catch {}

      const tabs = [...resolveTabs(screen)];
      const scopeKeys = [
        screen.id ? JSON.stringify([currentProject, screen.id]) : '',
        screen.id ? JSON.stringify([scope, screen.id]) : '',
        screen.id || '',
      ].filter(Boolean);
      let legacyActiveTabId: string | undefined;
      for (const key of scopeKeys) {
        if (legacyTabsMap[key]?.activeTabId) {
          legacyActiveTabId = legacyTabsMap[key].activeTabId;
          break;
        }
      }

      // Reconcile tabs with canvas tabIds so canvas references are never orphan
      if (canvasState && canvasState.root) {
        const tabIdsInCanvas: string[] = [];
        const collectTabIds = (node: any) => {
          if (!node) return;
          if (node.kind === 'group' && Array.isArray(node.tabIds)) {
            tabIdsInCanvas.push(...node.tabIds);
          } else if (node.kind === 'split') {
            collectTabIds(node.first);
            collectTabIds(node.second);
          }
        };
        collectTabIds(canvasState.root);
        const tabIdSet = new Set(tabs.map(t => t.id));
        for (const cid of tabIdsInCanvas) {
          if (!tabIdSet.has(cid)) {
            tabs.push({
              id: cid,
              type: cid.startsWith('conv') ? 'conversation' : cid.startsWith('term') ? 'terminal' : 'files',
              title: cid,
              url: `custos://tab/${cid}`,
            });
            tabIdSet.add(cid);
          }
        }
      }

      const activeTabId = screen.activeTabId || legacyActiveTabId || tabs[0]?.id || '';

      const fallbackCanvas: CanvasState = {
        version: 1,
        root: { kind: 'group', id: 'root', tabIds: tabs.map(t => t.id), activeTabId },
        focusedGroupId: 'root',
        zoomedGroupId: null,
      };

      return {
        ...screen,
        canvasState: canvasState || fallbackCanvas,
        tabs,
        activeTabId,
      };
    });

    const activeId = screensToMigrate[0]?.id || 'screen-default';
    const snapshot: ScreenLayoutSnapshotV2 = {
      version: 2,
      scope,
      screens: screensToMigrate,
      activeScreenId: activeId,
      timestamp: Date.now(),
    };

    // Safe write-then-readback validation
    try {
      const serialized = JSON.stringify(snapshot);
      localStorage.setItem(v2Key, serialized);
      const readBack = localStorage.getItem(v2Key);
      if (readBack === serialized) {
        // Read-back confirmed, snapshot established
      }
    } catch {}

    return { screens: screensToMigrate, activeScreenId: activeId, futureSnapshotPreserved: false };
  } catch {
    return { screens: defaultScreens, activeScreenId: defaultScreens[0]?.id || 'screen-default', futureSnapshotPreserved: false };
  }
}
