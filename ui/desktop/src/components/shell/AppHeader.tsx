import WorktreeIcon from '~icons/codicon/worktree';
import React from 'react';
import {
  MessageSquare,
  Code2,
  FlaskConical,
  ArrowLeft,
  ArrowRight,
  PanelLeft,
  PanelRight,
  Maximize2,
  Minimize2,
  Search,
  Bot,
  AlertCircle,
  X,
  Plus,
  Globe,
  FileDiff,
  Terminal as TerminalIcon,
  FolderTree,
  ShieldCheck,
  Activity,
  FileCode,
  BookOpen,
  NotebookTabs,
  Layers,
  Server,
  LayoutGrid,
  Smartphone,
  Send,
  FileText,
  Users,
  Calendar,
  GitBranch,
} from 'lucide-react';
import { type OrcaTabId, type ResourceTabsState, getTabType } from '@/components/workbenches/OrcaTabbedContainer';
import { formatKeyCombo } from '@/lib/utils';
import { useAppContext } from '@/context/AppContext';
import { getAgentBrandIcon } from '@/components/common/AgentIcons';


// Three workbench lenses over the current Workspace and Task/Session state.
export type ActiveWorkbench = 'copilot' | 'engineering' | 'research';
export type AppWorkspaceMode = ActiveWorkbench;

interface AppHeaderProps {
  mode: ActiveWorkbench;
  workspaceTitle?: string;
  activeWorkspace?: import('@/types/domain').ExecutionWorkspace | null;
  onSwitchMode: (mode: ActiveWorkbench) => void;

  isSidebarCollapsed: boolean;
  onToggleSidebar: () => void;
  onShowToast?: (msg: string) => void;

  // Split & Resource Canvas state
  splitPercent: number;
  isWebTabOpen: boolean;
  isWebTabExpanded: boolean;
  onToggleWebTab: () => void;
  onToggleExpandWebTab: () => void;
  resourceTabs: ResourceTabsState;
  onCloseTab: (e: React.MouseEvent, tabId: string) => void;
  onAddNewTab: () => void;

  // Lens presentation
  lensBadge: string;
  lensIconColor: string;

  // Header quick triggers
  onOpenCommandPalette?: () => void;
  attentionCount?: number;
  onOpenAttention?: () => void;
  onOpenKanban?: () => void;

  // Running jobs per tab indicator
  runningJobsByTab?: Record<string, number>;
}

