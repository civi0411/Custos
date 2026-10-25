import WorktreeIcon from '~icons/codicon/worktree';
import React from 'react';
import {
  MessageSquare,
  Maximize2,
  Minimize2,
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
  FlaskConical,
} from 'lucide-react';
import { type OrcaTabId, type ResourceTabsState, getTabType } from '@/components/workbenches/OrcaTabbedContainer';
import { formatKeyCombo } from '@/lib/utils';
import { getAgentBrandIcon } from '@/components/common/AgentIcons';


import type { ActiveWorkbench, AppWorkspaceMode } from '@/types/workbench';
export type { ActiveWorkbench, AppWorkspaceMode };



export interface AppHeaderProps {
  mode?: ActiveWorkbench;
  workspaceTitle?: string;
  activeWorkspace?: import('@/types/domain').ExecutionWorkspace | null;
  onSwitchMode?: (mode: ActiveWorkbench) => void;

  isSidebarCollapsed?: boolean;
  onToggleSidebar?: () => void;
  onShowToast?: (msg: string) => void;

  // Split & Resource Canvas state
  splitPercent?: number;
  isWebTabOpen?: boolean;
  isWebTabExpanded?: boolean;
  onToggleWebTab?: () => void;
  onToggleExpandWebTab?: () => void;
  resourceTabs: ResourceTabsState;
  onCloseTab: (e: React.MouseEvent, tabId: string) => void;
  onAddNewTab: () => void;

  // Lens presentation
  lensBadge?: string;
  lensIconColor?: string;

  // Header quick triggers
  onOpenCommandPalette?: () => void;
  attentionCount?: number;
  onOpenAttention?: () => void;
  onOpenKanban?: () => void;

  // Running jobs per tab indicator
  runningJobsByTab?: Record<string, number>;
}

export type ResourceTabHeaderProps = AppHeaderProps;

export const ResourceTabHeader: React.FC<ResourceTabHeaderProps> = ({
  workspaceTitle,
  isWebTabExpanded = false,
  onToggleWebTab,
  onToggleExpandWebTab,
  resourceTabs,
  onCloseTab,
  onAddNewTab,
  attentionCount = 0,
  onOpenAttention,
  onOpenKanban,
  runningJobsByTab,
}) => {
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
    <header className="h-9 w-full bg-[#18181b] border-b border-white/10 flex items-center justify-between px-2 shrink-0 select-none z-30 font-sans text-xs min-w-0">
      {/* Tabs List */}
      <div className="flex items-center gap-1 min-w-0 flex-1 h-full overflow-x-auto no-scrollbar pr-2">
        {/* Back to Chat pill when in Full Expanded View */}
        {isWebTabExpanded && onToggleExpandWebTab && (
          <>
            <button
              type="button"
              onClick={onToggleExpandWebTab}
              className="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[12px] text-white/60 hover:text-white hover:bg-white/10 transition shrink-0 cursor-pointer"
              title="Restore split layout"
            >
              <MessageSquare className="w-3.5 h-3.5 text-workbench-accent" />
              <span className="font-medium truncate max-w-[120px]">{workspaceTitle || 'Chat'}</span>
            </button>
            <div className="w-px h-4 bg-white/10 mx-1 shrink-0" />
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
              className={`group flex items-center gap-1.5 px-2.5 py-1 rounded-md cursor-pointer transition max-w-[170px] shrink-0 text-[12px] h-7 ${
                isActive
                  ? 'bg-white/12 text-white font-medium shadow-xs'
                  : 'bg-transparent text-white/50 hover:text-white/80 hover:bg-white/6'
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
                type="button"
                onClick={(e) => onCloseTab(e, tab.id)}
                className="opacity-0 group-hover:opacity-100 hover:text-white p-0.5 rounded transition text-white/40 hover:bg-white/10 shrink-0 ml-0.5 cursor-pointer"
                title="Close tab"
              >
                <X className="w-3 h-3" />
              </button>
            </div>
          );
        })}

        {/* Add New Tab Button */}
        <button
          type="button"
          onClick={onAddNewTab}
          className="p-1 rounded-md text-white/40 hover:text-white hover:bg-white/10 transition shrink-0 ml-0.5 cursor-pointer"
          title="New resource tab"
        >
          <Plus className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Right Quick Controls */}
      <div className="flex items-center gap-1 shrink-0 pl-2 border-l border-white/10">
        {/* Attention Alert Pill */}
        {attentionCount > 0 && onOpenAttention && (
          <button
            type="button"
            onClick={onOpenAttention}
            className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-amber-500/10 border border-amber-500/30 text-amber-400 text-[11px] font-medium hover:bg-amber-500/20 transition cursor-pointer"
            title={`${attentionCount} actions require your attention`}
          >
            <AlertCircle className="w-3.5 h-3.5 text-amber-500" />
            <span className="hidden sm:inline text-[10px]">Attention</span>
            <span className="bg-amber-500/20 text-amber-300 px-1 rounded text-[9px] font-mono font-semibold">{attentionCount}</span>
          </button>
        )}

        {/* Fleet Kanban */}
        {onOpenKanban && (
          <button
            type="button"
            onClick={onOpenKanban}
            className="p-1 rounded-md text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer"
            title={`Fleet Kanban Board (${formatKeyCombo({ ctrlOrCmd: true, alt: true, key: 'K' })})`}
          >
            <Bot className="w-3.5 h-3.5" />
          </button>
        )}

        {/* Maximize / Restore Split Toggle */}
        {onToggleExpandWebTab && (
          <button
            type="button"
            onClick={onToggleExpandWebTab}
            className="p-1 rounded-md text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer"
            title={isWebTabExpanded ? 'Restore split layout' : 'Maximize resource canvas'}
          >
            {isWebTabExpanded ? <Minimize2 className="w-3.5 h-3.5" /> : <Maximize2 className="w-3.5 h-3.5" />}
          </button>
        )}

        {/* Close Resource Pane */}
        {onToggleWebTab && (
          <button
            type="button"
            onClick={onToggleWebTab}
            className="p-1 rounded-md text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer"
            title="Close resource canvas"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        )}
      </div>
    </header>
  );
};

export const AppHeader = ResourceTabHeader;

