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
  GitBranch,
  ShieldCheck,
  Activity,
  FileCode,
  BookOpen,
  NotebookTabs,
  Layers,
  Server
} from 'lucide-react';
import type { OrcaTabId, ResourceTabsState } from '@/components/views/OrcaTabbedContainer';
import { formatKeyCombo } from '@/lib/utils';

// 3 primary workspace modes
export type AppWorkspaceMode = 'chat' | 'code' | 'research';

interface AppHeaderProps {
  mode: AppWorkspaceMode;
  workspaceTitle?: string;
  onSwitchMode: (mode: AppWorkspaceMode) => void;

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
  onCloseTab: (e: React.MouseEvent, tabId: OrcaTabId) => void;
  onAddNewTab: () => void;

  // Lens presentation
  lensBadge: string;
  lensIconColor: string;

  // Header quick triggers
  onOpenCommandPalette?: () => void;
  attentionCount?: number;
  onOpenAttention?: () => void;
  onOpenKanban?: () => void;
}

export const AppHeader: React.FC<AppHeaderProps> = ({
  mode,
  workspaceTitle,
  onSwitchMode,
  isSidebarCollapsed,
  onToggleSidebar,
  onShowToast,
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
  onOpenKanban
}) => {
  const modes: { id: AppWorkspaceMode; label: string; icon: React.ElementType; shortcut: string; description: string }[] = [
    {
      id: 'chat',
      label: 'Copilot',
      icon: MessageSquare,
      shortcut: formatKeyCombo({ ctrlOrCmd: true, key: '1' }),
      description: 'Custos Copilot workspace'
    },
    {
      id: 'code',
      label: 'Coding',
      icon: Code2,
      shortcut: formatKeyCombo({ ctrlOrCmd: true, key: '2' }),
      description: 'Custos Coding ADE'
    },
    {
      id: 'research',
      label: 'Research',
      icon: FlaskConical,
      shortcut: formatKeyCombo({ ctrlOrCmd: true, key: '3' }),
      description: 'Custos Science Lab'
    }
  ];

  const getTabIcon = (id: OrcaTabId, isActive: boolean) => {
    const cls = `w-3.5 h-3.5 ${isActive ? 'workbench-accent' : 'text-fg-muted group-hover:text-fg-editor transition-colors'}`;
    switch (id) {
      case 'tools': return <Globe className={cls} />;
      case 'changes': return <FileDiff className={cls} />;
      case 'terminal': return <TerminalIcon className={cls} />;
      case 'files': return <FolderTree className={cls} />;
      case 'worktrees': return <GitBranch className={cls} />;
      case 'kanban': return <Bot className={cls} />;
      case 'evidence': return <ShieldCheck className={cls} />;
      case 'dag': return <Activity className={cls} />;
      case 'browser': return <Globe className={cls} />;
      case 'notes': return <FileCode className={cls} />;
      case 'literature': return <BookOpen className={cls} />;
      case 'claims': return <ShieldCheck className={cls} />;
      case 'knowledge': return <FlaskConical className={cls} />;
      case 'experiments': return <NotebookTabs className={cls} />;
      case 'runs': return <FlaskConical className={cls} />;
      case 'synthesis': return <Layers className={cls} />;
      case 'artifacts': return <FileCode className={cls} />;
      case 'fleet': return <Server className={cls} />;
      default: return <Globe className={cls} />;
    }
  };

  return (
    <header className="h-12 bg-[var(--color-surface-1)] border-b border-[var(--color-border-muted)] flex items-center select-none z-30 shrink-0 font-sans text-xs overflow-hidden">
      {/* ── 1. LEFT: Nav Controls + Sidebar + Mode Switcher (Aligned to w-64 when expanded) ── */}
      <div
        className={`flex items-center justify-between shrink-0 h-full transition-all duration-300 ${
          !isSidebarCollapsed ? 'w-64 px-3 border-r border-[var(--color-border-muted)]' : 'w-auto px-2.5 border-r border-[var(--color-border-muted)] gap-2.5'
        }`}
      >
        <div className="flex items-center gap-1">
          {/* Navigation History */}
          <button
            onClick={() => { if (onShowToast) onShowToast('Back'); }}
            className="p-1 rounded text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
            title="Back"
          >
            <ArrowLeft className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => { if (onShowToast) onShowToast('Forward'); }}
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

        {/* 3-Mode Lens Switcher Pill (Icon-only) */}
        <div className="workbench-switcher" role="tablist" aria-label="Choose workbench">
          {modes.map((m) => {
            const Icon = m.icon;
            const isActive = mode === m.id;
            return (
              <button
                key={m.id}
                onClick={() => {
                  onSwitchMode(m.id);
                  if (onShowToast) onShowToast(`Switched to ${m.description} (${m.shortcut})`);
                }}
                title={`${m.label} (${m.shortcut}) — ${m.description}`}
                className="workbench-switcher__item"
                role="tab"
                aria-selected={isActive}
                aria-label={m.label}
              >
                <Icon className={`w-3.5 h-3.5 ${isActive ? 'workbench-accent' : ''}`} />
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
          className={`h-full items-center justify-between px-4 shrink-0 bg-[var(--color-surface-1)] ${
            isWebTabOpen && !isWebTabExpanded ? 'border-r border-border-muted' : ''
          }`}
        >
          {/* Session Title & Lens Badge */}
          <div className="flex items-center gap-2 min-w-0">
            <div className="flex items-center gap-1.5 text-[12px] text-fg-editor font-medium truncate">
              {mode === 'code' ? (
                <Code2 className="w-3.5 h-3.5 workbench-accent shrink-0" />
              ) : mode === 'research' ? (
                <FlaskConical className="w-3.5 h-3.5 workbench-accent shrink-0" />
              ) : (
                <MessageSquare className="w-3.5 h-3.5 workbench-accent shrink-0" />
              )}
              <span className="truncate">{workspaceTitle || 'Session'}</span>
            </div>
            <span className={`workbench-kicker shrink-0 hidden sm:inline ${lensIconColor}`}>
              {lensBadge}
            </span>
          </div>

          {/* Quick Triggers when Right Pane is Closed */}
          {!isWebTabOpen && (
            <div className="flex items-center gap-2 shrink-0">
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
                return (
                  <div
                    key={tab.id}
                    onClick={() => resourceTabs.setActiveTabId(tab.id)}
                    className={`group flex items-center gap-1.5 px-2.5 py-1 rounded-md cursor-pointer transition max-w-[160px] shrink-0 text-[12px] h-7 ${
                      isActive
                        ? 'bg-surface-2 text-fg-editor font-medium border border-border-default shadow-xs'
                        : 'bg-transparent text-fg-muted hover:text-fg-editor hover:bg-surface-2/60'
                    }`}
                  >
                    {getTabIcon(tab.id, isActive)}
                    <span className="truncate">{tab.title}</span>
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
            <div className="flex items-center gap-1 shrink-0 pl-2 border-l border-border-muted">
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
                  className="flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-amber-500/15 border border-amber-500/30 text-amber-500 text-[11px] font-medium animate-pulse"
                  title={`${attentionCount} actions require your attention`}
                >
                  <AlertCircle className="w-3 h-3" />
                  <span className="hidden sm:inline text-[10px]">Needs You</span>
                  <span className="bg-amber-500 text-white px-1 rounded-full text-[9px] font-bold">{attentionCount}</span>
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
