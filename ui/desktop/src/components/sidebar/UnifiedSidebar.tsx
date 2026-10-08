import React, { useState, useRef, useEffect } from 'react';
import {
  Sparkles,
  MessageSquare,
  KeyRound,
  GitFork,
  Activity,
  Database,
  Settings2,
  Plus,
  Search,
  PanelLeftClose,
  PanelLeftOpen,
  Terminal,
  FlaskConical,
  Bot,
  Inbox,
  FolderOpen,
  ChevronDown,
  CheckCircle2,
  MoreHorizontal
} from 'lucide-react';
import { MainTab, Session } from '@/types';

interface UnifiedSidebarProps {
  currentTab: MainTab;
  onSwitchTab: (tab: MainTab) => void;
  currentProject: string;
  projectNames: string[];
  onSelectProject: (projectName: string) => void;
  onNewProjectPrompt: () => void;
  sessions: Session[];
  activeSessionId: string;
  onSelectSession: (id: string) => void;
  onOpenNewSessionModal: () => void;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  onOpenSettings?: () => void;
  onShowToast: (msg: string) => void;
}

type PackFilter = 'all' | 'coding' | 'research' | 'assistant';

const PACK_FILTERS: { id: PackFilter; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
  { id: 'all',       label: 'All',      icon: MessageSquare },
  { id: 'coding',    label: 'Code',     icon: Terminal },
  { id: 'research',  label: 'Research', icon: FlaskConical },
  { id: 'assistant', label: 'Assist',   icon: Bot },
];

function sessionMatchesPack(session: Session, pack: PackFilter): boolean {
  if (pack === 'all') return true;
  if (session.pack) {
    if (pack === 'coding')    return session.pack === 'engineering' || session.pack === 'coding';
    if (pack === 'research')  return session.pack === 'research';
    if (pack === 'assistant') return session.pack === 'assistant';
  }
  return true;
}

function PackDot({ pack }: { pack?: string }) {
  if (!pack) return null;
  const cls =
    pack === 'engineering' || pack === 'coding' ? 'text-emerald-500' :
    pack === 'research'  ? 'workbench-accent' :
    pack === 'assistant' ? 'text-fg-editor' : 'text-fg-subtle';
  return (
    <span className={`font-mono text-[9px] shrink-0 ${cls}`}>
      ●
    </span>
  );
}

function StatusChip({ status }: { status?: string }) {
  if (!status) return null;
  const cls =
    status === 'Active' || status === 'Running' ? 'text-emerald-500' :
    status === 'Done' ? 'workbench-accent' :
    status === 'Blocked' ? 'text-amber-500' : 'text-fg-subtle';
  return (
    <span className={`font-mono text-[10px] shrink-0 ${cls}`}>
      {status}
    </span>
  );
}

