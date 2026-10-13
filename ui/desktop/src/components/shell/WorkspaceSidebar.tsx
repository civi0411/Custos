import React, { useState, useRef, useEffect } from 'react';
import { useAppContext } from '@/context/AppContext';
import {
  SlidersHorizontal,
  Search,
  ChevronDown,
  Settings,
  Trash2,
  MoreVertical,
  Code2,
  FlaskConical,
  MessageSquare,
  Check
} from 'lucide-react';
import { Session } from '@/types';
import { formatKeyCombo } from '@/lib/utils';

export interface SidebarTool {
  icon: React.ElementType;
  label: string;
  onClick: () => void;
  rightElement?: React.ReactNode;
  iconColor?: string;
}

export interface WorkspaceSidebarProps {
  currentProject: string;
  projectNames: string[];
  onSelectProject: (name: string) => void;
  sessions: Session[];
  allSessions: Session[];
  activeSessionId: string;
  onSelectSession: (id: string) => void;
  onOpenFullChatIn?: (mode: 'chat' | 'code' | 'research', sessionId?: string) => void;
  onOpenSessionHere?: (id: string) => void;
  onDeleteSession?: (id: string) => void | Promise<void>;

  // Header button
  newTaskLabel: string;
  newTaskIcon: React.ElementType;
  newTaskIconColor?: string;
  onNewSession: () => void;

  // Nav Tools
  tools: SidebarTool[];
  
  // Lists
  sessionsTitle: string;
  emptySessionsMessage: string;
  projectActiveColor?: string;

  // Footer Avatar
  avatarColor: string;
  avatarText: string;
  avatarTextColor?: string;

  // Callbacks
  onShowToast?: (msg: string) => void;
  onOpenSettings?: () => void;
  onOpenAppGrid?: () => void;
  onSwitchMode?: (mode: 'chat' | 'code' | 'research') => void;
  isSidebarCollapsed?: boolean;
}

