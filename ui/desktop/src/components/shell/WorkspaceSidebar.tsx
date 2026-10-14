import React, { useState, useRef, useEffect, useMemo } from 'react';
import { useAppContext } from '@/context/AppContext';
import {
  SlidersHorizontal,
  Search,
  ChevronDown,
  Settings,
  Trash2,
} from 'lucide-react';
import { Session, WorkflowCard } from '@/types';
import { sessionToWorkflowCard } from '@/lib/workflow';
import { formatKeyCombo } from '@/lib/utils';
import { daemonClient } from '@/api/daemon_client';
import type { ExecutionWorkspace, WorkspaceNode } from '@/types/domain';
import { WorkspaceTree } from './WorkspaceTree';

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
  onOpenFullChatIn?: (mode: 'copilot' | 'engineering' | 'research' | any, sessionId?: string) => void;
  onOpenSessionHere?: (id: string) => void;
  onDeleteSession?: (id: string) => void | Promise<void>;

  // Workbench
  activeWorkbench?: 'engineering' | 'research' | 'copilot';
  onSwitchWorkbench?: (wb: 'engineering' | 'research' | 'copilot') => void;
  onSwitchMode?: (mode: any) => void;

  // Header button
  newTaskLabel: string;
  newTaskIcon: React.ElementType;
  newTaskIconColor?: string;
  onNewSession: () => void;

  // Nav Tools (optional)
  tools?: SidebarTool[];

  // Lists
  sessionsTitle?: string;
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
  isSidebarCollapsed?: boolean;
  workspaces?: ExecutionWorkspace[];
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  refreshWorkspaces?: () => Promise<ExecutionWorkspace[]>;
}

