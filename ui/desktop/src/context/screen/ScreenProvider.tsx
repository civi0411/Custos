import React, { createContext, useContext, useState, useEffect, useCallback, useMemo } from 'react';
import { screensForWorkbench, type Screen, type ScreenPreset } from '@/types/screen';
import { createCanvas, createCanvasFromPreset, type CanvasState } from '@/types/canvas';
import type { OrcaTab } from '@/components/workbenches/OrcaTabbedContainer';
import { useAppContext } from '../AppContext';
import {
  type ScreenContextType,
  type ScreenLayoutSnapshotV2,
  applyScreenCanvas,
  defaultTabsForScreen,
  loadAndMigrateScreens,
} from '../ScreenContext';
import {
  buildWorktreeScreenTabs,
  createWorktreeScreen,
  buildTaskScreenTabs,
  createTaskScreen,
  type WorktreeDescriptor,
  type TaskDescriptor,
} from './anchors';

export const ScreenContext = createContext<ScreenContextType | undefined>(undefined);

export const ScreenProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const {
    activeWorkbench,
    setActiveWorkbench,
    activeWorkspace,
    currentProject,
    setActiveScreenId: setAppActiveScreenId,
    initScreenTabs,
    setActiveWorkspaceId,
  } = useAppContext();
  const currentWb = activeWorkbench || 'engineering';
  const scope = currentProject || 'default';
  const storageKey = `custos.screens.${scope}`;

  const defaultScreens: Screen[] = useMemo(() => {
    const initialScreen: Screen = {
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
    };
    initialScreen.tabs = defaultTabsForScreen(initialScreen);
    initialScreen.activeTabId = initialScreen.tabs[0]?.id || '';
    return [initialScreen];
  }, [currentWb, activeWorkspace?.metadata?.project_id, activeWorkspace?.id, currentProject]);

  // Decode snapshot at initial mount
  const [screens, setScreens] = useState<Screen[]>(() => {
    return loadAndMigrateScreens(scope, currentProject, defaultScreens).screens;
  });

  const [activeScreenId, setActiveScreenId] = useState<string>(() => {
    const initial = loadAndMigrateScreens(scope, currentProject, defaultScreens);
    return initial.activeScreenId || initial.screens[0]?.id || 'screen-default';
  });

  const [futureSnapshotPreserved, setFutureSnapshotPreserved] = useState<boolean>(() => {
    return !!loadAndMigrateScreens(scope, currentProject, defaultScreens).futureSnapshotPreserved;
  });

  // hydratedKey tracks whether the state matches the current storageKey
  const [hydratedKey, setHydratedKey] = useState<string>('');

  // Initial sync & project scope switch effect: decode snapshot and establish state
  useEffect(() => {
    if (hydratedKey === storageKey) return;
    const migrated = loadAndMigrateScreens(scope, currentProject, defaultScreens);
    setFutureSnapshotPreserved(!!migrated.futureSnapshotPreserved);
    setScreens(migrated.screens);
    const targetId = migrated.activeScreenId && migrated.screens.some(s => s.id === migrated.activeScreenId)
      ? migrated.activeScreenId
      : (migrated.screens[0]?.id || 'screen-default');
    setActiveScreenId(targetId);
    setAppActiveScreenId(targetId);

    // Initialize tabs for target screen in AppContext so active screen never inherits stale tabs
    const targetScreen = migrated.screens.find(s => s.id === targetId) || migrated.screens[0];
    if (targetScreen) {
      const tabs = targetScreen.tabs && targetScreen.tabs.length > 0
        ? targetScreen.tabs
        : defaultTabsForScreen(targetScreen);
      const activeTab = targetScreen.activeTabId || tabs[0]?.id || '';
      initScreenTabs(targetId, tabs, activeTab);
    }

    setHydratedKey(storageKey);
  }, [storageKey, hydratedKey, defaultScreens, scope, currentProject, setAppActiveScreenId, initScreenTabs]);

  // Persist screens only after hydration has completed for the current storageKey
  useEffect(() => {
    if (!hydratedKey || hydratedKey !== storageKey) return;
    if (screens.length === 0) return;
    if (futureSnapshotPreserved) {
      return;
    }

    const validActiveId = screens.some(s => s.id === activeScreenId)
      ? activeScreenId
      : (screens[0]?.id || 'screen-default');

    try {
      const snapshot: ScreenLayoutSnapshotV2 = {
        version: 2,
        scope,
        screens,
        activeScreenId: validActiveId,
        timestamp: Date.now(),
      };
      localStorage.setItem(`custos.screen_layout.v2.${scope}`, JSON.stringify(snapshot));
      localStorage.setItem(storageKey, JSON.stringify(screens));
    } catch {}
  }, [storageKey, screens, activeScreenId, hydratedKey, scope, futureSnapshotPreserved]);

  // Keep activeScreenId valid and aligned with currentWorkbench
  useEffect(() => {
    if (!hydratedKey || hydratedKey !== storageKey) return;
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
      const targetScreen = wbScreens[0];
      setActiveScreenId(targetScreen.id);
      setAppActiveScreenId(targetScreen.id);
      const tabs = targetScreen.tabs && targetScreen.tabs.length > 0
        ? targetScreen.tabs
        : defaultTabsForScreen(targetScreen);
      initScreenTabs(targetScreen.id, tabs, targetScreen.activeTabId || tabs[0]?.id || '');
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
      newScreen.tabs = defaultTabsForScreen(newScreen);
      newScreen.activeTabId = initialTabIds[0] || '';

      initScreenTabs(newId, newScreen.tabs, newScreen.activeTabId);
      setScreens((prev) => [...prev, newScreen]);
      setActiveScreenId(newId);
      setAppActiveScreenId(newId);
    }
  }, [screens, activeScreenId, currentWb, activeWorkspace?.metadata?.project_id, activeWorkspace?.id, currentProject, hydratedKey, storageKey, setAppActiveScreenId, initScreenTabs]);

  const selectScreen = useCallback((screenId: string) => {
    setActiveScreenId(screenId);
    setAppActiveScreenId(screenId);
    const found = screens.find((s) => s.id === screenId);
    if (found) {
      if (found.homeWorkbench) {
        setActiveWorkbench(found.homeWorkbench);
      }
      const screenTabs = found.tabs && found.tabs.length > 0
        ? found.tabs
        : defaultTabsForScreen(found);
      const activeTab = found.activeTabId || screenTabs[0]?.id || '';
      initScreenTabs(screenId, screenTabs, activeTab);
    }
    setScreens((prev) => prev.map((s) => (s.id === screenId ? { ...s, lastUsedAt: Date.now() } : s)));
  }, [screens, setAppActiveScreenId, setActiveWorkbench, initScreenTabs]);

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
      tabs: initialTabsList,
      activeTabId: initialTabIds[0] || '',
      isPinned: false,
      createdAt: Date.now(),
      lastUsedAt: Date.now(),
    };
    setScreens((prev) => [...prev, newScreen]);
    setActiveScreenId(newId);
    setAppActiveScreenId(newId);
    if (newScreen.homeWorkbench) {
      setActiveWorkbench(newScreen.homeWorkbench);
    }
  }, [screens.length, activeWorkspace?.metadata?.project_id, activeWorkspace?.id, currentProject, currentWb, initScreenTabs, setAppActiveScreenId, setActiveWorkbench]);

  const closeScreen = useCallback((screenId: string) => {
    if (screensForWorkbench(screens, currentWb).length <= 1) return;

    if (activeScreenId === screenId) {
      const remaining = screens.filter((s) => s.id !== screenId);
      const nextScreen = screensForWorkbench(remaining, currentWb)
        .sort((a, b) => (b.lastUsedAt || 0) - (a.lastUsedAt || 0))[0];
      if (nextScreen) {
        setActiveScreenId(nextScreen.id);
        setAppActiveScreenId(nextScreen.id);
        const nextTabs = nextScreen.tabs && nextScreen.tabs.length > 0
          ? nextScreen.tabs
          : defaultTabsForScreen(nextScreen);
        initScreenTabs(nextScreen.id, nextTabs, nextScreen.activeTabId || nextTabs[0]?.id || '');
      }
    }

    setScreens((prev) => {
      if (screensForWorkbench(prev, currentWb).length <= 1) return prev;
      return prev.filter((s) => s.id !== screenId);
    });
  }, [screens, activeScreenId, currentWb, setAppActiveScreenId, initScreenTabs]);

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
    (worktree: WorktreeDescriptor) => {
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
        updatedScreen.tabs = defaultTabsForScreen(updatedScreen);
        updatedScreen.activeTabId = updatedScreen.tabs[0]?.id || '';
        initScreenTabs(updatedScreen.id, updatedScreen.tabs, updatedScreen.activeTabId);
        setScreens([updatedScreen]);
        setActiveWorkspaceId(worktree.id);
        return;
      }

      const newId = `screen-wt-${worktree.id}`;
      const initialTabsList = buildWorktreeScreenTabs(worktree, screenLabel);
      initScreenTabs(newId, initialTabsList, initialTabsList[0]?.id || '');

      const newScreen = createWorktreeScreen(
        worktree,
        activeWorkspace?.metadata?.project_id || currentProject || 'default',
        initialTabsList
      );

      setScreens((prev) => [...prev, newScreen]);
      setActiveScreenId(newId);
      setAppActiveScreenId(newId);
      setActiveWorkspaceId(worktree.id);
    },
    [
      screens,
      selectScreen,
      setActiveWorkspaceId,
      initScreenTabs,
      activeWorkspace?.metadata?.project_id,
      currentProject,
      setAppActiveScreenId,
    ]
  );

  const revealTaskScreen = useCallback(
    (task: TaskDescriptor) => {
      const existing = screens.find(
        (s) => s.anchor?.kind === 'task' && s.anchor.id === task.id
      );

      if (existing) {
        selectScreen(existing.id);
        return;
      }

      const newId = `screen-task-${task.id}`;
      const screenLabel = `Task: ${task.title}`;
      const initialTabsList = buildTaskScreenTabs(task, screenLabel);
      initScreenTabs(newId, initialTabsList, initialTabsList[0]?.id || '');

      const newScreen = createTaskScreen(task, currentProject || 'default', initialTabsList);

      setScreens((prev) => [...prev, newScreen]);
      setActiveScreenId(newId);
      setAppActiveScreenId(newId);
      if (newScreen.homeWorkbench) {
        setActiveWorkbench(newScreen.homeWorkbench);
      }
    },
    [screens, selectScreen, initScreenTabs, currentProject, setAppActiveScreenId, setActiveWorkbench]
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
      revealWorktreeScreen,
      revealTaskScreen,
      promoteEphemeralScreen,
      registerCanvasActions,
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
