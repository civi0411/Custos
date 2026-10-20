import React, { createContext, useContext, useState, useEffect, useCallback, useMemo } from 'react';
import { screensForWorkbench, type Screen, type ScreenPreset } from '@/types/screen';
import { createCanvas, createCanvasFromPreset, type CanvasState } from '@/types/canvas';
import type { OrcaTab } from '@/components/workbenches/OrcaTabbedContainer';
import { useAppContext } from './AppContext';

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

const ScreenContext = createContext<ScreenContextType | undefined>(undefined);

export const ScreenProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const {
    activeWorkbench,
    setActiveWorkbench,
    activeWorkspace,
    currentProject,
    activeScreenId,
    setActiveScreenId,
    initScreenTabs,
    setActiveWorkspaceId,
  } = useAppContext();
  const currentWb = activeWorkbench || 'engineering';
  const scope = currentProject || 'default';
  const storageKey = `custos.screens.${scope}`;

  const defaultScreens: Screen[] = useMemo(() => [
    {
      id: 'screen-default',
      name: currentWb === 'research' ? 'Lab Desk' : currentWb === 'copilot' ? 'Executive Desk' : 'Main Dev',
      icon: currentWb === 'research' ? 'FlaskConical' : currentWb === 'copilot' ? 'CheckSquare' : 'Code',
      projectId: activeWorkspace?.metadata?.project_id || currentProject || 'default',
      homeWorkbench: currentWb,
      defaultWorkspaceId: activeWorkspace?.id,
      canvasState: createCanvas(currentWb === 'research' ? ['conversation', 'lab'] : currentWb === 'copilot' ? ['conversation', 'tasks'] : ['conversation', 'terminal']),
      tabRegistry: {},
      isPinned: true,
      createdAt: Date.now(),
      lastUsedAt: Date.now(),
    },
  ], [currentWb, activeWorkspace?.metadata?.project_id, activeWorkspace?.id, currentProject]);

  const [hydratedKey, setHydratedKey] = useState<string>(storageKey);

  const [screens, setScreens] = useState<Screen[]>(() => {
    try {
      const stored = localStorage.getItem(storageKey);
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed) && parsed.length > 0) return parsed;
      }
    } catch {}
    return defaultScreens;
  });

  // When storageKey changes (e.g. project switch), reload stored screens safely
  useEffect(() => {
    if (hydratedKey === storageKey) return;
    try {
      const stored = localStorage.getItem(storageKey);
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed) && parsed.length > 0) {
          setScreens(parsed);
          setHydratedKey(storageKey);
          return;
        }
      }
    } catch {}
    setScreens(defaultScreens);
    setHydratedKey(storageKey);
  }, [storageKey, hydratedKey, defaultScreens]);

  // Persist screens only after hydration for the current storageKey
  useEffect(() => {
    if (hydratedKey !== storageKey) return;
    try {
      localStorage.setItem(storageKey, JSON.stringify(screens));
    } catch {}
  }, [storageKey, screens, hydratedKey]);

  // Keep activeScreenId valid and aligned with currentWorkbench
  useEffect(() => {
    if (hydratedKey !== storageKey) return;
    if (screens.length === 0) return;

    // Check if the current active screen belongs to currentWb
    const currentActive = screens.find((s) => s.id === activeScreenId);
    const activeBelongsToCurrentWb =
      currentActive &&
      (currentActive.homeWorkbench === currentWb ||
        (!currentActive.homeWorkbench && currentWb === 'engineering'));

    if (activeBelongsToCurrentWb) return;

    // Look for existing screens belonging to currentWb, sorted by lastUsedAt
    const wbScreens = screensForWorkbench(screens, currentWb)
      .sort((a, b) => (b.lastUsedAt || 0) - (a.lastUsedAt || 0));

    if (wbScreens.length > 0) {
      setActiveScreenId(wbScreens[0].id);
    } else {
      // Create initial default screen for this workbench
      const newId = `screen-${currentWb}-default`;
      const newName =
        currentWb === 'research'
          ? 'Research Desk'
          : currentWb === 'copilot'
          ? 'Executive Desk'
          : 'Main Dev';
      const newIcon =
        currentWb === 'research'
          ? 'FlaskConical'
          : currentWb === 'copilot'
          ? 'CheckSquare'
          : 'Code';
      const initialTabIds =
        currentWb === 'research'
          ? ['conversation', 'lab']
          : currentWb === 'copilot'
          ? ['conversation', 'tasks']
          : ['conversation', 'terminal'];

      const newScreen: Screen = {
        id: newId,
        name: newName,
        icon: newIcon,
        projectId: activeWorkspace?.metadata?.project_id || currentProject || 'default',
        homeWorkbench: currentWb,
        defaultWorkspaceId: activeWorkspace?.id,
        canvasState: createCanvas(initialTabIds),
        tabRegistry: {},
        isPinned: true,
        createdAt: Date.now(),
        lastUsedAt: Date.now(),
      };
      setScreens((prev) => [...prev, newScreen]);
      setActiveScreenId(newId);
    }
  }, [screens, activeScreenId, currentWb, activeWorkspace?.metadata?.project_id, activeWorkspace?.id, currentProject, setActiveScreenId]);

  const selectScreen = useCallback((screenId: string) => {
    setActiveScreenId(screenId);
    setScreens((prev) => {
      const found = prev.find((s) => s.id === screenId);
      if (found?.homeWorkbench) {
        setActiveWorkbench(found.homeWorkbench);
      }
      return prev.map((s) => (s.id === screenId ? { ...s, lastUsedAt: Date.now() } : s));
    });
  }, [setActiveScreenId, setActiveWorkbench]);

  const createScreen = useCallback((preset?: ScreenPreset) => {
    const newId = `screen-${Date.now()}`;
    const newName = preset?.name || `Screen ${screens.length + 1}`;
    const initialTabsList: OrcaTab[] = preset?.initialTabs?.map((t) => ({
      id: t.id,
      type: t.id as any,
      title: t.title,
      url: `custos://resource/${t.id}`,
    })) || (currentWb === 'research'
      ? [
          { id: 'conversation', type: 'conversation', title: 'AI Assistant', url: 'custos://chat' },
          { id: 'lab', type: 'lab', title: 'Lab', url: 'custos://resource/lab' },
        ]
      : currentWb === 'copilot'
      ? [
          { id: 'conversation', type: 'conversation', title: 'AI Assistant', url: 'custos://chat' },
          { id: 'drafts', type: 'drafts', title: 'Drafts', url: 'custos://resource/drafts' },
        ]
      : [
          { id: 'conversation', type: 'conversation', title: 'AI Assistant', url: 'custos://chat' },
          { id: 'terminal', type: 'terminal', title: 'Terminal', url: 'custos://resource/terminal' },
        ]);

    const initialTabIds = initialTabsList.map((t) => t.id);

    // Initialize tabs for the new screen in AppContext so it never inherits old screen tabs
    initScreenTabs(newId, initialTabsList, initialTabIds[0]);

    const newScreen: Screen = {
      id: newId,
      name: newName,
      icon: preset?.icon || 'Code',
      projectId: activeWorkspace?.metadata?.project_id || currentProject || 'default',
      homeWorkbench: currentWb,
      defaultWorkspaceId: activeWorkspace?.id,
      canvasState: createCanvasFromPreset(initialTabIds, preset?.splitConfig),
      tabRegistry: {},
      isPinned: false,
      createdAt: Date.now(),
      lastUsedAt: Date.now(),
    };
    setScreens((prev) => [...prev, newScreen]);
    setActiveScreenId(newId);
    if (newScreen.homeWorkbench) {
      setActiveWorkbench(newScreen.homeWorkbench);
    }
  }, [screens.length, activeWorkspace?.metadata?.project_id, activeWorkspace?.id, currentProject, currentWb, initScreenTabs, setActiveScreenId, setActiveWorkbench]);

  const closeScreen = useCallback((screenId: string) => {
    setScreens((prev) => {
      if (screensForWorkbench(prev, currentWb).length <= 1) return prev;
      const filtered = prev.filter((s) => s.id !== screenId);
      if (activeScreenId === screenId) {
        const nextScreen = screensForWorkbench(filtered, currentWb)
          .sort((a, b) => (b.lastUsedAt || 0) - (a.lastUsedAt || 0))[0];
        if (nextScreen) {
          setActiveScreenId(nextScreen.id);
        }
      }
      return filtered;
    });
  }, [activeScreenId, currentWb, setActiveScreenId]);

  const togglePinScreen = useCallback((screenId: string) => {
    setScreens((prev) =>
      prev.map((s) => (s.id === screenId ? { ...s, isPinned: !s.isPinned } : s))
    );
  }, []);

  const renameScreen = useCallback((screenId: string, newName: string) => {
    if (!newName.trim()) return;
    setScreens((prev) =>
      prev.map((s) => (s.id === screenId ? { ...s, name: newName.trim() } : s))
    );
  }, []);

  const updateActiveScreenLayout = useCallback((canvasState: CanvasState) => {
    setScreens(previous => applyScreenCanvas(previous, activeScreenId, canvasState));
  }, [activeScreenId]);

  const updateActiveScreen = useCallback((updates: Partial<Screen>) => {
    setScreens((prev) =>
      prev.map((s) => (s.id === activeScreenId ? { ...s, ...updates, lastUsedAt: Date.now() } : s))
    );
  }, [activeScreenId]);

  const setScreenWorkbench = useCallback((screenId: string, wb: 'engineering' | 'research' | 'copilot') => {
    setScreens((prev) =>
      prev.map((s) => (s.id === screenId ? { ...s, homeWorkbench: wb, lastUsedAt: Date.now() } : s))
    );
  }, []);

  const promoteEphemeralScreen = useCallback((screenId: string) => {
    setScreens((prev) =>
      prev.map((s) => (s.id === screenId ? { ...s, isEphemeral: false } : s))
    );
  }, []);

  const revealWorktreeScreen = useCallback(
    (worktree: {
      id: string;
      name: string;
      branch?: string;
      path?: string;
      repoName?: string;
    }) => {
      // Step 1: Check if any screen is already anchored or defaulted to this worktree
      const existing = screens.find(
        (s) =>
          (s.anchor?.kind === 'worktree' && s.anchor.id === worktree.id) ||
          s.defaultWorkspaceId === worktree.id
      );

      if (existing) {
        selectScreen(existing.id);
        setActiveWorkspaceId(worktree.id);
        return;
      }

      const screenLabel = worktree.branch
        ? `${worktree.repoName ? `${worktree.repoName} @ ` : ''}${worktree.branch}`
        : worktree.name;

      // Step 2: If we have only 1 default untouched screen, anchor it directly
      if (
        screens.length === 1 &&
        screens[0].id === 'screen-default' &&
        !screens[0].anchor
      ) {
        const updatedScreen: Screen = {
          ...screens[0],
          name: screenLabel,
          icon: 'GitBranch',
          defaultWorkspaceId: worktree.id,
          anchor: {
            kind: 'worktree',
            id: worktree.id,
            label: screenLabel,
            path: worktree.path,
            repoName: worktree.repoName,
            branch: worktree.branch,
          },
          lastUsedAt: Date.now(),
        };
        setScreens([updatedScreen]);
        setActiveWorkspaceId(worktree.id);
        return;
      }

      // Step 3: Otherwise, materialize a new screen for this worktree
      const newId = `screen-wt-${worktree.id}`;
      const initialTabsList: OrcaTab[] = [
        {
          id: `conv_${worktree.id}`,
          type: 'conversation',
          title: `${screenLabel}: Chat`,
          url: `custos://chat?workspace=${worktree.id}`,
          workspaceId: worktree.id,
        },
        {
          id: `terminal_${worktree.id}`,
          type: 'terminal',
          title: `${screenLabel}: Terminal`,
          url: `custos://terminal?workspace=${worktree.id}`,
          workspaceId: worktree.id,
        },
      ];
      const initialTabIds = initialTabsList.map((t) => t.id);

      initScreenTabs(newId, initialTabsList, initialTabIds[0]);

      const newScreen: Screen = {
        id: newId,
        name: screenLabel,
        icon: 'GitBranch',
        projectId: activeWorkspace?.metadata?.project_id || currentProject || 'default',
        homeWorkbench: 'engineering',
        defaultWorkspaceId: worktree.id,
        anchor: {
          kind: 'worktree',
          id: worktree.id,
          label: screenLabel,
          path: worktree.path,
          repoName: worktree.repoName,
          branch: worktree.branch,
        },
        canvasState: createCanvas(initialTabIds),
        tabRegistry: {},
        isPinned: false,
        createdAt: Date.now(),
        lastUsedAt: Date.now(),
      };

      setScreens((prev) => [...prev, newScreen]);
      setActiveScreenId(newId);
      setActiveWorkspaceId(worktree.id);
    },
    [
      screens,
      selectScreen,
      setActiveWorkspaceId,
      initScreenTabs,
      activeWorkspace?.metadata?.project_id,
      currentProject,
      setActiveScreenId,
    ]
  );

  const revealTaskScreen = useCallback(
    (task: { id: string; title: string; packId?: string }) => {
      const existing = screens.find(
        (s) => s.anchor?.kind === 'task' && s.anchor.id === task.id
      );

      if (existing) {
        selectScreen(existing.id);
        return;
      }

      const newId = `screen-task-${task.id}`;
      const screenLabel = `Task: ${task.title}`;
      const initialTabsList: OrcaTab[] = [
        {
          id: `task_${task.id}`,
          type: 'tasks',
          title: screenLabel,
          url: `custos://task?id=${task.id}`,
          taskId: task.id,
        },
        {
          id: `conv_task_${task.id}`,
          type: 'conversation',
          title: `${task.title} - Chat`,
          url: `custos://chat?taskId=${task.id}`,
          taskId: task.id,
        },
      ];
      const initialTabIds = initialTabsList.map((t) => t.id);
      initScreenTabs(newId, initialTabsList, initialTabIds[0]);

      const newScreen: Screen = {
        id: newId,
        name: screenLabel,
        icon: 'CheckSquare',
        projectId: currentProject || 'default',
        homeWorkbench:
          task.packId === 'code' ? 'engineering' : task.packId === 'research' ? 'research' : 'copilot',
        anchor: {
          kind: 'task',
          id: task.id,
          label: screenLabel,
        },
        canvasState: createCanvas(initialTabIds),
        tabRegistry: {},
        isPinned: false,
        createdAt: Date.now(),
        lastUsedAt: Date.now(),
      };

      setScreens((prev) => [...prev, newScreen]);
      setActiveScreenId(newId);
      if (newScreen.homeWorkbench) {
        setActiveWorkbench(newScreen.homeWorkbench);
      }
    },
    [screens, selectScreen, initScreenTabs, currentProject, setActiveScreenId, setActiveWorkbench]
  );

  // Canvas actions
  const [canvasActions, setCanvasActions] = useState<{
    splitRight?: () => void;
    splitDown?: () => void;
    toggleZoom?: () => void;
    isZoomed?: boolean;
  }>({});

  const registerCanvasActions = useCallback((actions: {
    splitRight: () => void;
    splitDown: () => void;
    toggleZoom: () => void;
    isZoomed: boolean;
  } | null) => {
    setCanvasActions(actions || {});
  }, []);

  const activeScreen = useMemo(
    () => screens.find((s) => s.id === activeScreenId) || screens[0],
    [screens, activeScreenId]
  );

  const value = useMemo(
    () => ({
      screens,
      activeScreenId,
      activeScreen,
      selectScreen,
      createScreen,
      closeScreen,
      togglePinScreen,
      renameScreen,
      updateActiveScreenLayout,
      updateActiveScreen,
      setScreenWorkbench,
      revealWorktreeScreen,
      revealTaskScreen,
      promoteEphemeralScreen,
      splitRight: canvasActions.splitRight,
      splitDown: canvasActions.splitDown,
      toggleZoom: canvasActions.toggleZoom,
      isZoomed: canvasActions.isZoomed,
      registerCanvasActions,
    }),
    [
      screens,
      activeScreenId,
      activeScreen,
      selectScreen,
      createScreen,
      closeScreen,
      togglePinScreen,
      renameScreen,
      updateActiveScreenLayout,
      updateActiveScreen,
      setScreenWorkbench,
      canvasActions,
    ]
  );

  return <ScreenContext.Provider value={value}>{children}</ScreenContext.Provider>;
};

export const useScreenContext = () => {
  const context = useContext(ScreenContext);
  if (!context) {
    throw new Error('useScreenContext must be used within a ScreenProvider');
  }
  return context;
};