export const UnifiedSidebar: React.FC<UnifiedSidebarProps> = ({
  currentTab,
  onSwitchTab,
  currentProject,
  projectNames,
  onSelectProject,
  onNewProjectPrompt,
  sessions,
  activeSessionId,
  onSelectSession,
  onOpenNewSessionModal,
  isCollapsed,
  onToggleCollapse,
  onOpenSettings,
  onShowToast,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [packFilter, setPackFilter] = useState<PackFilter>('all');
  const [isProjectOpen, setIsProjectOpen] = useState(false);
  const [isToolsOpen, setIsToolsOpen] = useState(false);
  const projectDropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (projectDropdownRef.current && !projectDropdownRef.current.contains(e.target as Node)) {
        setIsProjectOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  const filteredSessions = sessions.filter((s) => {
    const q = searchQuery.toLowerCase();
    const matchesSearch = !q || s.title.toLowerCase().includes(q) || s.preview.toLowerCase().includes(q);
    return matchesSearch && sessionMatchesPack(s, packFilter);
  });

  /* ── COLLAPSED RAIL (44px) ── */
  if (isCollapsed) {
    return (
      <aside 
        className="w-11 h-full z-20 select-none flex flex-col items-center justify-between py-2 shrink-0 bg-surface-1 border-r border-border-default"
      >
        <div className="flex flex-col items-center gap-1.5 w-full px-1">
          <button
            onClick={onToggleCollapse}
            className="p-2 rounded-lg text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
            title="Expand Sidebar (⌘B)"
          >
            <PanelLeftOpen className="w-4 h-4" />
          </button>

          <div className="w-6 h-px bg-border-muted my-1" />

          <button
            onClick={onOpenNewSessionModal}
            className="p-2 rounded-lg hover:bg-surface-2 transition bg-surface-2 border border-border-default text-fg-editor"
            title="New Chat (⌘N)"
          >
            <Plus className="w-4 h-4 workbench-accent" />
          </button>

          <div className="w-6 h-px bg-border-muted my-1" />

          {/* Core Lens Shortcuts */}
          <button
            onClick={() => onSwitchTab('studio')}
            className={`p-2 rounded-lg transition ${currentTab === 'studio' ? 'bg-surface-2 text-fg-editor' : 'text-fg-muted hover:text-fg-editor'}`}
            title="Claude Chat (Copilot)"
          >
            <MessageSquare className="w-4 h-4 workbench-accent" />
          </button>

          <button
            onClick={() => onSwitchTab('studio')}
            className="p-2 rounded-lg text-fg-muted hover:text-fg-editor transition"
            title="Claude Code (Codex)"
          >
            <Terminal className="w-4 h-4 text-emerald-500" />
          </button>

          <button
            onClick={() => onSwitchTab('studio')}
            className="p-2 rounded-lg text-fg-muted hover:text-fg-editor transition"
            title="Claude Science"
          >
            <FlaskConical className="w-4 h-4 workbench-accent" />
          </button>
        </div>

        <div className="flex flex-col items-center gap-1 pb-1">
          <button
            onClick={() => onOpenSettings ? onOpenSettings() : onSwitchTab('settings')}
            className="p-2 rounded-lg text-fg-muted hover:text-fg-editor transition"
            title="Settings (⌘,)"
          >
            <Settings2 className="w-4 h-4" />
          </button>
        </div>
      </aside>
    );
  }

  /* ── EXPANDED CLAUDE DESKTOP SIDEBAR (240px) ── */
  return (
    <aside
      className="w-60 flex flex-col h-full z-20 select-none overflow-hidden shrink-0 text-xs bg-surface-1 border-r border-border-default"
    >
      {/* ── 1. Brand Header ── */}
      <div
        className="h-10 px-3 flex items-center justify-between shrink-0 border-b border-border-muted"
      >
        <div className="flex items-center gap-2">
          <div className="w-5 h-5 rounded-md bg-surface-2 border border-border-default flex items-center justify-center shrink-0">
            <Sparkles className="w-3.5 h-3.5 workbench-accent" />
          </div>
          <span className="font-semibold text-fg-editor tracking-tight text-xs">
            Custos SADE
          </span>
          <span className="font-mono text-[9px] text-emerald-500 px-1 rounded bg-emerald-500/10 border border-emerald-500/30 font-bold">
            v0.1
          </span>
        </div>

        <button
          onClick={onToggleCollapse}
          className="p-1 rounded text-fg-muted hover:text-fg-editor transition"
          title="Collapse Sidebar (⌘B)"
        >
          <PanelLeftClose className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* ── 2. Prominent "+ New Chat" Button (Claude Desktop style) ── */}
      <div className="p-3 pb-2 shrink-0">
        <button
          onClick={onOpenNewSessionModal}
          className="w-full flex items-center justify-between px-3 py-2 rounded-xl text-xs font-semibold transition shadow-sm group bg-surface-2 border border-border-default text-fg-editor hover:bg-surface-3"
        >
          <div className="flex items-center gap-2">
            <Plus className="w-3.5 h-3.5 workbench-accent group-hover:scale-110 transition-transform" />
            <span>New chat</span>
          </div>
          <kbd 
            className="text-[10px] font-mono text-fg-muted px-1.5 py-0.2 rounded bg-canvas border border-border-muted"
          >
            ⌘N
          </kbd>
        </button>
      </div>

      {/* ── 3. Workspace Dropdown ── */}
      <div className="px-3 pb-2 shrink-0 relative" ref={projectDropdownRef}>
        <button
          onClick={() => setIsProjectOpen(!isProjectOpen)}
          className="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg text-[11px] transition text-fg-editor hover:text-fg-editor bg-surface-0 border border-border-default"
        >
          <span className="flex items-center gap-1.5 truncate">
            <FolderOpen className="w-3.5 h-3.5 workbench-accent shrink-0" />
            <span className="truncate font-medium">{currentProject}</span>
          </span>
          <ChevronDown className="w-3 h-3 text-fg-subtle shrink-0" />
        </button>

        {isProjectOpen && (
          <div
            className="absolute left-3 right-3 mt-1 rounded-xl shadow-2xl p-1.5 text-xs z-50 animate-in fade-in duration-100 bg-surface-2 border border-border-default"
          >
            <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-fg-muted">
              Workspaces
            </div>
            {projectNames.map((proj) => (
              <button
                key={proj}
                onClick={() => {
                  onSelectProject(proj);
                  setIsProjectOpen(false);
                  onShowToast(`Switched workspace to ${proj}`);
                }}
                className={`w-full text-left px-2 py-1 rounded text-xs transition flex items-center justify-between ${
                  proj === currentProject ? 'bg-surface-3 text-fg-editor font-medium' : 'text-fg-muted hover:text-fg-editor'
                }`}
              >
                <span>{proj}</span>
                {proj === currentProject && <CheckCircle2 className="w-3 h-3 text-emerald-500" />}
              </button>
            ))}
            <div className="my-1 border-t border-border-default" />
            <button
              onClick={() => { setIsProjectOpen(false); onNewProjectPrompt(); }}
              className="w-full text-left px-2 py-1 rounded text-[11px] workbench-accent hover:bg-surface-3 transition flex items-center gap-1.5"
            >
              <Plus className="w-3 h-3" />
              <span>New Workspace...</span>
            </button>
          </div>
        )}
      </div>

      {/* ── 4. Search & Filters ── */}
      <div className="px-3 pb-2 shrink-0 space-y-1.5 border-b border-border-muted">
        <div className="relative">
          <Search className="w-3 h-3 absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-subtle" />
          <input
            type="text"
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            placeholder="Search conversations..."
            className="w-full pl-7 pr-2 py-1 rounded text-fg-editor placeholder-fg-subtle text-[11px] focus:outline-none font-sans bg-surface-0 border border-border-default"
          />
        </div>

        {/* Filter Pills */}
        <div className="flex items-center gap-1 font-mono text-[10px]">
          {PACK_FILTERS.map(f => {
            const active = packFilter === f.id;
            return (
              <button
                key={f.id}
                onClick={() => setPackFilter(f.id)}
                className={`px-2 py-0.5 rounded transition font-medium ${active ? 'bg-surface-2 text-fg-editor border border-border-default' : 'bg-transparent text-fg-muted border border-transparent'}`}
              >
                {f.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* ── 5. Recents Conversation List ── */}
      <div className="flex-1 overflow-y-auto px-2 py-1.5 space-y-0.5">
        <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">
          Recent Tasks
        </div>

        {filteredSessions.length === 0 ? (
          <div className="p-6 text-center text-fg-subtle text-xs">
            <Inbox className="w-4 h-4 mx-auto mb-1.5 opacity-50" />
            <p>No recent conversations</p>
          </div>
        ) : (
          filteredSessions.map((session) => {
            const isActive = session.id === activeSessionId;
            return (
              <div
                key={session.id}
                onClick={() => {
                  onSelectSession(session.id);
                  onSwitchTab('studio');
                }}
                className={`w-full px-2.5 py-2 rounded-lg cursor-pointer transition text-left space-y-0.5 group ${isActive ? 'bg-surface-2 border border-border-default text-fg-editor' : 'bg-transparent border border-transparent text-fg-muted'}`}
              >
                <div className="flex items-center justify-between gap-1">
                  <div className="flex items-center gap-1.5 truncate">
                    <PackDot pack={session.pack} />
                    <span className="font-semibold text-xs truncate text-fg-editor">
                      {session.title}
                    </span>
                  </div>
                  <StatusChip status={session.taskStatus} />
                </div>
                <p className="text-[11px] text-fg-subtle truncate pl-3">
                  {session.preview}
                </p>
              </div>
            );
          })
        )}
      </div>

      {/* ── 6. Advanced Tools Drawer & Footer ── */}
      <div 
        className="shrink-0 p-2.5 space-y-1.5 border-t border-border-muted bg-surface-1"
      >
        {/* Toggle secondary tools */}
        <button
          onClick={() => setIsToolsOpen(!isToolsOpen)}
          className="w-full flex items-center justify-between px-2 py-1 rounded text-[11px] text-fg-muted hover:text-fg-editor transition"
        >
          <span className="flex items-center gap-1.5 font-medium">
            <MoreHorizontal className="w-3.5 h-3.5 text-fg-subtle" />
            <span>Advanced Tools</span>
          </span>
          <ChevronDown className={`w-3 h-3 text-fg-subtle transition-transform ${isToolsOpen ? 'rotate-180' : ''}`} />
        </button>

        {isToolsOpen && (
          <div className="space-y-0.5 pl-2 font-mono text-[11px]">
            <button
              onClick={() => onSwitchTab('providers')}
              className={`w-full text-left px-2 py-1 rounded transition flex items-center gap-1.5 ${currentTab === 'providers' ? 'text-fg-editor font-bold' : 'text-fg-muted hover:text-fg-editor'}`}
            >
              <KeyRound className="w-3 h-3 workbench-accent" />
              <span>Providers & Keys</span>
            </button>
            <button
              onClick={() => onSwitchTab('chains')}
              className={`w-full text-left px-2 py-1 rounded transition flex items-center gap-1.5 ${currentTab === 'chains' ? 'text-fg-editor font-bold' : 'text-fg-muted hover:text-fg-editor'}`}
            >
              <GitFork className="w-3 h-3 text-emerald-500" />
              <span>OmniRoute Cascade</span>
            </button>
            <button
              onClick={() => onSwitchTab('telemetry')}
              className={`w-full text-left px-2 py-1 rounded transition flex items-center gap-1.5 ${currentTab === 'telemetry' ? 'text-fg-editor font-bold' : 'text-fg-muted hover:text-fg-editor'}`}
            >
              <Activity className="w-3 h-3 workbench-accent" />
              <span>Telemetry & Logs</span>
            </button>
            <button
              onClick={() => onSwitchTab('cache')}
              className={`w-full text-left px-2 py-1 rounded transition flex items-center gap-1.5 ${currentTab === 'cache' ? 'text-fg-editor font-bold' : 'text-fg-muted hover:text-fg-editor'}`}
            >
              <Database className="w-3 h-3 text-fg-subtle" />
              <span>KV Semantic Cache</span>
            </button>
          </div>
        )}

        {/* Footer row */}
        <div className="flex items-center justify-between pt-1">
          <div className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
            <span className="font-mono text-[10px] text-emerald-500">Sovereign · Safe</span>
          </div>

          <button
            onClick={() => onOpenSettings ? onOpenSettings() : onSwitchTab('settings')}
            className="p-1 rounded text-fg-muted hover:text-fg-editor transition"
            title="Settings (⌘,)"
          >
            <Settings2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </aside>
  );
};

export default UnifiedSidebar;
