import React, { useState, useRef, useEffect, useMemo } from 'react';
import { useAppContext } from '@/context/AppContext';
import {
  SlidersHorizontal,
  Search,
  ChevronDown,
  Trash2,
  MessageSquare,
  ListTodo,
} from 'lucide-react';
import { Session, WorkflowCard } from '@/types';
import { sessionToWorkflowCard } from '@/lib/workflow';
import { daemonClient } from '@/api/daemon_client';
import type { ExecutionWorkspace, Task, WorkspaceNode } from '@/types/domain';
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
  tasks: Task[];
  selectedTaskId: string | null;
  onSelectTask: (id: string) => void;
  onClearTask: () => void;
  listMode: 'chats' | 'tasks';
  onChangeListMode: (mode: 'chats' | 'tasks') => void;
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
  onRequestCreateTask?: () => void;

  // Nav Tools (optional)
  tools?: SidebarTool[];

  // Lists
  sessionsTitle?: string;
  emptySessionsMessage: string;
  projectActiveColor?: string;

  // Footer Avatar (optional/deprecated - canonical entry is ActivityRail)
  avatarColor?: string;
  avatarText?: string;
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
  tasks,
  selectedTaskId,
  onSelectTask,
  onClearTask,
  listMode,
  onChangeListMode,
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
  onRequestCreateTask,
  tools: _tools = [],
  sessionsTitle,
  emptySessionsMessage,
  projectActiveColor: _projectActiveColor = 'bg-accent',
  avatarColor: _avatarColor,
  avatarText: _avatarText,
  avatarTextColor: _avatarTextColor = 'text-fg-on-emphasis',
  onShowToast,
  onOpenSettings: _onOpenSettings,
  isSidebarCollapsed,
  workspaces: propWorkspaces,
  activeWorkspaceId: propActiveWorkspaceId,
  onSelectWorkspace: propOnSelectWorkspace,
  refreshWorkspaces: propRefreshWorkspaces,
}) => {
  const {
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

  const [isNewMenuOpen, setIsNewMenuOpen] = useState(false);
  const newMenuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (newMenuRef.current && !newMenuRef.current.contains(e.target as Node)) {
        setIsNewMenuOpen(false);
      }
    };
    if (isNewMenuOpen) {
      document.addEventListener('mousedown', handleOutsideClick);
    }
    return () => {
      document.removeEventListener('mousedown', handleOutsideClick);
    };
  }, [isNewMenuOpen]);

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
          isActive: ws.id === activeWorkspaceId,
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
          isActive: ws.id === activeWorkspaceId,
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
    if (workspaces.some((workspace) => workspace.id === id)) {
      onSelectWorkspace?.(id);
      return;
    }
    const projectNode = workspaceNodes.find((node) => node.id === id && id.startsWith('proj-'));
    if (projectNode) onSelectProject(projectNode.name);
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
      const targetPath = `${repoPath.replace(/[\\/]+$/, '')}.worktrees/${sanitizedBranch}`;

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

  const [showAllSessions, setShowAllSessions] = useState(false);
  const [sessionQuery, setSessionQuery] = useState('');
  const changeListMode = (next: 'chats' | 'tasks') => {
    onChangeListMode(next);
  };

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

  // Convert sessions to unified WorkflowCards
  const workflowCards = useMemo(() => {
    const tasksById = Object.fromEntries(tasks.map((task) => [task.id, task]));
    return visibleSessions.map((s) => sessionToWorkflowCard(s, { tasksById }));
  }, [visibleSessions, tasks]);

  const visibleTasks = useMemo(() => tasks.filter((task) => {
    if (task.metadata?.kind === 'conversation_chat_turn') return false;
    const pack = String(task.contract?.pack_id || task.metadata?.pack_id || '').toLowerCase();
    if (!pack) return true;
    if (activeWorkbench === 'copilot') return pack === 'assistant' || pack === 'copilot';
    if (activeWorkbench === 'engineering') return pack === 'engineering' || pack === 'coding';
    return pack === 'research';
  }), [tasks, activeWorkbench]);

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
            ? 'bg-white/12 text-white font-medium border border-white/10 shadow-xs'
            : 'text-white/60 hover:text-white hover:bg-white/6 border border-transparent'
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
    <aside className="w-64 bg-[#212124] border-r border-white/10 flex flex-col shrink-0 z-20 relative select-none">
      <div className="flex flex-col min-h-0 flex-1 p-3">
        {/* Action button with dropdown for New Chat vs New Task */}
        <div className="relative mb-3 flex w-full items-stretch rounded-lg border border-white/10 bg-white/5 shadow-2xs">
          <button
            type="button"
            onClick={() => {
              if (listMode === 'tasks') {
                onRequestCreateTask?.();
              } else {
                changeListMode('chats');
                onClearTask();
                onNewSession();
              }
            }}
            className="flex flex-1 items-center justify-center gap-2 rounded-l-lg py-2 px-3 text-[12.5px] font-medium text-fg-editor transition hover:bg-surface-3"
            aria-label={listMode === 'tasks' ? 'Create a new task' : 'Create a new chat'}
          >
            {listMode === 'tasks' ? (
              <>
                <ListTodo className="h-4 w-4 text-workbench-accent" />
                <span>New Task</span>
              </>
            ) : (
              <>
                <NewTaskIcon className={`h-4 w-4 ${newTaskIconColor}`} />
                <span>New Chat</span>
              </>
            )}
          </button>

          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setIsNewMenuOpen(!isNewMenuOpen);
            }}
            className="flex items-center justify-center border-l border-border-muted/60 px-2 rounded-r-lg text-fg-muted hover:text-fg-editor hover:bg-surface-3 transition"
            title="Create options"
            aria-expanded={isNewMenuOpen}
          >
            <ChevronDown className="h-3.5 w-3.5" />
          </button>

          {isNewMenuOpen && (
            <div
              ref={newMenuRef}
              className="absolute left-0 top-full mt-1.5 w-full rounded-xl border border-border-default bg-surface-1 p-1.5 shadow-xl z-50 animate-in fade-in zoom-in-95 duration-100"
            >
              <button
                type="button"
                onClick={() => {
                  setIsNewMenuOpen(false);
                  changeListMode('chats');
                  onClearTask();
                  onNewSession();
                }}
                className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs text-fg-editor hover:bg-surface-2 transition text-left"
              >
                <MessageSquare className="h-4 w-4 text-sky-400 shrink-0" />
                <div>
                  <div className="font-medium">New Conversation</div>
                  <div className="text-[10px] text-fg-muted">Start an interactive chat session</div>
                </div>
              </button>

              <button
                type="button"
                onClick={() => {
                  setIsNewMenuOpen(false);
                  onRequestCreateTask?.();
                }}
                className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs text-fg-editor hover:bg-surface-2 transition text-left"
              >
                <ListTodo className="h-4 w-4 text-emerald-400 shrink-0" />
                <div>
                  <div className="font-medium">New Task</div>
                  <div className="text-[10px] text-fg-muted">Create formal task with criteria & worktree</div>
                </div>
              </button>
            </div>
          )}
        </div>
        {/* ─── Workspaces Tree (Only in Engineering lens) ─── */}
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

        {/* Chats and Tasks are separate canonical navigation projections. */}
        <div className="flex items-center justify-between px-3 py-1.5 text-[11px] font-medium text-[var(--color-fg-subtle)] mb-1">
          <div className="flex items-center rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)]/80 p-0.5" role="tablist" aria-label="History type">
            <button
              type="button"
              role="tab"
              aria-selected={listMode === 'chats'}
              onClick={() => { changeListMode('chats'); onClearTask(); }}
              className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 transition ${
                listMode === 'chats'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-xs font-semibold'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <MessageSquare className="h-3 w-3" /> Chats
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={listMode === 'tasks'}
              onClick={() => changeListMode('tasks')}
              className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 transition ${
                listMode === 'tasks'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-xs font-semibold'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <ListTodo className="h-3 w-3" /> Tasks
            </button>
          </div>
          {listMode === 'chats' && (
            <button
              onClick={() => setShowAllSessions((shown) => !shown)}
              className="p-1 hover:text-[var(--color-editor-fg)] transition rounded-md text-[var(--color-fg-subtle)] hover:bg-[var(--color-surface-2)]"
              title="Search chats across project"
              aria-label={`Search ${sessionsTitle || 'chats'} across project`}
            >
              <SlidersHorizontal className="w-3.5 h-3.5" />
            </button>
          )}
        </div>

        {showAllSessions && listMode === 'chats' && (
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

        {/* Session history and Task records are never conflated. */}
        <div className="flex-1 overflow-y-auto overflow-x-hidden space-y-2 pr-1 no-scrollbar min-h-0 text-[12.5px]">
          {listMode === 'tasks' ? (
            visibleTasks.length === 0 ? (
              <div className="px-3 py-4 text-center text-fg-subtle text-[12px]">No tasks in this workbench.</div>
            ) : (
              <div className="space-y-0.5">
                {visibleTasks.map((task) => {
                  const isActive = task.id === selectedTaskId;
                  const status = String(task.status).toLowerCase();
                  return (
                    <button key={task.id} type="button" onClick={() => onSelectTask(task.id)} className={`flex w-full items-center justify-between gap-2 rounded-md border px-2.5 py-2 text-left text-xs transition ${isActive ? 'border-border-default bg-surface-2 text-fg-editor' : 'border-transparent text-fg-muted hover:bg-surface-2/60 hover:text-fg-editor'}`} title={task.title}>
                      <span className="min-w-0 truncate">{task.title}</span>
                      <span className="shrink-0 text-[9px] uppercase tracking-wide text-fg-subtle">{status}</span>
                    </button>
                  );
                })}
              </div>
            )
          ) : workflowCards.length === 0 ? (
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

    </aside>
  );
};