export const WorkspaceSidebar: React.FC<WorkspaceSidebarProps> = ({
  currentProject,
  projectNames,
  onSelectProject,
  sessions,
  allSessions,
  activeSessionId,
  onSelectSession,
  onOpenFullChatIn: _onOpenFullChatIn,
  onOpenSessionHere,
  onDeleteSession,
  activeWorkbench = 'engineering',
  onSwitchWorkbench: _onSwitchWorkbench,
  onSwitchMode: _onSwitchMode,
  newTaskLabel: _newTaskLabel,
  newTaskIcon: NewTaskIcon,
  newTaskIconColor = 'text-fg-muted',
  onNewSession,
  tools: _tools = [],
  sessionsTitle,
  emptySessionsMessage,
  projectActiveColor: _projectActiveColor = 'bg-accent',
  avatarColor,
  avatarText,
  avatarTextColor = 'text-fg-on-emphasis',
  onShowToast,
  onOpenSettings,
  isSidebarCollapsed,
  workspaces: propWorkspaces,
  activeWorkspaceId: propActiveWorkspaceId,
  onSelectWorkspace: propOnSelectWorkspace,
  refreshWorkspaces: propRefreshWorkspaces,
}) => {
  const {
    openSettings,
    handleOpenFolderDirect,
    activeDetection,
    workspaces: ctxWorkspaces,
    activeWorkspaceId: ctxActiveWorkspaceId,
    setActiveWorkspaceId: ctxSetActiveWorkspaceId,
    refreshWorkspaces: ctxRefreshWorkspaces,
  } = useAppContext();

  const workspaces = propWorkspaces ?? ctxWorkspaces;
  const activeWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const onSelectWorkspace = propOnSelectWorkspace ?? ctxSetActiveWorkspaceId;
  const refreshWorkspaces = propRefreshWorkspaces ?? ctxRefreshWorkspaces;

  useEffect(() => {
    if (workspaces.length === 0) {
      void refreshWorkspaces();
    }
  }, [workspaces.length, refreshWorkspaces]);

  // Build WorkspaceTree nodes with stable IDs and no orphan clobbering
  const workspaceNodes: WorkspaceNode[] = useMemo(() => {
    const nodes: WorkspaceNode[] = [];
    const rootIdMap = new Map<string, WorkspaceNode>();
    const rootPathMap = new Map<string, WorkspaceNode>();
    const seenRoots = new Set<string>();
    const seenRootPaths = new Set<string>();

    // 1. Process root workspaces (physical directories / repos)
    workspaces.forEach((ws) => {
      const isGit = ws.kind?.type === 'git';
      const repoPath = isGit && ws.kind && 'repo_path' in ws.kind ? ws.kind.repo_path : undefined;
      const isWorktree =
        (isGit && !!repoPath && repoPath !== ws.path && repoPath !== '.') ||
        ws.path.includes('.worktrees') ||
        ws.path.includes('.worktree') ||
        !!ws.lineage?.parent_workspace_id;

      if (!isWorktree) {
        if (seenRoots.has(ws.id) || (ws.path && seenRootPaths.has(ws.path))) return;
        seenRoots.add(ws.id);
        if (ws.path) seenRootPaths.add(ws.path);

        const node: WorkspaceNode = {
          id: ws.id,
          name: ws.name || ws.path.split('/').pop() || ws.id,
          path: ws.path,
          kind: isGit ? 'git' : ws.kind?.type === 'remote_ssh' ? 'ssh' : 'folder',
          isActive: ws.id === activeWorkspaceId || ws.name === currentProject,
          branch: ws.active_branch || undefined,
          isDirty: ws.dirty_manifest?.is_dirty,
          dirtyCount: ws.dirty_manifest?.modified_files?.length,
          worktrees: [],
        };
        nodes.push(node);
        rootIdMap.set(ws.id, node);
        rootPathMap.set(ws.path, node);
        if (repoPath) rootPathMap.set(repoPath, node);
      }
    });

    // 2. Attach worktrees to corresponding root via lineage or repoPath
    workspaces.forEach((ws) => {
      const isGit = ws.kind?.type === 'git';
      const repoPath = isGit && ws.kind && 'repo_path' in ws.kind ? ws.kind.repo_path : undefined;
      const isWorktree =
        (isGit && !!repoPath && repoPath !== ws.path && repoPath !== '.') ||
        ws.path.includes('.worktrees') ||
        ws.path.includes('.worktree') ||
        !!ws.lineage?.parent_workspace_id;

      if (isWorktree) {
        const parent =
          (ws.lineage?.parent_workspace_id ? rootIdMap.get(ws.lineage.parent_workspace_id) : undefined) ||
          (repoPath ? rootPathMap.get(repoPath) : undefined);

        const wtNode: WorkspaceNode = {
          id: ws.id,
          name: ws.name || ws.path.split('/').pop() || ws.id,
          path: ws.path,
          kind: 'git',
          isActive: ws.id === activeWorkspaceId || ws.name === currentProject,
          branch: ws.active_branch || ws.name,
          isDirty: ws.dirty_manifest?.is_dirty,
          dirtyCount: ws.dirty_manifest?.modified_files?.length,
        };

        if (parent) {
          parent.worktrees = parent.worktrees || [];
          if (!parent.worktrees.some((w) => w.id === wtNode.id)) {
            parent.worktrees.push(wtNode);
          }
        } else {
          // Never assign orphan worktrees to nodes[0]! Keep as independent node.
          nodes.push(wtNode);
        }
      }
    });

    // 3. Fallback: Include recognized projectNames from projectData if not already in nodes
    projectNames.forEach((pName) => {
      // This is the conversation projection bucket, not an on-disk Workspace.
      if (pName === 'Custos Workspace' && !workspaces.some((workspace) => workspace.name === pName)) return;
      const alreadyPresent = nodes.some(
        (n) => n.id === pName || n.name === pName || n.worktrees?.some((w) => w.id === pName || w.name === pName)
      );
      if (!alreadyPresent) {
        nodes.push({
          id: `proj-${pName}`,
          name: pName,
          path: pName,
          kind: 'folder',
          isActive: pName === currentProject,
          worktrees: [],
        });
      }
    });

    // 4. Fallback if empty but activeDetection is available
    if (nodes.length === 0 && activeDetection) {
      nodes.push({
        id: 'active-root',
        name: activeDetection.name || currentProject || 'Workspace',
        path: activeDetection.root_path || '.',
        kind: activeDetection.is_git ? 'git' : 'folder',
        isActive: true,
        branch: activeDetection.branch || undefined,
        isDirty: activeDetection.is_dirty,
        dirtyCount: activeDetection.dirty_count,
        worktrees: [],
      });
    }

    return nodes;
  }, [workspaces, activeWorkspaceId, currentProject, projectNames, activeDetection]);

  const handleSelectWorkspaceNode = (id: string) => {
    onSelectWorkspace?.(id);
    const found =
      workspaceNodes.find((n) => n.id === id) ||
      workspaceNodes.flatMap((n) => n.worktrees || []).find((w) => w.id === id);
    if (found) {
      onSelectProject(found.name);
    }
  };

  const handleAddWorktree = async (parentId: string, branch: string) => {
    try {
      const parent = workspaces.find((w) => w.id === parentId);
      if (!parent) throw new Error('Parent workspace not found');

      const repoPath =
        parent.kind?.type === 'git' && parent.kind.repo_path
          ? parent.kind.repo_path
          : parent.path;

      const sanitizedBranch = branch.replace(/[\/\\]/g, '-');
      const targetPath = `${repoPath.replace(/\/+$/, '')}/.worktrees/${sanitizedBranch}`;

      const created = await daemonClient.createWorkspace({
        name: branch,
        kind: {
          type: 'git',
          repo_path: repoPath,
          branch,
        },
        path: targetPath,
      });
      onShowToast?.(`Worktree branch "${branch}" created.`);
      await refreshWorkspaces();
      if (created?.id) {
        onSelectWorkspace?.(created.id);
      }
    } catch (err: any) {
      onShowToast?.(`Failed to create worktree: ${err?.message || err}`);
    }
  };

  const handleRemoveWorkspaceNode = async (id: string) => {
    try {
      await daemonClient.archiveWorkspace(id);
      await refreshWorkspaces();
      onShowToast?.('Workspace removed from active roster.');
    } catch (err: any) {
      onShowToast?.(`Failed to remove workspace: ${err?.message || err}`);
      // Invariant: Do not delete local state when backend call fails!
    }
  };

  const handleOpenSettings = () => {
    setIsProfileDropdownOpen(false);
    if (onOpenSettings) {
      onOpenSettings();
    } else {
      openSettings('general');
    }
  };

  const [isProfileDropdownOpen, setIsProfileDropdownOpen] = useState(false);
  const [daemonOnline, setDaemonOnline] = useState<boolean | null>(null);
  useEffect(() => {
    if (!isProfileDropdownOpen) return;
    let mounted = true;
    setDaemonOnline(null);
    daemonClient.checkHealth()
      .then((online) => { if (mounted) setDaemonOnline(online); })
      .catch(() => { if (mounted) setDaemonOnline(false); });
    return () => { mounted = false; };
  }, [isProfileDropdownOpen]);
  const [showAllSessions, setShowAllSessions] = useState(false);
  const [sessionQuery, setSessionQuery] = useState('');
  const profileRef = useRef<HTMLDivElement>(null);

  // StudioPage supplies the canonical lens membership projection. Do not
  // filter by the Task's original pack: a session may be opened in two lenses.
  const visibleSessions = sessions;

  const otherSessions = useMemo(() => {
    return allSessions.filter(
      (session) =>
        !visibleSessions.some((visible) => visible.id === session.id) &&
        `${session.title} ${session.preview || ''}`.toLowerCase().includes(sessionQuery.trim().toLowerCase())
    );
  }, [allSessions, visibleSessions, sessionQuery]);

  const derivedSessionsTitle = useMemo(() => {
    if (sessionsTitle) return sessionsTitle;
    if (activeWorkbench === 'engineering') return 'Coding Workflows';
    if (activeWorkbench === 'research') return 'Research Studies';
    return 'Conversations';
  }, [sessionsTitle, activeWorkbench]);

  // Convert sessions to unified WorkflowCards
  const workflowCards = useMemo(() => {
    return visibleSessions.map((s) => sessionToWorkflowCard(s));
  }, [visibleSessions]);

  // Group by status (Active / Needs Input / Recent)
  const runningCards = useMemo(() => {
    return workflowCards.filter(
      (c) => c.status === 'running' || c.status === 'active' || (c.session as any)?.isRunning
    );
  }, [workflowCards]);

  const attentionCards = useMemo(() => {
    return workflowCards.filter(
      (c) =>
        ((c.session as any)?.needsAttention || c.status === 'blocked') &&
        !runningCards.some((r) => r.id === c.id)
    );
  }, [workflowCards, runningCards]);

  const recentCards = useMemo(() => {
    return workflowCards.filter(
      (c) => !runningCards.some((r) => r.id === c.id) && !attentionCards.some((a) => a.id === c.id)
    );
  }, [workflowCards, runningCards, attentionCards]);

  // Close popovers on outside click
  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      const target = e.target as Node;
      if (profileRef.current && !profileRef.current.contains(target)) {
        setIsProfileDropdownOpen(false);
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsProfileDropdownOpen(false);
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

  const renderCard = (card: WorkflowCard) => {
    const isActive = card.id === activeSessionId;
    const isRunning = card.status === 'running' || card.status === 'active' || (card.session as any)?.isRunning;
    const isBlocked = card.status === 'blocked' || (card.session as any)?.needsAttention;

    const handleDelete = (event: React.MouseEvent<HTMLButtonElement>) => {
      event.stopPropagation();
      if (!onDeleteSession) return;
      const confirmed = window.confirm(`Delete "${card.title}"?`);
      if (!confirmed) return;
      void onDeleteSession(card.id);
    };

    return (
      <div
        key={card.id}
        onClick={() => onSelectSession(card.id)}
        className={`group flex items-center justify-between px-2.5 py-1.5 rounded-md cursor-pointer transition text-xs ${
          isActive
            ? 'bg-surface-2 text-fg-editor font-medium border border-border-default'
            : 'text-fg-muted hover:text-fg-editor hover:bg-surface-2/60 border border-transparent'
        }`}
        title={card.title}
      >
        <div className="flex items-center gap-2 min-w-0 flex-1 mr-1">
          <span
            className={`w-1.5 h-1.5 rounded-full shrink-0 ${
              isRunning
                ? 'bg-emerald-500'
                : isBlocked
                  ? 'bg-amber-400'
                  : isActive
                    ? 'bg-fg-muted'
                    : 'bg-fg-subtle/50'
            }`}
          />
          <span className="truncate">{card.title}</span>
        </div>

        <span className="mr-1 shrink-0 text-[9px] uppercase tracking-wide text-fg-subtle">
          {card.session?.taskId ? 'Task' : 'Chat'}
        </span>

        {onDeleteSession && (
          <button
            type="button"
            onClick={handleDelete}
            className="opacity-0 group-hover:opacity-100 p-0.5 rounded text-fg-subtle hover:text-fg-editor hover:bg-surface-3 transition shrink-0"
            title="Delete workflow"
            aria-label={`Delete ${card.title}`}
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        )}
      </div>
    );
  };

  return (
    <aside className="w-64 bg-surface-1 border-r border-border-default flex flex-col shrink-0 z-20 relative select-none">
      <div className="flex flex-col min-h-0 flex-1 p-3">
        <button
          type="button"
          onClick={onNewSession}
          className="mb-3 flex w-full items-center justify-center gap-2 rounded-lg border border-border-muted/80 bg-surface-2 px-3 py-2 text-[12.5px] font-medium text-fg-editor transition hover:bg-surface-3"
          aria-label="Create a chat session or task"
        >
          <NewTaskIcon className={`h-4 w-4 ${newTaskIconColor}`} />
          <span>New</span>
        </button>
        {/* ─── Workspaces Tree (Only in Engineering, or in Research when projects/corpus attached) ─── */}
        {activeWorkbench === 'engineering' && (
          <WorkspaceTree
            nodes={workspaceNodes}
            activeId={activeWorkspaceId}
            onSelectNode={handleSelectWorkspaceNode}
            onAddFolder={handleOpenFolderDirect}
            onAddWorktree={handleAddWorktree}
            onRemoveNode={handleRemoveWorkspaceNode}
            activeDetection={activeDetection}
          />
        )}
        {activeWorkbench === 'research' && workspaceNodes.length > 0 && (
          <WorkspaceTree
            nodes={workspaceNodes}
            activeId={activeWorkspaceId}
            onSelectNode={handleSelectWorkspaceNode}
            onAddFolder={handleOpenFolderDirect}
            onAddWorktree={handleAddWorktree}
            onRemoveNode={handleRemoveWorkspaceNode}
            activeDetection={activeDetection}
          />
        )}

        {/* Section: Workflows Header */}
        <div className="flex items-center justify-between px-3 py-1 text-[11px] font-medium text-fg-subtle mb-1">
          <span>{derivedSessionsTitle}</span>
          <button
            onClick={() => setShowAllSessions((shown) => !shown)}
            className="p-1 hover:text-fg-editor transition rounded text-fg-muted"
            title="Search workflows across project"
            aria-label="Search workflows across project"
          >
            <SlidersHorizontal className="w-3.5 h-3.5" />
          </button>
        </div>

        {showAllSessions && (
          <div className="mx-1 mb-2 rounded-lg border border-border-default bg-surface-2 p-2">
            <label className="flex items-center gap-1.5 text-fg-muted">
              <Search className="h-3.5 w-3.5 shrink-0" />
              <input
                autoFocus
                value={sessionQuery}
                onChange={(event) => setSessionQuery(event.target.value)}
                placeholder="Search workflows..."
                className="w-full min-w-0 bg-transparent text-xs text-fg-editor outline-none"
              />
            </label>
            <div className="mt-2 max-h-40 overflow-y-auto">
              {otherSessions.length === 0 ? (
                <p className="px-1 py-2 text-[11px] text-fg-subtle">No matching workflows.</p>
              ) : (
                otherSessions.map((session) => (
                  <button
                    key={session.id}
                    type="button"
                    onClick={() => {
                      onOpenSessionHere?.(session.id);
                      setShowAllSessions(false);
                      setSessionQuery('');
                    }}
                    className="block w-full truncate rounded px-1.5 py-1.5 text-left text-xs text-fg-muted hover:bg-surface-3 hover:text-fg-editor"
                    title={`Open ${session.title} here`}
                  >
                    {session.title} <span className="text-[10px] text-fg-subtle">· {session.pack || 'copilot'}</span>
                  </button>
                ))
              )}
            </div>
          </div>
        )}

        {/* Unified Workflows List Grouped: Active / Needs Input / Recent */}
        <div className="flex-1 overflow-y-auto overflow-x-hidden space-y-2 pr-1 no-scrollbar min-h-0 text-[12.5px]">
          {workflowCards.length === 0 ? (
            <div className="px-3 py-4 text-center text-fg-subtle text-[12px]">
              {emptySessionsMessage}
            </div>
          ) : (
            <>
              {/* 1. Active & Running Tasks */}
              {runningCards.length > 0 && (
                <div>
                  <div className="flex items-center gap-1.5 px-2.5 py-1 text-[10px] font-semibold text-fg-muted uppercase tracking-wider mb-0.5">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                    <span>Active ({runningCards.length})</span>
                  </div>
                  <div className="space-y-0.5">{runningCards.map(renderCard)}</div>
                </div>
              )}

              {/* 2. Needs Input / Attention */}
              {attentionCards.length > 0 && (
                <div>
                  <div className="flex items-center gap-1.5 px-2.5 py-1 text-[10px] font-semibold text-fg-muted uppercase tracking-wider mb-0.5">
                    <span className="w-1.5 h-1.5 rounded-full bg-amber-500" />
                    <span>Needs Input ({attentionCards.length})</span>
                  </div>
                  <div className="space-y-0.5">{attentionCards.map(renderCard)}</div>
                </div>
              )}

              {/* 3. Recent Workflows */}
              <div>
                {(runningCards.length > 0 || attentionCards.length > 0) && (
                  <div className="flex items-center justify-between px-2.5 py-1 text-[10px] font-semibold text-fg-muted uppercase tracking-wider mb-0.5">
                    <span>Recent</span>
                    <span className="text-[9.5px] font-mono text-fg-subtle">{recentCards.length}</span>
                  </div>
                )}
                <div className="space-y-0.5">{recentCards.map(renderCard)}</div>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Bottom Profile & Settings Footer */}
      <div className="p-2.5 border-t border-border-default flex items-center justify-between relative shrink-0">
        <div ref={profileRef} className="relative">
          <div
            onClick={() => setIsProfileDropdownOpen(!isProfileDropdownOpen)}
            className="flex items-center gap-2 cursor-pointer hover:opacity-85 transition p-1 rounded-md hover:bg-surface-2"
          >
            <div
              className={`w-6 h-6 rounded-full ${avatarColor} ${avatarTextColor} font-bold text-[10px] flex items-center justify-center shrink-0`}
            >
              {avatarText}
            </div>
            <div className="flex items-center gap-1 min-w-0">
              <span className="text-[12.5px] font-medium text-fg-editor truncate max-w-[120px]">Custos</span>
              <ChevronDown
                className={`w-3 h-3 text-fg-subtle transition-transform ${isProfileDropdownOpen ? 'rotate-180' : ''
                  }`}
              />
            </div>
          </div>

          {/* Profile Popover Menu */}
          {isProfileDropdownOpen && (
            <div className="absolute left-0 bottom-full mb-2 w-64 rounded-xl bg-surface-1 border border-border-default shadow-2xl p-2 z-50 animate-in fade-in zoom-in-95 duration-100">
              {/* User Header */}
              <div className="p-2 border-b border-border-muted mb-1.5">
                <div className="flex items-center gap-2.5">
                  <div
                    className={`w-8 h-8 rounded-full ${avatarColor} ${avatarTextColor} font-bold text-xs flex items-center justify-center shrink-0`}
                  >
                    {avatarText}
                  </div>
                  <div className="min-w-0">
                    <div className="text-xs font-semibold text-fg-editor truncate">Custos</div>
                    <div className="text-[11px] text-fg-muted truncate">Local workspace</div>
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
                <span>Custos daemon</span>
                <span className={`flex items-center gap-1 font-mono ${daemonOnline === true ? 'text-emerald-400' : daemonOnline === false ? 'text-rose-400' : 'text-fg-muted'}`}>
                  <span className={`w-1.5 h-1.5 rounded-full ${daemonOnline === true ? 'bg-emerald-400' : daemonOnline === false ? 'bg-rose-400' : 'bg-fg-muted'}`} />
                  {daemonOnline === null ? 'checking' : daemonOnline ? 'online' : 'offline'}
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