export const AppHeader: React.FC<AppHeaderProps> = ({
  mode,
  workspaceTitle,
  activeWorkspace,
  onSwitchMode,
  isSidebarCollapsed,
  onToggleSidebar,
  onShowToast: _onShowToast,
  splitPercent,
  isWebTabOpen,
  isWebTabExpanded,
  onToggleWebTab,
  onToggleExpandWebTab,
  resourceTabs,
  onCloseTab,
  onAddNewTab,
  lensBadge,
  lensIconColor,
  onOpenCommandPalette,
  attentionCount = 0,
  onOpenAttention,
  onOpenKanban,
  runningJobsByTab,
}) => {
  const { activeDetection } = useAppContext();
  const modes: { id: ActiveWorkbench; label: string; icon: React.ElementType; shortcut: string; description: string }[] = [
    {
      id: 'copilot',
      label: 'Copilot',
      icon: MessageSquare,
      shortcut: formatKeyCombo({ ctrlOrCmd: true, key: '1' }),
      description: 'Copilot workbench'
    },
    {
      id: 'engineering',
      label: 'Coding',
      icon: Code2,
      shortcut: formatKeyCombo({ ctrlOrCmd: true, key: '2' }),
      description: 'Coding workbench'
    },
    {
      id: 'research',
      label: 'Research',
      icon: FlaskConical,
      shortcut: formatKeyCombo({ ctrlOrCmd: true, key: '3' }),
      description: 'Research workbench'
    }
  ];

  const getTabIcon = (id: OrcaTabId, isActive: boolean, agentId?: string) => {
    const cls = `w-3.5 h-3.5 ${isActive ? 'workbench-accent' : 'text-fg-muted group-hover:text-fg-editor transition-colors'}`;
    if (agentId || id === 'agents') {
      return getAgentBrandIcon(agentId || 'agents', 14, 'shrink-0');
    }
    switch (id) {
      case 'tools': return <LayoutGrid className={cls} />;
      case 'files': return <FolderTree className={cls} />;
      case 'changes': return <FileDiff className={cls} />;
      case 'activity': return <Activity className={cls} />;
      case 'terminal': return <TerminalIcon className={cls} />;
      case 'worktrees': return <WorktreeIcon className={cls} />;
      case 'kanban': return <Bot className={cls} />;
      case 'evidence': return <ShieldCheck className={cls} />;
      case 'dag': return <Activity className={cls} />;
      case 'browser': return <Globe className={cls} />;
      case 'emulator': return <Smartphone className={cls} />;
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
      case 'fleet': return <Server className={cls} />;
      default: return <Globe className={cls} />;
    }
  };

  return (
    <header className="h-12 bg-surface-1 border-b border-border-default flex items-center select-none z-30 shrink-0 font-sans text-xs overflow-hidden">
      {/* ── 1. LEFT: Nav Controls + Sidebar + Mode Switcher (Aligned to w-64 when expanded) ── */}
      <div
        className={`flex items-center justify-between shrink-0 h-full transition-all duration-300 ${!isSidebarCollapsed ? 'w-64 px-3 border-r border-border-default' : 'w-auto px-2.5 border-r border-border-default gap-2.5'
          }`}
      >
        <div className="flex items-center gap-1">
          {/* Navigation History */}
          <button
            disabled
            className="p-1 rounded text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
            title="Back"
          >
            <ArrowLeft className="w-3.5 h-3.5" />
          </button>
          <button
            disabled
            className="p-1 rounded text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
            title="Forward"
          >
            <ArrowRight className="w-3.5 h-3.5" />
          </button>

          {/* Toggle Sidebar */}
          <button
            onClick={onToggleSidebar}
            className={`p-1 rounded transition ml-0.5 ${!isSidebarCollapsed ? 'text-fg-muted hover:text-fg-editor hover:bg-surface-2' : 'workbench-accent bg-surface-2'}`}
            title={`Toggle Sidebar (${formatKeyCombo({ ctrlOrCmd: true, key: 'B' })})`}
          >
            <PanelLeft className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* 3-Mode Lens Switcher Segmented Control */}
        <div className="flex items-center gap-0.5 bg-surface-2 p-0.5 rounded-lg border border-border-muted" role="tablist" aria-label="Choose workbench">
          {modes.map((m) => {
            const Icon = m.icon;
            const isActive = mode === m.id;
            return (
              <button
                key={m.id}
                onClick={() => onSwitchMode(m.id)}
                title={`${m.label} (${m.shortcut}) — ${m.description}`}
                className={`flex h-7 w-8 items-center justify-center rounded-md transition cursor-pointer ${isActive
                    ? 'bg-surface-0 text-fg-editor border border-border-default shadow-xs font-semibold'
                    : 'text-fg-muted hover:text-fg-editor hover:bg-surface-3/50 border border-transparent'
                  }`}
                role="tab"
                aria-selected={isActive}
                aria-label={m.label}
              >
                <Icon className={`w-3.5 h-3.5 shrink-0 ${isActive ? 'workbench-accent' : 'text-fg-subtle'}`} />
              </button>
            );
          })}
        </div>
      </div>

      {/* ── 2. CENTER & RIGHT: Split-Aligned Panes Headers (Zero Seam Offset) ── */}
      <div className="flex-1 flex h-full overflow-hidden min-w-0">
        {/* CONVERSATION PANE HEADER (Synchronized with splitPercent) */}
        <div
          style={{
            width: isWebTabExpanded
              ? '0%'
              : isWebTabOpen
                ? `${splitPercent}%`
                : '100%',
            display: isWebTabExpanded ? 'none' : 'flex'
          }}
          className={`h-full items-center justify-between px-4 shrink-0 bg-surface-1 ${isWebTabOpen && !isWebTabExpanded ? 'border-r border-border-default' : ''
            }`}
        >
          {/* Session Title & Lens Badge */}
          <div className="flex items-center gap-2 min-w-0">
            <div className="flex items-center gap-1.5 text-[12px] text-fg-editor font-medium truncate">
              {mode === 'engineering' ? (
                <Code2 className="w-3.5 h-3.5 workbench-accent shrink-0" />
              ) : mode === 'research' ? (
                <FlaskConical className="w-3.5 h-3.5 workbench-accent shrink-0" />
              ) : (
                <MessageSquare className="w-3.5 h-3.5 workbench-accent shrink-0" />
              )}
              <span className="truncate">{workspaceTitle || 'Session'}</span>
            </div>
            {mode === 'engineering' && (activeWorkspace?.active_branch || (activeWorkspace?.kind?.type === 'git' ? activeWorkspace.kind.branch : null) || activeDetection?.branch) && (
              <div className="hidden md:flex items-center gap-1.5 px-2 py-0.5 rounded bg-surface-2 text-[10px] font-mono text-fg-muted shrink-0 border border-border-default">
                <GitBranch className="w-3 h-3 text-fg-subtle shrink-0" />
                <span className="text-fg-editor truncate max-w-[90px]">
                  {activeWorkspace?.active_branch || (activeWorkspace?.kind?.type === 'git' ? activeWorkspace.kind.branch : null) || activeDetection?.branch}
                </span>
                {Boolean(activeWorkspace ? (activeWorkspace.dirty_manifest?.is_dirty ?? activeWorkspace.dirtyManifest?.is_dirty) : activeDetection?.is_dirty) && (
                  <span className="text-amber-500 font-medium shrink-0">• {activeWorkspace ? (activeWorkspace.dirty_manifest?.modified_files?.length ?? activeWorkspace.dirtyManifest?.modified_files?.length ?? 0) : (activeDetection?.dirty_count ?? 0)}*</span>
                )}
              </div>
            )}
            <span className={`workbench-kicker shrink-0 hidden sm:inline ${lensIconColor}`}>
              {lensBadge}
            </span>
          </div>

          {/* Quick Triggers when Right Pane is Closed */}
          {!isWebTabOpen && (
            <div className="flex items-center gap-1.5 shrink-0">

              {onOpenCommandPalette && (
                <button
                  onClick={onOpenCommandPalette}
                  className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-canvas hover:bg-surface-2 border border-border-default text-fg-muted hover:text-fg-editor text-[11px] transition"
                  title={`Jump Palette (${formatKeyCombo({ ctrlOrCmd: true, key: 'K' })})`}
                >
                  <Search className="w-3.5 h-3.5 text-fg-muted" />
                  <span className="hidden md:inline">Jump...</span>
                  <kbd className="hidden lg:inline text-[9px] font-mono bg-surface-1 px-1 py-0.2 rounded border border-border-default">
                    {formatKeyCombo({ ctrlOrCmd: true, key: 'K' })}
                  </kbd>
                </button>
              )}
              <button
                onClick={onToggleWebTab}
                className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-surface-2 hover:bg-surface-3 text-fg-editor text-[11px] border border-border-default transition"
                title={`Open Resource Canvas (${formatKeyCombo({ ctrlOrCmd: true, shift: true, key: 'B' })})`}
              >
                <PanelRight className="w-3.5 h-3.5 text-fg-muted" />
                <span className="hidden sm:inline">Resources</span>
                <kbd className="text-[9px] font-mono bg-canvas px-1 py-0.2 rounded border border-border-default">
                  {formatKeyCombo({ ctrlOrCmd: true, shift: true, key: 'B' })}
                </kbd>
              </button>
            </div>
          )}
        </div>

        {/* RESOURCE CANVAS TABS HEADER (Synchronized with 100 - splitPercent) */}
        {isWebTabOpen && (
          <div
            style={{
              width: isWebTabExpanded ? '100%' : `${100 - splitPercent}%`
            }}
            className="h-full flex items-center justify-between px-2 min-w-0 shrink-0 bg-[var(--color-surface-1)]"
          >
            {/* Tabs List */}
            <div className="flex items-center gap-1 min-w-0 flex-1 h-full overflow-x-auto no-scrollbar pr-2">
              {/* Back to Chat pill when in Full Expanded View */}
              {isWebTabExpanded && (
                <>
                  <button
                    onClick={onToggleExpandWebTab}
                    className="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[12px] text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition shrink-0"
                    title="Restore split layout"
                  >
                    <MessageSquare className="w-3.5 h-3.5 workbench-accent" />
                    <span className="font-medium truncate max-w-[120px]">{workspaceTitle || 'Chat'}</span>
                  </button>
                  <div className="w-px h-4 bg-border-muted mx-1 shrink-0" />
                </>
              )}

              {/* Resource Tabs */}
              {resourceTabs.tabs.map((tab) => {
                const isActive = tab.id === resourceTabs.activeTabId;
                const tabType = getTabType(tab);
                const runningCount = runningJobsByTab?.[tab.id] ?? runningJobsByTab?.[tabType] ?? 0;
                return (
                  <div
                    key={tab.id}
                    onClick={() => resourceTabs.setActiveTabId(tab.id)}
                    className={`group flex items-center gap-1.5 px-2.5 py-1 rounded-md cursor-pointer transition max-w-[170px] shrink-0 text-[12px] h-7 ${isActive
                      ? 'bg-surface-2 text-fg-editor font-medium shadow-xs'
                      : 'bg-transparent text-fg-muted hover:text-fg-editor hover:bg-surface-2/60'
                      }`}
                  >
                    {getTabIcon(tabType, isActive, tab.agentId)}
                    <span className="truncate">{tab.title}</span>
                    {runningCount > 0 && (
                      <span
                        className="inline-flex items-center gap-0.5 text-[10px] text-emerald-400 font-mono font-bold shrink-0 ml-0.5"
                        title={`${runningCount} job${runningCount > 1 ? 's' : ''} running`}
                      >
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                        {runningCount > 1 && <span>{runningCount}</span>}
                      </span>
                    )}
                    <button
                      onClick={(e) => onCloseTab(e, tab.id)}
                      className="opacity-0 group-hover:opacity-100 hover:text-fg-editor p-0.5 rounded transition text-fg-muted shrink-0 ml-0.5"
                      title="Close tab"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </div>
                );
              })}

              {/* Add New Tab Button */}
              <button
                onClick={onAddNewTab}
                className="p-1 rounded-md text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition shrink-0 ml-0.5"
                title="New resource tab"
              >
                <Plus className="w-3.5 h-3.5" />
              </button>


            </div>

            {/* Right Quick Controls */}
            <div className="flex items-center gap-1 shrink-0 pl-2 border-l border-border-default">
              {/* Jump Palette Trigger */}
              {onOpenCommandPalette && (
                <button
                  onClick={onOpenCommandPalette}
                  className="flex items-center gap-1 px-2 py-1 rounded-md bg-canvas hover:bg-surface-2 border border-border-default text-fg-muted hover:text-fg-editor text-[11px] transition"
                  title={`Jump Palette (${formatKeyCombo({ ctrlOrCmd: true, key: 'K' })} / ${formatKeyCombo({ ctrlOrCmd: true, key: 'P' })})`}
                >
                  <Search className="w-3 h-3 text-fg-muted" />
                  <span className="hidden xl:inline">Jump...</span>
                  <kbd className="hidden 2xl:inline text-[9px] font-mono bg-surface-1 px-1 rounded border border-border-default">{formatKeyCombo({ ctrlOrCmd: true, key: 'K' })}</kbd>
                </button>
              )}

              {/* Attention Alert Pill */}
              {attentionCount > 0 && (
                <button
                  onClick={onOpenAttention}
                  className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-surface-2 border border-border-default text-fg-editor text-[11px] font-medium hover:bg-surface-3 transition"
                  title={`${attentionCount} actions require your attention`}
                >
                  <AlertCircle className="w-3.5 h-3.5 text-amber-500" />
                  <span className="hidden sm:inline text-[10px]">Attention</span>
                  <span className="bg-surface-3 text-fg-editor px-1 rounded text-[9px] font-mono font-semibold">{attentionCount}</span>
                </button>
              )}

              {/* Fleet Kanban */}
              {onOpenKanban && (
                <button
                  onClick={onOpenKanban}
                  className="p-1 rounded-md text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
                  title={`Fleet Kanban Board (${formatKeyCombo({ ctrlOrCmd: true, alt: true, key: 'K' })})`}
                >
                  <Bot className="w-3.5 h-3.5" />
                </button>
              )}

              {/* Maximize / Restore Split Toggle */}
              <button
                onClick={onToggleExpandWebTab}
                className="p-1 rounded-md text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
                title={isWebTabExpanded ? 'Restore split layout' : 'Maximize resource canvas'}
              >
                {isWebTabExpanded ? <Minimize2 className="w-3.5 h-3.5" /> : <Maximize2 className="w-3.5 h-3.5" />}
              </button>

              {/* Close Resource Pane */}
              <button
                onClick={onToggleWebTab}
                className="p-1 rounded-md text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
                title="Close resource canvas"
              >
                <PanelRight className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        )}
      </div>
    </header>
  );
};
