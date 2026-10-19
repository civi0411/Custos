import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  PanelLeft,
  PanelRight,
  Search,
  Minus,
  Square,
  Copy,
  X,
  ChevronLeft,
  ChevronRight,
  Code2,
  FlaskConical,
  Bot,
  Globe,
  Smartphone,
} from 'lucide-react';
import { isMacOS, formatKeyCombo } from '@/lib/utils';
import { useAppContext } from '@/context/AppContext';
import { useScreenContext } from '@/context/ScreenContext';
import { ActionIcon } from '@/components/common/ActionIcon';
import { ScreenSwitcherBar } from '@/components/workbenches/ScreenSwitcherBar';
import { daemonClient } from '@/api/daemon_client';

interface SessionMetrics {
  session_id?: string;
  tokens_in: number;
  tokens_out: number;
  total_tokens: number;
  estimated_cost_usd: number;
  active_runs: number;
  pending_approvals: number;
}

export const AppTopBar: React.FC = () => {
  const isMac = isMacOS();

  const {
    isSidebarCollapsed,
    toggleSidebar,
    isInspectorOpen,
    toggleInspector,
    activeSession,
    activeSessionId,
    activeWorkbench,
    setActiveWorkbench,
    currentProject: _currentProject,
    setIsOpenProjectOpen: _setIsOpenProjectOpen,
  } = useAppContext();

  const {
    screens,
    activeScreenId,
    selectScreen,
    createScreen,
    closeScreen,
    togglePinScreen,
    renameScreen,
  } = useScreenContext();

  // Screen history navigation (Back / Forward)
  const [navHistory, setNavHistory] = useState<string[]>([]);
  const [navIndex, setNavIndex] = useState<number>(-1);
  const isNavigatingRef = useRef(false);

  useEffect(() => {
    if (!activeScreenId) return;
    if (isNavigatingRef.current) {
      isNavigatingRef.current = false;
      return;
    }
    setNavHistory((prev) => {
      const current = prev.slice(0, navIndex + 1);
      if (current[current.length - 1] === activeScreenId) return prev;
      const next = [...current, activeScreenId];
      setNavIndex(next.length - 1);
      return next;
    });
  }, [activeScreenId, navIndex]);

  const canGoBack = navIndex > 0;
  const canGoForward = navIndex >= 0 && navIndex < navHistory.length - 1;

  const handleGoBack = useCallback(() => {
    if (!canGoBack) return;
    isNavigatingRef.current = true;
    const targetScreenId = navHistory[navIndex - 1];
    setNavIndex((i) => i - 1);
    selectScreen(targetScreenId);
  }, [canGoBack, navHistory, navIndex, selectScreen]);

  const handleGoForward = useCallback(() => {
    if (!canGoForward) return;
    isNavigatingRef.current = true;
    const targetScreenId = navHistory[navIndex + 1];
    setNavIndex((i) => i + 1);
    selectScreen(targetScreenId);
  }, [canGoForward, navHistory, navIndex, selectScreen]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === '[') {
        e.preventDefault();
        handleGoBack();
      } else if ((e.metaKey || e.ctrlKey) && e.key === ']') {
        e.preventDefault();
        handleGoForward();
      } else if ((e.metaKey || e.ctrlKey) && e.altKey && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent('custos:open-tab', { detail: { id: 'browser' } }));
      } else if ((e.metaKey || e.ctrlKey) && e.altKey && e.key.toLowerCase() === 'p') {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent('custos:open-tab', { detail: { id: 'emulator' } }));
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleGoBack, handleGoForward]);

  const workbenchModes: {
    id: 'engineering' | 'research' | 'copilot';
    label: string;
    icon: React.ElementType;
    shortcut: string;
  }[] = [
      { id: 'engineering', label: 'Engineering', icon: Code2, shortcut: '⌥1' },
      { id: 'research', label: 'Research', icon: FlaskConical, shortcut: '⌥2' },
      { id: 'copilot', label: 'Copilot', icon: Bot, shortcut: '⌥3' },
    ];

  const [isWindowMaximized, setIsWindowMaximized] = useState(false);

  // Inspector dynamic width synchronization
  const [inspectorWidth, setInspectorWidth] = useState<number>(() => {
    try {
      const saved = Number(localStorage.getItem('custos.right-sidebar-width'));
      return saved >= 260 && saved <= 760 ? saved : 320;
    } catch {
      return 320;
    }
  });

  useEffect(() => {
    const handleResize = (e: Event) => {
      const custom = e as CustomEvent<number>;
      if (typeof custom.detail === 'number') {
        setInspectorWidth(custom.detail);
      }
    };
    window.addEventListener('custos:right-sidebar-resized', handleResize);
    return () => window.removeEventListener('custos:right-sidebar-resized', handleResize);
  }, []);

  // Real live data from daemon for active runs
  const [metrics, setMetrics] = useState<SessionMetrics | null>(null);

  const fetchLiveData = useCallback(async () => {
    try {
      const isOnline = await daemonClient.checkHealth();
      if (isOnline) {
        const metricsData = await daemonClient
          .getSessionMetrics(activeSessionId || activeSession?.id)
          .catch(() => null);
        if (metricsData) setMetrics(metricsData);
      }
    } catch {
      // Keep last known data
    }
  }, [activeSessionId, activeSession?.id]);

  useEffect(() => {
    fetchLiveData();
    const pollInterval = metrics && metrics.active_runs > 0 ? 5000 : 15000;
    const interval = setInterval(fetchLiveData, pollInterval);
    return () => clearInterval(interval);
  }, [fetchLiveData, metrics?.active_runs]);

  useEffect(() => {
    if (isMac) return;

    let unlisten: (() => void) | undefined;
    const initTauriWindow = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const appWindow = getCurrentWindow();
        setIsWindowMaximized(await appWindow.isMaximized());
        unlisten = await appWindow.onResized(async () => {
          setIsWindowMaximized(await appWindow.isMaximized());
        });
      } catch { }
    };

    void initTauriWindow();
    return () => {
      unlisten?.();
    };
  }, [isMac]);

  const handleMinimize = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().minimize();
    } catch { }
  };

  const handleToggleMaximize = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().toggleMaximize();
    } catch { }
  };

  const handleClose = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } catch { }
  };

  const handleOpenGlobalSearch = () => {
    window.dispatchEvent(new CustomEvent('custos:open-command-palette'));
  };

  const activeRuns = metrics?.active_runs ?? 0;


  return (
    <header
      className="relative flex h-10 w-full shrink-0 items-center justify-between bg-[#18181b] text-white/70 select-none z-50 border-b border-white/6"
      aria-label="Custos Titlebar"
    >
      {/* ── LEFT COLUMN: Window spacer (Traffic lights/Brand) + Sidebar Toggle + Workbench Modes ── */}
      <div
        data-tauri-drag-region
        className={`flex items-center gap-1.5 shrink-0 h-full z-30 transition-[width] duration-200 ease-out pl-2.5 pr-2 ${!isSidebarCollapsed ? 'border-r border-white/6' : ''
          }`}
        style={{
          width: isSidebarCollapsed ? (isMac ? 176 : 148) : 304,
        }}
      >
        {/* macOS Traffic Lights Reserved Space or Windows Brand Name */}
        {isMac ? (
          <div
            data-tauri-drag-region
            className="w-18 shrink-0 h-full cursor-default select-none"
            title="Window Drag Region"
          />
        ) : (
          <div className="flex items-center gap-1.5 pl-1 pr-2 shrink-0 select-none" data-tauri-drag-region>
            <span className="font-mono text-[11px] font-bold text-workbench-accent tracking-wider">
              Custos
            </span>
          </div>
        )}

        {/* Sidebar Toggle Button */}
        <ActionIcon
          icon={PanelLeft}
          tooltip={`Toggle Sidebar (${formatKeyCombo({ ctrlOrCmd: true, key: 'B' })})`}
          ariaLabel="Toggle Sidebar"
          active={!isSidebarCollapsed}
          size="sm"
          onClick={(e) => {
            e.stopPropagation();
            toggleSidebar();
          }}
        />

        {/* Navigation History: Back & Forward */}
        <div className="flex items-center gap-0.5 shrink-0">
          <ActionIcon
            icon={ChevronLeft}
            tooltip={`Back (${formatKeyCombo({ ctrlOrCmd: true, key: '[' })})`}
            ariaLabel="Back"
            size="sm"
            disabled={!canGoBack}
            onClick={(e) => {
              e.stopPropagation();
              handleGoBack();
            }}
          />
          <ActionIcon
            icon={ChevronRight}
            tooltip={`Forward (${formatKeyCombo({ ctrlOrCmd: true, key: ']' })})`}
            ariaLabel="Forward"
            size="sm"
            disabled={!canGoForward}
            onClick={(e) => {
              e.stopPropagation();
              handleGoForward();
            }}
          />
        </div>

        {/* Compact Workbench Modes Switcher (Icon-only, sleek 3-button segmented control) */}
        <div
          className="flex items-center gap-0.5 rounded-lg border border-white/10 bg-black/40 p-0.5 shrink-0 shadow-xs"
          role="tablist"
          aria-label="Workbench Lens"
        >
          {workbenchModes.map((m) => {
            const Icon = m.icon;
            const isActive = activeWorkbench === m.id;
            return (
              <button
                key={m.id}
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setActiveWorkbench(m.id);
                  const targetScreen = screens
                    .filter((s) => s.homeWorkbench === m.id || (!s.homeWorkbench && m.id === 'engineering'))
                    .sort((a, b) => (b.lastUsedAt || 0) - (a.lastUsedAt || 0))[0];
                  if (targetScreen) {
                    selectScreen(targetScreen.id);
                  }
                  window.dispatchEvent(new CustomEvent('custos:switch-workbench', { detail: m.id }));
                }}
                className={`flex items-center justify-center size-6 rounded-md transition cursor-pointer select-none ${isActive
                    ? 'bg-white/18 text-white shadow-xs'
                    : 'text-white/45 hover:text-white/85 hover:bg-white/8'
                  }`}
                title={`${m.label} Workbench (${m.shortcut})`}
                aria-label={`${m.label} Workbench`}
              >
                <Icon className={`size-3.5 shrink-0 ${isActive ? 'text-workbench-accent' : ''}`} />
              </button>
            );
          })}
        </div>

        {/* Empty draggable titlebar space filling up to the 304px workpane boundary */}
        <div data-tauri-drag-region className="flex-1 h-full min-w-0" />
      </div>

      {/* ── CENTER: Screen Tabs Bar (Strictly aligned with central workpane axis between the 2 side panels) ── */}
      <div
        data-tauri-drag-region
        className="flex-1 min-w-0 h-full flex items-center justify-start overflow-hidden px-2 z-20 cursor-default"
      >
        <ScreenSwitcherBar
          screens={screens}
          activeScreenId={activeScreenId}
          onSelectScreen={selectScreen}
          onCreateScreen={createScreen}
          onCloseScreen={closeScreen}
          onTogglePinScreen={togglePinScreen}
          onRenameScreen={renameScreen}
          currentWorkbench={activeWorkbench || 'engineering'}
          className="flex items-center justify-start gap-1 max-w-full overflow-x-auto no-scrollbar select-none text-xs"
        />

        {/* Draggable titlebar area between the screen tabs and right controls */}
        <div data-tauri-drag-region className="flex-1 h-full min-w-6 flex items-center pl-2">
          {activeSession?.title && (
            <span
              data-tauri-drag-region
              className="text-[11px] text-white/30 truncate max-w-[200px] select-none cursor-default"
              title={`Active Session: ${activeSession.title}`}
            >
              · {activeSession.title}
            </span>
          )}
        </div>
      </div>

      {/* ── RIGHT COLUMN: Git Branch + Quick Open + Attention Indicator + AI Chat + Inspector Toggle + Window Controls ── */}
      <div
        data-tauri-drag-region
        className={`flex items-center justify-end gap-1.5 shrink-0 h-full transition-[width,min-width] duration-200 ease-out pl-2 pr-2.5 ml-auto ${isInspectorOpen ? 'border-l border-white/6' : ''
          }`}
        style={{
          minWidth: isInspectorOpen ? inspectorWidth : undefined,
        }}
      >
        {/* Quick Preview buttons in Engineering mode */}
        {activeWorkbench === 'engineering' && (
          <div className="flex items-center gap-1 border-r border-white/8 pr-1.5 mr-0.5">
            <button
              type="button"
              onClick={() => window.dispatchEvent(new CustomEvent('custos:open-tab', { detail: { id: 'browser' } }))}
              className="flex items-center gap-1 h-6.5 px-2 rounded-md border border-white/10 bg-white/5 text-white/70 hover:bg-white/10 hover:text-white transition text-xs cursor-pointer select-none"
              title="Browser Preview (⌥⌘B)"
              aria-label="Browser Preview"
            >
              <Globe className="size-3.5 text-blue-400" />
              <span className="text-[11px] font-medium hidden md:inline">Browser</span>
            </button>
            <button
              type="button"
              onClick={() => window.dispatchEvent(new CustomEvent('custos:open-tab', { detail: { id: 'emulator' } }))}
              className="flex items-center gap-1 h-6.5 px-2 rounded-md border border-white/10 bg-white/5 text-white/70 hover:bg-white/10 hover:text-white transition text-xs cursor-pointer select-none"
              title="Phone Simulator (⌥⌘P)"
              aria-label="Phone Simulator"
            >
              <Smartphone className="size-3.5 text-purple-400" />
              <span className="text-[11px] font-medium hidden md:inline">Phone</span>
            </button>
          </div>
        )}

        {/* Quick Open Search Button (Compact & clean) */}
        <button
          type="button"
          onClick={handleOpenGlobalSearch}
          className="flex items-center gap-1.5 h-6.5 px-2 rounded-md border border-white/10 bg-white/5 text-white/50 hover:bg-white/10 hover:text-white/80 transition text-xs cursor-pointer select-none"
          title="Quick Open (⌘K)"
          aria-label="Quick Open"
        >
          <Search className="size-3.5" />
          <kbd className="font-mono text-[9px] text-white/40 bg-white/10 px-1 py-0.2 rounded">
            {formatKeyCombo({ ctrlOrCmd: true, key: 'K' })}
          </kbd>
        </button>

        {/* Attention Indicator (Active agent runs) */}
        {activeRuns > 0 && (
          <div
            className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-white/6 border border-white/10 text-white/70 text-[10.5px] font-mono select-none shrink-0"
            title={`${activeRuns} active agent run(s)`}
          >
            <span className="size-1.5 rounded-full bg-white/60 animate-pulse" />
            <span>{activeRuns}</span>
          </div>
        )}


        {/* Context Inspector Toggle (PanelRight) */}
        <ActionIcon
          icon={PanelRight}
          tooltip={`Context Inspector (${formatKeyCombo({ alt: true, key: 'I' })})`}
          ariaLabel="Toggle Context Inspector"
          active={isInspectorOpen}
          size="sm"
          onClick={(e) => {
            e.stopPropagation();
            toggleInspector();
          }}
        />

        {/* Windows / Linux Native Window Controls */}
        {!isMac && (
          <div className="flex items-center ml-1 border-l border-white/10 pl-1">
            <button
              type="button"
              onClick={handleMinimize}
              className="flex size-7 items-center justify-center text-white/60 hover:bg-white/10 hover:text-white transition-colors cursor-pointer"
              title="Minimize"
              aria-label="Minimize Window"
            >
              <Minus className="size-3" />
            </button>
            <button
              type="button"
              onClick={handleToggleMaximize}
              className="flex size-7 items-center justify-center text-white/60 hover:bg-white/10 hover:text-white transition-colors cursor-pointer"
              title={isWindowMaximized ? 'Restore' : 'Maximize'}
              aria-label={isWindowMaximized ? 'Restore Window' : 'Maximize Window'}
            >
              {isWindowMaximized ? <Copy className="size-3" /> : <Square className="size-3" />}
            </button>
            <button
              type="button"
              onClick={handleClose}
              className="flex size-7 items-center justify-center text-white/60 hover:bg-red-500 hover:text-white transition-colors cursor-pointer"
              title="Close"
              aria-label="Close Window"
            >
              <X className="size-3" />
            </button>
          </div>
        )}
      </div>
    </header>
  );
};