export const WorkspaceSidebar: React.FC<WorkspaceSidebarProps> = ({
  currentProject,
  projectNames,
  onSelectProject,
  sessions,
  allSessions,
  activeSessionId,
  onSelectSession,
  onOpenFullChatIn,
  onOpenSessionHere,
  onDeleteSession,
  newTaskLabel,
  newTaskIcon: NewTaskIcon,
  newTaskIconColor = 'text-fg-muted',
  onNewSession,
  tools,
  sessionsTitle,
  emptySessionsMessage,
  projectActiveColor = 'bg-accent',
  avatarColor,
  avatarText,
  avatarTextColor = 'text-fg-on-emphasis',

  onOpenSettings,
  isSidebarCollapsed
}) => {
  const { openSettings } = useAppContext();
  const handleOpenSettings = () => {
    setIsProfileDropdownOpen(false);
    if (onOpenSettings) {
      onOpenSettings();
    } else {
      openSettings('general');
    }
  };
  const [isProfileDropdownOpen, setIsProfileDropdownOpen] = useState(false);
  const [activeMenuSessionId, setActiveMenuSessionId] = useState<string | null>(null);
  const [showAllSessions, setShowAllSessions] = useState(false);
  const [sessionQuery, setSessionQuery] = useState('');
  const profileRef = useRef<HTMLDivElement>(null);
  const sessionMenuRef = useRef<HTMLDivElement>(null);
  const otherSessions = allSessions.filter((session) =>
    !sessions.some((visible) => visible.id === session.id) &&
    `${session.title} ${session.preview || ''}`.toLowerCase().includes(sessionQuery.trim().toLowerCase())
  );

  // Close popovers on outside click
  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      const target = e.target as Node;
      if (profileRef.current && !profileRef.current.contains(target)) {
        setIsProfileDropdownOpen(false);
      }
      if (sessionMenuRef.current && !sessionMenuRef.current.contains(target)) {
        setActiveMenuSessionId(null);
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsProfileDropdownOpen(false);
        setActiveMenuSessionId(null);
      }
    };
    document.addEventListener('mousedown', handleOutsideClick);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleOutsideClick);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  if (isSidebarCollapsed) return null;

  return (
    <aside className="w-64 bg-surface-1 border-r border-border-muted flex flex-col shrink-0 z-20 relative select-none">
      <div className="flex flex-col min-h-0 flex-1 p-3">
        {/* + New Task Button */}
        <button
          onClick={onNewSession}
          className="w-full flex items-center gap-2 px-3 py-2 rounded-lg bg-surface-2 hover:bg-surface-3 text-fg-editor text-[13px] font-medium transition mb-3 group"
        >
          <NewTaskIcon className={`w-4 h-4 ${newTaskIconColor} group-hover:text-fg-editor transition`} />
          <span>{newTaskLabel}</span>
        </button>

        {/* Tools Nav Links */}
        {tools.length > 0 && (
          <nav className="space-y-0.5 mb-3 border-b border-border-muted pb-3">
            {tools.map((tool, idx) => {
              const Icon = tool.icon;
              return (
                <button
                  key={idx}
                  onClick={tool.onClick}
                  className="w-full flex items-center justify-between px-3 py-1.5 rounded-md text-[12.5px] text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
                >
                  <div className="flex items-center gap-2.5 truncate min-w-0">
                    <Icon className={`w-4 h-4 shrink-0 ${tool.iconColor || 'text-fg-muted'}`} />
                    <span className="truncate">{tool.label}</span>
                  </div>
                  {tool.rightElement && <span className="shrink-0 ml-1">{tool.rightElement}</span>}
                </button>
              );
            })}
          </nav>
        )}

        {/* Section: Sessions */}
        <div className="flex items-center justify-between px-3 py-1 text-[11px] font-medium text-fg-subtle mb-1">
          <span>{sessionsTitle}</span>
          <button
            onClick={() => setShowAllSessions((shown) => !shown)}
            className="p-1 hover:text-fg-editor transition rounded text-fg-muted"
            title="Find conversations from other workbenches"
            aria-label="Find conversations from other workbenches"
          >
            <SlidersHorizontal className="w-3.5 h-3.5" />
          </button>
        </div>

        {showAllSessions && (
          <div className="mx-2 mb-2 rounded-lg border border-border-default bg-surface-2 p-2">
            <label className="flex items-center gap-1.5 text-fg-muted">
              <Search className="h-3.5 w-3.5 shrink-0" />
              <input autoFocus value={sessionQuery} onChange={(event) => setSessionQuery(event.target.value)} placeholder="Find another conversation" className="w-full min-w-0 bg-transparent text-xs text-fg-editor outline-none" />
            </label>
            <div className="mt-2 max-h-40 overflow-y-auto">
              {otherSessions.length === 0 ? <p className="px-1 py-2 text-[11px] text-fg-subtle">No other conversations in this project.</p> : otherSessions.map((session) => (
                <button key={session.id} type="button" onClick={() => { onOpenSessionHere?.(session.id); setShowAllSessions(false); setSessionQuery(''); }} className="block w-full truncate rounded px-1.5 py-1.5 text-left text-xs text-fg-muted hover:bg-surface-3 hover:text-fg-editor" title={`Open ${session.title} here`}>
                  {session.title} <span className="text-[10px] text-fg-subtle">· {session.pack || 'copilot'}</span>
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="flex-1 overflow-y-auto overflow-x-hidden space-y-0.5 pr-1 no-scrollbar min-h-0 text-[12.5px]">
          {sessions.length === 0 ? (
            <div className="px-3 py-4 text-center text-fg-subtle text-[12px]">
              {emptySessionsMessage}
            </div>
          ) : (
            sessions.map((s) => {
              const isActive = s.id === activeSessionId;
              const isChild = (s as any).isChild || (s as any).parentSessionId;
              const worktreeBranch = (s as any).worktreeBranch || null;
              const isRunning = s.taskStatus === 'running' || s.taskStatus === 'active';
              const needsAttention = (s as any).needsAttention || s.taskStatus === 'blocked';
              const handleDelete = (event: React.MouseEvent<HTMLButtonElement>) => {
                event.stopPropagation();
                if (!onDeleteSession) return;
                const confirmed = window.confirm(`Delete conversation "${s.title}" from this desktop view?`);
                if (!confirmed) return;
                void onDeleteSession(s.id);
              };

              const packColor =
                s.pack === 'engineering' || s.pack === 'coding' ? '#58a6ff' :
                s.pack === 'research' ? '#a371f7' :
                s.pack === 'assistant' ? '#cc785c' : '#3fb950';

              return (
                <div
                  key={s.id}
                  onClick={() => onSelectSession(s.id)}
                  className={`flex items-center justify-between px-3 py-1.5 rounded-lg cursor-pointer transition group relative ${
                    isChild ? 'ml-3 pl-4 border-l border-border-muted text-[11.5px]' : ''
                  } ${
                    isActive
                      ? 'bg-surface-2 text-fg-editor font-medium border border-border-default shadow-xs'
                      : 'text-fg-muted hover:text-fg-editor hover:bg-surface-2'
                  }`}
                >
                  <div className="flex items-center gap-2 min-w-0">
                    {/* Status Dot */}
                    <span className="relative flex h-2 w-2 shrink-0">
                      {isRunning ? (
                        <>
                          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                          <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
                        </>
                      ) : needsAttention ? (
                        <span className="relative inline-flex rounded-full h-2 w-2 bg-amber-400" />
                      ) : (
                        <span
                          className="relative inline-flex rounded-full h-1.5 w-1.5"
                          style={{ background: isActive ? packColor : 'var(--color-fg-subtle)' }}
                        />
                      )}
                    </span>

                    {isChild && <span className="text-fg-subtle text-[10px] font-mono">↳</span>}
                    <span className="truncate">{s.title}</span>
                  </div>

                  <div className="flex items-center gap-1 shrink-0 ml-2">
                    {worktreeBranch && (
                      <span className="text-[9px] font-mono px-1 py-0.2 rounded bg-canvas-inset text-blue-500 border border-border-muted opacity-80 group-hover:opacity-100">
                        {worktreeBranch}
                      </span>
                    )}
                    {s.taskStatus && s.taskStatus !== 'draft' && (
                      <span
                        className="text-[9.5px] px-1 py-0.2 rounded font-mono hidden group-hover:block"
                        style={{ background: 'var(--color-canvas-inset)', color: packColor, border: `1px solid ${packColor}33` }}
                      >
                        {s.taskStatus}
                      </span>
                    )}

                    {/* 3-dots Session Action Menu */}
                    <div className="relative" ref={activeMenuSessionId === s.id ? sessionMenuRef : undefined}>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          setActiveMenuSessionId((curr) => (curr === s.id ? null : s.id));
                        }}
                        className={`h-5 w-5 items-center justify-center rounded text-fg-subtle transition hover:bg-surface-3 hover:text-fg-editor ${
                          activeMenuSessionId === s.id ? 'flex bg-surface-3 text-fg-editor' : 'hidden group-hover:flex'
                        }`}
                        title="Session options & move workbench"
                        aria-label={`Session options for ${s.title}`}
                      >
                        <MoreVertical className="h-3.5 w-3.5" />
                      </button>

                      {/* Dropdown Menu */}
                      {activeMenuSessionId === s.id && (
                        <div
                          onClick={(e) => e.stopPropagation()}
                          className="absolute right-0 top-full mt-1 w-48 rounded-xl bg-surface-1 border border-border-default shadow-2xl p-1.5 z-50 animate-in fade-in zoom-in-95 duration-100 text-xs"
                        >
                          <div className="px-2 py-1 text-[10px] font-semibold text-fg-subtle uppercase tracking-wider">
                            Move to Workbench
                          </div>

                          <button
                            type="button"
                            onClick={() => {
                              onOpenFullChatIn?.('chat', s.id);
                              setActiveMenuSessionId(null);
                            }}
                            className="w-full flex items-center justify-between px-2 py-1.5 rounded-lg text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition text-left"
                          >
                            <div className="flex items-center gap-2">
                              <MessageSquare className="w-3.5 h-3.5 text-amber-500" />
                              <span>Copilot</span>
                            </div>
                            {s.pack === 'assistant' && <Check className="w-3 h-3 text-emerald-400" />}
                          </button>

                          <button
                            type="button"
                            onClick={() => {
                              onOpenFullChatIn?.('code', s.id);
                              setActiveMenuSessionId(null);
                            }}
                            className="w-full flex items-center justify-between px-2 py-1.5 rounded-lg text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition text-left"
                          >
                            <div className="flex items-center gap-2">
                              <Code2 className="w-3.5 h-3.5 text-blue-400" />
                              <span>Coding</span>
                            </div>
                            {(s.pack === 'engineering' || s.pack === 'coding') && <Check className="w-3 h-3 text-emerald-400" />}
                          </button>

                          <button
                            type="button"
                            onClick={() => {
                              onOpenFullChatIn?.('research', s.id);
                              setActiveMenuSessionId(null);
                            }}
                            className="w-full flex items-center justify-between px-2 py-1.5 rounded-lg text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition text-left"
                          >
                            <div className="flex items-center gap-2">
                              <FlaskConical className="w-3.5 h-3.5 text-purple-400" />
                              <span>Research</span>
                            </div>
                            {s.pack === 'research' && <Check className="w-3 h-3 text-emerald-400" />}
                          </button>

                          {onDeleteSession && (
                            <>
                              <div className="my-1 border-t border-border-muted" />
                              <button
                                type="button"
                                onClick={(e) => {
                                  setActiveMenuSessionId(null);
                                  handleDelete(e);
                                }}
                                className="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-[var(--color-task-failed)] hover:bg-[rgba(248,81,73,0.12)] transition text-left"
                              >
                                <Trash2 className="w-3.5 h-3.5" />
                                <span>Delete session</span>
                              </button>
                            </>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Project switcher */}
        {projectNames.length > 1 && (
          <div className="mt-3 pt-3 border-t border-border-muted">
            <div className="text-[10px] font-semibold text-fg-subtle px-2 mb-1 uppercase tracking-wider">Switch Project</div>
            {projectNames.map((pName) => (
              <button
                key={pName}
                onClick={() => onSelectProject(pName)}
                className={`w-full flex items-center gap-2 px-2.5 py-1 rounded-lg text-[11.5px] text-left transition ${
                  pName === currentProject
                    ? 'bg-surface-2 text-fg-editor font-medium border border-border-default'
                    : 'text-fg-muted hover:text-fg-editor hover:bg-surface-2'
                }`}
              >
                <span className={`w-1.5 h-1.5 rounded-full ${pName === currentProject ? projectActiveColor : 'bg-neutral-500'}`} />
                <span className="truncate">{pName}</span>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Bottom Profile & Settings Footer */}
      <div className="p-2.5 border-t border-border-muted flex items-center justify-between relative shrink-0">
        <div ref={profileRef} className="relative">
          <div
            onClick={() => setIsProfileDropdownOpen(!isProfileDropdownOpen)}
            className="flex items-center gap-2 cursor-pointer hover:opacity-85 transition p-1 rounded-md hover:bg-surface-2"
          >
            <div className={`w-6 h-6 rounded-full ${avatarColor} ${avatarTextColor} font-bold text-[10px] flex items-center justify-center shrink-0`}>
              {avatarText}
            </div>
            <div className="flex items-center gap-1 min-w-0">
              <span className="text-[12.5px] font-medium text-fg-editor truncate max-w-[120px]">Chí Vĩ</span>
              <ChevronDown className={`w-3 h-3 text-fg-subtle transition-transform ${isProfileDropdownOpen ? 'rotate-180' : ''}`} />
            </div>
          </div>

          {/* Profile Popover Menu */}
          {isProfileDropdownOpen && (
            <div className="absolute left-0 bottom-full mb-2 w-64 rounded-xl bg-surface-1 border border-border-default shadow-2xl p-2 z-50 animate-in fade-in zoom-in-95 duration-100">
              {/* User Header */}
              <div className="p-2 border-b border-border-muted mb-1.5">
                <div className="flex items-center gap-2.5">
                  <div className={`w-8 h-8 rounded-full ${avatarColor} ${avatarTextColor} font-bold text-xs flex items-center justify-center shrink-0`}>
                    {avatarText}
                  </div>
                  <div className="min-w-0">
                    <div className="text-xs font-semibold text-fg-editor truncate">Chí Vĩ</div>
                    <div className="text-[11px] text-fg-muted truncate">Lead Architect • Sovereign</div>
                  </div>
                </div>
              </div>

              {/* Action Links */}
              <div className="space-y-0.5 text-xs">
                <button
                  onClick={handleOpenSettings}
                  className="w-full flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
                >
                  <Settings className="w-3.5 h-3.5 text-fg-muted" />
                  <span>Preferences & Settings {formatKeyCombo({ ctrlOrCmd: true, key: ',' })}</span>
                </button>
              </div>

              {/* Footer status */}
              <div className="mt-2 pt-2 border-t border-border-muted px-2.5 py-1 flex items-center justify-between text-[10.5px] text-fg-muted">
                <span>Daemon 127.0.0.1:3000</span>
                <span className="flex items-center gap-1 text-emerald-400 font-mono">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                  online
                </span>
              </div>
            </div>
          )}
        </div>

        {/* Direct Settings Trigger */}
        <button
          onClick={handleOpenSettings}
          className="p-1.5 rounded-md text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
          title={`Preferences & Settings (${formatKeyCombo({ ctrlOrCmd: true, key: ',' })})`}
        >
          <Settings className="w-3.5 h-3.5" />
        </button>
      </div>
    </aside>
  );
};
