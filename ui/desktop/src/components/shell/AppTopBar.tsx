import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  ArrowLeft,
  ArrowRight,
  PanelLeft,
  PanelRight,
  MessageSquare,
  Code2,
  FlaskConical,
  GitBranch,
  Minus,
  Square,
  Copy,
  X,
  Plus,
  Maximize2,
  Minimize2,
  Columns2,
  FolderTree,
  FileDiff,
  Activity,
  Terminal as TerminalIcon,
  ShieldCheck,
  BookOpen,
  NotebookTabs,
  Layers,
  FileCode,
  FileText,
  Send,
  Users,
  Calendar,
  Globe,
  LayoutGrid,
} from 'lucide-react';
import WorktreeIcon from '~icons/codicon/worktree';
import { isMacOS, formatKeyCombo } from '@/lib/utils';
import { useAppContext, type ActiveWorkbench } from '@/context/AppContext';
import { getTabType, type OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export const AppTopBar: React.FC = () => {
  const navigate = useNavigate();
  const isMac = isMacOS();

  const {
    activeWorkbench,
    setActiveWorkbench,
    isSidebarCollapsed,
    toggleSidebar,
    isWebTabOpen,
    toggleWebTab,
    isWebTabExpanded,
    toggleExpandWebTab,
    resourceTabs,
    handleCloseResourceTab,
    setIsResourcePickerOpen,
    splitPercent,
    column2Width,
    splitTabId,
    setSplitTabId,
    toggleSplitTab,
    closeSplitTab,
    activeSession,
    activeWorkspace,
    activeDetection,
  } = useAppContext();

  const [isWindowMaximized, setIsWindowMaximized] = useState(false);

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
      } catch {}
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
    } catch {}
  };

  const handleToggleMaximize = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().toggleMaximize();
    } catch {}
  };

  const handleClose = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } catch {}
  };

  const handleBack = (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      navigate(-1);
    } catch {}
  };

  const handleForward = (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      navigate(1);
    } catch {}
  };

  const workbenchModes: {
    id: ActiveWorkbench;
    label: string;
    icon: React.ElementType;
    shortcut: string;
  }[] = [
    { id: 'copilot', label: 'Copilot', icon: MessageSquare, shortcut: '⌥1' },
    { id: 'engineering', label: 'Engineering', icon: Code2, shortcut: '⌥2' },
    { id: 'research', label: 'Research', icon: FlaskConical, shortcut: '⌥3' },
  ];

  const getTabIcon = (id: OrcaTabId, isActive: boolean) => {
    const cls = `w-3 h-3 ${isActive ? 'text-workbench-accent' : 'text-white/50 group-hover:text-white/80 transition-colors'}`;
    switch (id) {
      case 'tools': return <LayoutGrid className={cls} />;
      case 'files': return <FolderTree className={cls} />;
      case 'changes': return <FileDiff className={cls} />;
      case 'activity': return <Activity className={cls} />;
      case 'terminal': return <TerminalIcon className={cls} />;
      case 'worktrees': return <WorktreeIcon className={cls} />;
      case 'evidence': return <ShieldCheck className={cls} />;
      case 'literature': return <BookOpen className={cls} />;
      case 'findings': return <ShieldCheck className={cls} />;
      case 'claims': return <ShieldCheck className={cls} />;
      case 'lab': return <FlaskConical className={cls} />;
      case 'experiments': return <NotebookTabs className={cls} />;
      case 'runs': return <FlaskConical className={cls} />;
      case 'knowledge': return <FlaskConical className={cls} />;
      case 'synthesis': return <Layers className={cls} />;
      case 'artifacts': return <FileCode className={cls} />;
      case 'drafts': return <FileText className={cls} />;
      case 'outbox': return <Send className={cls} />;
      case 'contacts': return <Users className={cls} />;
      case 'calendar': return <Calendar className={cls} />;
      case 'notes': return <FileCode className={cls} />;
      default: return <Globe className={cls} />;
    }
  };

  const currentBranch =
    activeWorkspace?.active_branch ||
    (activeWorkspace?.kind?.type === 'git' ? activeWorkspace.kind.branch : null) ||
    activeDetection?.branch;

  const isDirty = Boolean(
    activeWorkspace
      ? activeWorkspace.dirty_manifest?.is_dirty ?? activeWorkspace.dirtyManifest?.is_dirty
      : activeDetection?.is_dirty
  );

  const dirtyCount = activeWorkspace
    ? activeWorkspace.dirty_manifest?.modified_files?.length ??
      activeWorkspace.dirtyManifest?.modified_files?.length ??
      0
    : activeDetection?.dirty_count ?? 0;

  return (
    <header
      className="relative flex h-9.5 w-full shrink-0 items-center justify-between bg-[#212124] border-b border-white/10 text-white/70 select-none z-50 px-2"
      aria-label="Unified Sovereign Titlebar"
    >
      {/* ── LEFT: macOS Traffic Lights + Nav + Sidebar Toggle + 3 Lenses ── */}
      <div className="flex items-center gap-1 shrink-0">
        {/* macOS Traffic Lights Reserved Space */}
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

        {/* Back / Forward History */}
        <div className="flex items-center gap-0.5">
          <button
            type="button"
            onClick={handleBack}
            aria-label="Back"
            title="Back (⌘[)"
            className="flex size-6 items-center justify-center rounded-md text-white/50 transition-colors hover:bg-white/10 hover:text-white active:scale-95 cursor-pointer"
          >
            <ArrowLeft className="size-3.5" strokeWidth={2} />
          </button>
          <button
            type="button"
            onClick={handleForward}
            aria-label="Forward"
            title="Forward (⌘])"
            className="flex size-6 items-center justify-center rounded-md text-white/50 transition-colors hover:bg-white/10 hover:text-white active:scale-95 cursor-pointer"
          >
            <ArrowRight className="size-3.5" strokeWidth={2} />
          </button>
        </div>

        {/* Vertical divider */}
        <div className="h-3.5 w-px bg-white/10 mx-1 shrink-0" />

        {/* Sidebar Toggle Button */}
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            toggleSidebar();
          }}
          title={`Toggle Sidebar (${formatKeyCombo({ ctrlOrCmd: true, key: 'B' })})`}
          className={`flex size-6 items-center justify-center rounded-md transition-colors cursor-pointer ${
            !isSidebarCollapsed
              ? 'text-white/80 bg-white/10'
              : 'text-white/40 hover:bg-white/5 hover:text-white/70'
          }`}
        >
          <PanelLeft className="size-3.5" />
        </button>

        {/* ── 3 Workbench Lenses Segmented Switcher (Icon-only: Copilot | Engineering | Research) ── */}
        <div
          className="flex items-center gap-0.5 rounded-lg border border-white/10 bg-black/25 p-0.5 shrink-0 ml-1"
          role="tablist"
          aria-label="Choose workbench lens"
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
                }}
                title={`${m.label} (${m.shortcut})`}
                aria-label={`${m.label} (${m.shortcut})`}
                className={`flex size-6 items-center justify-center rounded-md transition cursor-pointer ${
                  isActive
                    ? 'bg-white/18 text-white shadow-xs'
                    : 'text-white/45 hover:text-white/90 hover:bg-white/8'
                }`}
                role="tab"
                aria-selected={isActive}
              >
                <Icon className={`size-3.5 shrink-0 ${isActive ? 'text-workbench-accent' : 'text-white/50'}`} />
              </button>
            );
          })}
        </div>
      </div>

      {/* ── CENTER: Draggable Title Region + Active Session / Branch ── */}
      <div
        data-tauri-drag-region
        className="flex-1 h-full flex items-center justify-center gap-2 cursor-default select-none px-2 min-w-0"
      >
        <span
          data-tauri-drag-region
          className="text-[11.5px] font-medium text-white/70 truncate max-w-[280px]"
        >
          {activeSession?.title || 'Custos Sovereign Workspace'}
        </span>

        {currentBranch && (
          <div
            data-tauri-drag-region
            className="hidden lg:flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/5 border border-white/10 text-[10px] font-mono text-white/50 shrink-0"
          >
            <GitBranch className="size-2.5 text-white/40 shrink-0" />
            <span className="truncate max-w-[90px] text-white/70">{currentBranch}</span>
            {isDirty && (
              <span className="text-amber-400 font-bold shrink-0">
                • {dirtyCount}*
              </span>
            )}
          </div>
        )}
      </div>
 
      {/* ── RIGHT: Unified Resource Tabs & Pane Controls ── */}
      <div
        className={`flex items-center gap-1 shrink-0 h-full ${
          isWebTabOpen && !isWebTabExpanded ? 'border-l border-white/10 pl-2' : ''
        }`}
        style={{
          width:
            isWebTabOpen && !isWebTabExpanded
              ? (column2Width ? `${column2Width}px` : `${100 - splitPercent}%`)
              : undefined,
          minWidth: 0,
        }}
      >
        {isWebTabOpen && (
          <div className="flex-1 min-w-0 flex items-center justify-between gap-1 overflow-hidden h-full">
            {/* Resource Tabs Strip */}
            <div className="flex-1 min-w-0 flex items-center gap-0.5 overflow-x-auto no-scrollbar py-0.5">
              {resourceTabs.tabs.map((tab) => {
                const isActive = tab.id === resourceTabs.activeTabId;
                const isSplit = tab.id === splitTabId;
                const tabType = getTabType(tab);
                return (
                  <div
                    key={tab.id}
                    draggable={true}
                    onDragStart={(e) => {
                      e.dataTransfer.setData('application/custos-tab-id', tab.id);
                      e.dataTransfer.setData('text/plain', tab.id);
                      e.dataTransfer.effectAllowed = 'copyMove';
                    }}
                    onClick={() => {
                      resourceTabs.setActiveTabId(tab.id);
                    }}
                    className={`group flex items-center gap-1.5 px-2 py-0.5 rounded-md cursor-pointer transition shrink-0 text-[11.5px] h-6 select-none ${
                      isActive
                        ? 'bg-white/14 text-white font-medium shadow-xs border border-white/10'
                        : isSplit
                          ? 'bg-purple-500/15 text-purple-200 border border-purple-500/40 font-medium'
                          : 'bg-transparent text-white/50 hover:text-white/80 hover:bg-white/6'
                    }`}
                    title={`${tab.title} (Kéo thả xuống canvas để mở song song)`}
                  >
                    {getTabIcon(tabType, isActive || isSplit)}
                    <span className="truncate max-w-[100px]">{tab.title}</span>

                    {/* Split View Indicator Badges */}
                    {splitTabId && isActive && (
                      <span className="px-1 py-0.2 rounded text-[8.5px] font-mono bg-blue-500/20 text-blue-400 font-semibold border border-blue-500/30 shrink-0">
                        1
                      </span>
                    )}
                    {splitTabId && isSplit && (
                      <span className="px-1 py-0.2 rounded text-[8.5px] font-mono bg-purple-500/25 text-purple-300 font-semibold border border-purple-500/40 shrink-0">
                        2
                      </span>
                    )}

                    {/* Quick Split Toggle Button on Tab Hover */}
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        toggleSplitTab(tab.id);
                      }}
                      className={`opacity-0 group-hover:opacity-100 hover:text-white p-0.5 rounded transition text-white/40 hover:bg-white/10 shrink-0 cursor-pointer ${
                        isSplit ? '!opacity-100 text-purple-300' : ''
                      }`}
                      title={isSplit ? 'Tắt xem song song tab này' : 'Xem song song với tab hiện tại (Split View)'}
                    >
                      <Columns2 className="size-2.5" />
                    </button>

                    {/* Close Tab Button */}
                    <button
                      type="button"
                      onClick={(e) => handleCloseResourceTab(e, tab.id)}
                      className="opacity-0 group-hover:opacity-100 hover:text-white p-0.5 rounded transition text-white/40 hover:bg-white/10 shrink-0 cursor-pointer"
                      title="Close tab"
                    >
                      <X className="size-2.5" />
                    </button>
                  </div>
                );
              })}

              {/* Add New Tab Button */}
              <button
                type="button"
                onClick={() => setIsResourcePickerOpen(true)}
                className="flex size-6 items-center justify-center rounded-md text-white/40 hover:text-white hover:bg-white/10 transition shrink-0 cursor-pointer"
                title="New resource tab"
              >
                <Plus className="size-3" />
              </button>
            </div>

            {/* Pane Controls Right Cluster */}
            <div className="flex items-center gap-1 shrink-0">
              {/* Split View Toggle Button */}
              {resourceTabs.tabs.length > 1 && (
                <button
                  type="button"
                  onClick={() => {
                    if (splitTabId) {
                      closeSplitTab();
                    } else {
                      const secondary = resourceTabs.tabs.find((t) => t.id !== resourceTabs.activeTabId);
                      if (secondary) setSplitTabId(secondary.id);
                    }
                  }}
                  className={`flex size-6 items-center justify-center rounded-md transition cursor-pointer ${
                    splitTabId
                      ? 'bg-purple-600/30 text-purple-200 border border-purple-500/40 shadow-xs'
                      : 'text-white/45 hover:text-white hover:bg-white/8'
                  }`}
                  title={splitTabId ? 'Tắt xem song song (Exit split view)' : 'Bật xem song song 2 tab (Split view)'}
                >
                  <Columns2 className="size-3.5" />
                </button>
              )}

              {/* Subtle Divider */}
              <div className="h-3.5 w-px bg-white/10 mx-0.5 shrink-0" />

              {/* Maximize / Restore Split Toggle */}
              <button
                type="button"
                onClick={toggleExpandWebTab}
                className="flex size-6 items-center justify-center rounded-md text-white/45 hover:text-white hover:bg-white/8 transition cursor-pointer"
                title={isWebTabExpanded ? 'Restore split layout' : 'Maximize resource canvas'}
              >
                {isWebTabExpanded ? <Minimize2 className="size-3.5" /> : <Maximize2 className="size-3.5" />}
              </button>
            </div>
          </div>
        )}

        {/* Resource Canvas Toggle Button */}
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            toggleWebTab();
          }}
          className={`flex size-6 items-center justify-center rounded-md transition cursor-pointer ${
            isWebTabOpen
              ? 'bg-white/18 text-white shadow-xs'
              : 'text-white/45 hover:bg-white/8 hover:text-white/90'
          }`}
          title={`Toggle Resource Canvas (${formatKeyCombo({ ctrlOrCmd: true, shift: true, key: 'B' })})`}
          aria-label="Toggle Resource Canvas"
        >
          <PanelRight className="size-3.5" />
        </button>

        {/* Windows / Linux Native Window Controls */}
        {!isMac && (
          <div className="flex items-center ml-1">
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
