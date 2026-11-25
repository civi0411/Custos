import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import type { OrcaTab, ResourceTabsState } from '@/components/workbenches/OrcaTabbedContainer';

export type ActiveWorkbench = 'copilot' | 'engineering' | 'research' | 'quant';

export const getDefaultTabForMode = (_mode: ActiveWorkbench): { tabs: OrcaTab[]; activeTabId: string } => ({
  tabs: [{ id: 'conversation', type: 'conversation', title: 'Conversation', url: 'custos://chat' }],
  activeTabId: 'conversation',
});

export interface CanvasLayoutDeps {
  currentProject: string;
  activeWorkbench: ActiveWorkbench;
  activeWorkspaceId: string;
  activeSessionId: string;
  isSidebarCollapsed: boolean;
}

export function useCanvasLayoutSlice({
  currentProject,
  activeWorkbench,
  activeWorkspaceId,
  activeSessionId,
  isSidebarCollapsed,
}: CanvasLayoutDeps) {
  const [activeScreenId, setActiveScreenId] = useState<string>('screen-default');
  const tabScope = useMemo(
    () => JSON.stringify([currentProject, activeScreenId]),
    [currentProject, activeScreenId]
  );
  const [canvasViews, setCanvasViews] = useState<Record<string, { open: boolean; expanded: boolean }>>(() => {
    try {
      const saved = JSON.parse(localStorage.getItem('custos.canvas-views.v1') || '{}');
      return Object.fromEntries(Object.entries(saved).filter((entry): entry is [string, { open: boolean; expanded: boolean }] => {
        const value = entry[1] as { open?: unknown; expanded?: unknown } | null;
        return !!value && typeof value.open === 'boolean' && typeof value.expanded === 'boolean';
      }));
    } catch { return {}; }
  });
  const isWebTabOpen = canvasViews[tabScope]?.open ?? true;
  const isWebTabExpanded = canvasViews[tabScope]?.expanded ?? false;
  const setIsWebTabOpen = useCallback((update: React.SetStateAction<boolean>) => {
    setCanvasViews(previous => {
      const view = previous[tabScope] || { open: false, expanded: false };
      const open = typeof update === 'function' ? update(view.open) : update;
      return { ...previous, [tabScope]: { open, expanded: open && view.expanded } };
    });
  }, [tabScope]);
  const setIsWebTabExpanded = useCallback((update: React.SetStateAction<boolean>) => {
    setCanvasViews(previous => {
      const view = previous[tabScope] || { open: false, expanded: false };
      return { ...previous, [tabScope]: { ...view, expanded: typeof update === 'function' ? update(view.expanded) : update } };
    });
  }, [tabScope]);
  const toggleWebTab = useCallback(() => setIsWebTabOpen(previous => !previous), [setIsWebTabOpen]);
  const toggleExpandWebTab = useCallback(() => setIsWebTabExpanded(previous => !previous), [setIsWebTabExpanded]);

  const [isInspectorOpen, setIsInspectorOpen] = useState<boolean>(() => {
    try {
      const saved = localStorage.getItem('custos.inspector-open');
      if (saved !== null) return saved === 'true';
      return typeof window !== 'undefined' && window.innerWidth >= 1024;
    } catch {
      return true;
    }
  });

  const setIsInspectorOpenPersisted = useCallback((update: React.SetStateAction<boolean>) => {
    setIsInspectorOpen((prev) => {
      const next = typeof update === 'function' ? update(prev) : update;
      try {
        localStorage.setItem('custos.inspector-open', String(next));
      } catch {}
      return next;
    });
  }, []);

  const toggleInspector = useCallback(() => setIsInspectorOpenPersisted((previous) => !previous), [setIsInspectorOpenPersisted]);
  const [isResourcePickerOpen, setIsResourcePickerOpen] = useState<boolean>(false);

  const [tabStates, setTabStates] = useState<Record<string, { tabs: OrcaTab[]; activeTabId: string }>>(() => {
    try {
      const saved = JSON.parse(localStorage.getItem('custos.resource-tabs.v1') || '{}');
      if (saved.version !== 1 || !saved.scopes || typeof saved.scopes !== 'object') return {};
      return Object.fromEntries(Object.entries(saved.scopes).filter((entry): entry is [string, { tabs: OrcaTab[]; activeTabId: string }] => {
        const value = entry[1] as { tabs?: OrcaTab[]; activeTabId?: string } | null;
        return !!value && Array.isArray(value.tabs) && value.tabs.length <= 100 && typeof value.activeTabId === 'string'
          && new Set(value.tabs.map(tab => tab?.id)).size === value.tabs.length
          && value.tabs.every(tab => tab && typeof tab.id === 'string' && typeof tab.title === 'string' && typeof tab.url === 'string' && typeof tab.type === 'string' && ['workspaceId', 'sessionId', 'taskId', 'agentId', 'runId', 'artifactId', 'sourceId', 'revision'].every(key => tab[key as keyof OrcaTab] === undefined || typeof tab[key as keyof OrcaTab] === 'string'));
      }));
    } catch { return {}; }
  });

  const currentTabState = useMemo(() => {
    const state = tabStates[tabScope] ?? getDefaultTabForMode(activeWorkbench);
    return {
      ...state,
      tabs: state.tabs.map(tab => ({
        ...tab,
        workspaceId: tab.workspaceId || activeWorkspaceId || undefined,
        sessionId: tab.sessionId || activeSessionId || undefined,
      })),
    };
  }, [tabStates, tabScope, activeWorkbench, activeWorkspaceId, activeSessionId]);

  const setTabs = useCallback((update: React.SetStateAction<OrcaTab[]>) => {
    setTabStates((previous) => {
      const current = previous[tabScope] ?? getDefaultTabForMode(activeWorkbench);
      const tabs = typeof update === 'function' ? update(current.tabs) : update;
      return { ...previous, [tabScope]: { ...current, tabs } };
    });
  }, [tabScope, activeWorkbench]);

  const setActiveTabId = useCallback((update: React.SetStateAction<string>) => {
    setTabStates((previous) => {
      const current = previous[tabScope] ?? getDefaultTabForMode(activeWorkbench);
      const activeTabId = typeof update === 'function' ? update(current.activeTabId) : update;
      return { ...previous, [tabScope]: { ...current, activeTabId } };
    });
  }, [tabScope, activeWorkbench]);

  const initScreenTabs = useCallback((screenId: string, initialTabs: OrcaTab[], initialActiveTabId?: string) => {
    const scope = JSON.stringify([currentProject, screenId]);
    setTabStates((prev) => ({
      ...prev,
      [scope]: {
        tabs: initialTabs,
        activeTabId: initialActiveTabId || initialTabs[0]?.id || '',
      },
    }));
  }, [currentProject]);

  const [splitPercent, setSplitPercent] = useState<number>(() => {
    try {
      const saved = localStorage.getItem('custos_split_pct');
      if (saved) {
        const val = parseFloat(saved);
        if (!isNaN(val) && val >= 20 && val <= 80) return val;
      }
    } catch {}
    return 42;
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_split_pct', splitPercent.toString());
    } catch {}
  }, [splitPercent]);

  const [splitTabId, setSplitTabId] = useState<string | null>(null);
  const [splitRatio, setSplitRatio] = useState<number>(50);
  const [column2Width, setColumn2Width] = useState<number>(() => {
    try {
      const saved = localStorage.getItem('custos_column2_width');
      if (saved) {
        const val = parseInt(saved, 10);
        if (!isNaN(val) && val >= 260 && val <= 1600) return val;
      }
    } catch {}
    return Math.max(360, Math.min(1100, Math.round((window.innerWidth - 296) * 0.58)));
  });

  const [isResizingColumn, setIsResizingColumn] = useState<boolean>(false);

  useEffect(() => {
    try {
      localStorage.setItem('custos_column2_width', column2Width.toString());
    } catch {}
  }, [column2Width]);

  const dragStartCollapsedRef = useRef<boolean>(false);

  const handleStartResizeColumn2 = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragStartCollapsedRef.current = !isWebTabOpen;
    setIsResizingColumn(true);
  }, [isWebTabOpen]);

  const handleResetColumn2Width = useCallback(() => {
    const def = Math.max(360, Math.min(1100, Math.round((window.innerWidth - 296) * 0.58)));
    setColumn2Width(def);
    setIsWebTabOpen(true);
  }, [setIsWebTabOpen]);

  // Global mousemove and mouseup listeners when isResizingColumn is true
  useEffect(() => {
    if (!isResizingColumn) return;

    const handleMouseMove = (e: MouseEvent) => {
      const rawWidth = Math.round(window.innerWidth - e.clientX);
      const minCol2 = 240;
      const leftReserved = (isSidebarCollapsed ? 44 : 44 + 256) + 320;
      const maxCol2 = Math.max(minCol2, window.innerWidth - leftReserved);

      if (dragStartCollapsedRef.current) {
        if (rawWidth > 40) {
          setIsWebTabOpen(true);
          const clamped = Math.min(Math.max(rawWidth, minCol2), maxCol2);
          setColumn2Width(clamped);
          setSplitPercent(Math.round(((window.innerWidth - clamped) / window.innerWidth) * 100));
        }
      } else {
        if (rawWidth < 180) {
          setIsWebTabOpen(false);
        } else {
          setIsWebTabOpen(true);
          const clamped = Math.min(Math.max(rawWidth, 180), maxCol2);
          setColumn2Width(clamped);
          setSplitPercent(Math.round(((window.innerWidth - clamped) / window.innerWidth) * 100));
        }
      }
    };

    const handleMouseUp = (e: MouseEvent) => {
      const rawWidth = Math.round(window.innerWidth - e.clientX);
      if (rawWidth < 200) {
        setIsWebTabOpen(false);
        const def = Math.max(360, Math.min(1100, Math.round((window.innerWidth - 296) * 0.58)));
        setColumn2Width(def);
      } else {
        setIsWebTabOpen(true);
        const minCol2 = 240;
        const leftReserved = (isSidebarCollapsed ? 44 : 44 + 256) + 320;
        const maxCol2 = Math.max(minCol2, window.innerWidth - leftReserved);
        const clamped = Math.min(Math.max(rawWidth, minCol2), maxCol2);
        setColumn2Width(clamped);
      }
      setIsResizingColumn(false);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizingColumn, isSidebarCollapsed, setIsWebTabOpen]);

  useEffect(() => {
    const handleWindowResize = () => {
      const minCol2 = 260;
      const leftReserved = (isSidebarCollapsed ? 44 : 44 + 256) + 320;
      const maxCol2 = Math.max(minCol2, window.innerWidth - leftReserved);
      setColumn2Width((prev) => Math.min(Math.max(prev, minCol2), maxCol2));
    };

    window.addEventListener('resize', handleWindowResize);
    return () => window.removeEventListener('resize', handleWindowResize);
  }, [isSidebarCollapsed]);

  const toggleSplitTab = useCallback((tabId: string) => {
    setSplitTabId((prev) => {
      if (prev === tabId) return null;
      if (tabId === currentTabState.activeTabId) {
        if (prev) return null;
        const other = currentTabState.tabs.find((t) => t.id !== tabId);
        return other ? other.id : null;
      }
      return tabId;
    });
  }, [currentTabState.activeTabId, currentTabState.tabs]);

  const closeSplitTab = useCallback(() => {
    setSplitTabId(null);
  }, []);

  const resourceTabs: ResourceTabsState = useMemo(() => ({
    ...currentTabState,
    scopeKey: tabScope,
    setTabs,
    setActiveTabId,
    splitTabId,
    setSplitTabId,
    splitRatio,
    setSplitRatio,
    toggleSplitTab,
    closeSplitTab,
  }), [currentTabState, tabScope, setTabs, setActiveTabId, splitTabId, splitRatio, toggleSplitTab, closeSplitTab]);

  const handleCloseResourceTab = useCallback((e: React.MouseEvent, tabId: string) => {
    e.stopPropagation();
    if (splitTabId === tabId) {
      setSplitTabId(null);
    }
    const nextTabs = resourceTabs.tabs.filter((t) => t.id !== tabId);
    resourceTabs.setTabs(nextTabs);
    if (nextTabs.length === 0) {
      setIsWebTabOpen(false);
      return;
    }
    if (resourceTabs.activeTabId === tabId) {
      const closedIndex = resourceTabs.tabs.findIndex((t) => t.id === tabId);
      const nextActive = nextTabs[Math.max(0, closedIndex - 1)];
      resourceTabs.setActiveTabId(nextActive.id);
    }
  }, [resourceTabs, splitTabId, setIsWebTabOpen]);

  return {
    activeScreenId,
    setActiveScreenId,
    tabScope,
    canvasViews,
    setCanvasViews,
    isWebTabOpen,
    setIsWebTabOpen,
    toggleWebTab,
    isWebTabExpanded,
    setIsWebTabExpanded,
    toggleExpandWebTab,
    isInspectorOpen,
    setIsInspectorOpen: setIsInspectorOpenPersisted,
    toggleInspector,
    isResourcePickerOpen,
    setIsResourcePickerOpen,
    tabStates,
    setTabStates,
    currentTabState,
    resourceTabs,
    handleCloseResourceTab,
    initScreenTabs,
    splitPercent,
    setSplitPercent,
    splitTabId,
    setSplitTabId,
    splitRatio,
    setSplitRatio,
    toggleSplitTab,
    closeSplitTab,
    column2Width,
    setColumn2Width,
    isResizingColumn,
    handleStartResizeColumn2,
    handleResetColumn2Width,
  };
}
