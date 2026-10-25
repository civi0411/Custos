import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { useAppContext } from '@/context/AppContext';
import { useScreenContext } from '@/context/ScreenContext';
import {
  FolderGit2,
  FolderPlus,
  Plus,
  RefreshCw,
  Search,
  Clock,
  HelpCircle,
  FlaskConical,
  Bot,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { WorkspaceDetectionResult } from '@/types/domain';
import { Tooltip } from '@/components/common';
import {
  normPath,
  formatRelativeTime,
  type WorkspaceSidebarProps,
} from './sidebar/types';
import { CopilotTaskHub } from './sidebar/CopilotTaskHub';
import { ResearchStudyExplorer } from './sidebar/ResearchStudyExplorer';
import { EngineeringWorkspaceTree } from './sidebar/EngineeringWorkspaceTree';
import { RecentActivityFlyout } from './sidebar/RecentActivityFlyout';
import { useProjectGroups } from './sidebar/useProjectGroups';

export * from './sidebar/types';

export const WorkspaceSidebar: React.FC<WorkspaceSidebarProps> = ({
  currentProject,
  projectNames: _projectNames,
  onSelectProject: _onSelectProject,
  sessions: _sessions = [],
  allSessions = [],
  activeSessionId,
  onSelectSession,
  tasks: propTasks,
  selectedTaskId,
  onSelectTask,
  onDeleteSession,
  onRequestCreateTask,
  onNewSession: _onNewSession,
  onShowToast,
  isSidebarCollapsed,
  workspaces: propWorkspaces,
  activeWorkspaceId: propActiveWorkspaceId,
  onSelectWorkspace: propOnSelectWorkspace,
  refreshWorkspaces: propRefreshWorkspaces,
  activeWorkbench = 'engineering',
  onOpenTab: _onOpenTab,
  onOpenSettings,
}) => {
  const {
    handleOpenFolderDirect,
    workspaces: ctxWorkspaces,
    activeWorkspaceId: ctxActiveWorkspaceId,
    setActiveWorkspaceId: ctxSetActiveWorkspaceId,
    refreshWorkspaces: ctxRefreshWorkspaces,
    activeDetection,
    tasks: ctxTasks,
    isRecentActivityOpen: isRecentOpen,
    setIsRecentActivityOpen: setIsRecentOpen,
  } = useAppContext();

  const { revealWorktreeScreen } = useScreenContext();

  const workspaces = propWorkspaces ?? ctxWorkspaces;
  const activeWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const onSelectWorkspace = propOnSelectWorkspace ?? ctxSetActiveWorkspaceId;
  const refreshWorkspaces = propRefreshWorkspaces ?? ctxRefreshWorkspaces;
  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId);
  const tasks = propTasks ?? ctxTasks ?? [];

  // Search & Filters
  const [filterQuery, setFilterQuery] = useState('');
  const [filterActiveOnly, setFilterActiveOnly] = useState(false);

  // Expand / collapse states
  const [expandedProjects, setExpandedProjects] = useState<Record<string, boolean>>({ proj_agenthub: true });
  const [expandedWorkspaces, setExpandedWorkspaces] = useState<Record<string, boolean>>({});

  // Worktree creation state
  const [creatingWorktreeFor, setCreatingWorktreeFor] = useState<string | null>(null);
  const [worktreeBranchInput, setWorktreeBranchInput] = useState('');
  const [isSubmittingWorktree, setIsSubmittingWorktree] = useState(false);

  // Live workspace detection state (ensures zero mock / zero delay)
  const effectiveDetection = activeDetection;
  const currentProjectPath =
    effectiveDetection?.root_path ||
    effectiveDetection?.path ||
    localStorage.getItem('custos_current_project_path') ||
    '.';

  // Research study detection state
  const [researchPath, setResearchPath] = useState<string | null>(() => {
    try {
      return localStorage.getItem('custos_project_research_path');
    } catch {
      return null;
    }
  });
  const [researchDetection, setResearchDetection] = useState<WorkspaceDetectionResult | null>(null);

  useEffect(() => {
    if (!researchPath) {
      setResearchDetection(null);
      return;
    }
    let isMounted = true;
    daemonClient
      .detectWorkspace(researchPath)
      .then((det) => {
        if (isMounted && det) {
          setResearchDetection(det);
        }
      })
      .catch((err) => {
        console.warn('[WorkspaceSidebar] Research study detection failed:', err);
      });
    return () => {
      isMounted = false;
    };
  }, [researchPath]);

  const handleOpenStudyFolder = async () => {
    try {
      const chosen = await daemonClient.pickFolder();
      if (!chosen) return;
      const det = await daemonClient.detectWorkspace(chosen);
      const studyName = det?.name || chosen.split('/').filter(Boolean).pop() || 'Study';
      localStorage.setItem('custos_project_research_path', chosen);
      localStorage.setItem('custos_project_research_name', studyName);
      setResearchPath(chosen);
      setResearchDetection(det);
      const ws = await daemonClient.createWorkspace({
        name: studyName,
        path: chosen,
        kind: { type: 'folder', path: chosen },
        metadata: { pack: 'research' },
      });
      await refreshWorkspaces();
      if (ws?.id) onSelectWorkspace(ws.id);
      onShowToast?.(`Opened research study: ${studyName}`);
    } catch (err: any) {
      onShowToast?.(`Failed to open study: ${err?.message || err}`);
    }
  };

  const handleOpenFolder = activeWorkbench === 'research' ? handleOpenStudyFolder : handleOpenFolderDirect;

  useEffect(() => {
    if (workspaces.length === 0) {
      void refreshWorkspaces();
    }
  }, [workspaces.length, refreshWorkspaces]);

  const getAgentActivity = useCallback((wsId: string, wsPath: string) => {
    const matched = allSessions.filter((s) => s.workspaceId === wsId || s.workspaceId === wsPath);
    if (matched.length === 0) return null;
    const latest = matched[0];
    const isRunning =
      latest.taskStatus === 'running' ||
      latest.messages?.some((m) => m.runStatus === 'running' || m.runStatus === 'queued');
    const isBlocked =
      latest.taskStatus === 'blocked' || latest.messages?.some((m) => m.runStatus === 'blocked');
    return {
      status: isRunning ? ('running' as const) : isBlocked ? ('blocked' as const) : ('completed' as const),
      title: latest.title || latest.preview || 'Agent task',
      model: latest.model || undefined,
      timeAgo: latest.time || undefined,
    };
  }, [allSessions]);

  // Build the complete 3-tier hierarchy: Projects -> Workspaces (repos) -> Worktrees
  const projectGroups = useProjectGroups({
    currentProject,
    effectiveDetection,
    currentProjectPath,
    workspaces,
    activeWorkspaceId,
    activeWorkspace,
    allSessions,
    filterQuery,
    filterActiveOnly,
    getAgentActivity,
  });

  const workbenchConfig = useMemo(() => {
    switch (activeWorkbench) {
      case 'research':
        return {
          title: 'Research Study',
          icon: FlaskConical,
          iconColor: 'text-amber-400',
          badge: 'STUDY',
          badgeClass: 'bg-amber-400/10 text-amber-300 border-amber-400/20',
          searchPlaceholder: 'Search studies, findings, claims...',
          allLabel: 'All Studies',
          allCount: 1,
          activeLabel: 'Active',
          activeCount: researchPath ? 1 : 0,
          footerStatus: researchDetection?.name || 'Research Study Ready',
        };
      case 'copilot':
        return {
          title: 'Autonomous Tasks',
          icon: Bot,
          iconColor: 'text-emerald-400',
          badge: 'SUPERVISOR',
          badgeClass: 'bg-emerald-400/10 text-emerald-300 border-emerald-400/20',
          searchPlaceholder: 'Search tasks, operations...',
          allLabel: 'All Tasks',
          allCount: tasks.length,
          activeLabel: 'Action Needed',
          activeCount: tasks.filter((t) => t.status === 'Blocked' || t.metadata?.approval_required === 'true').length,
          footerStatus: 'Copilot Supervisor Ready',
        };
      case 'engineering':
      default: {
        const totalWorktrees = projectGroups.reduce(
          (acc, g) => acc + g.workspaces.reduce((wAcc, ws) => wAcc + ws.worktrees.length, 0),
          0
        );
        const activeWorktrees = projectGroups.reduce(
          (acc, g) =>
            acc +
            g.workspaces.reduce(
              (wAcc, ws) => wAcc + ws.worktrees.filter((wt) => wt.isDirty || wt.isRunning || wt.isBlocked).length,
              0
            ),
          0
        );
        return {
          title: 'Project Workspaces',
          icon: FolderGit2,
          iconColor: 'text-white/80',
          badge: projectGroups[0]?.isMultiRepo ? 'MULTI-REPO' : 'MONOREPO',
          badgeClass: 'bg-white/10 text-white/80 border-white/20',
          searchPlaceholder: 'Search workspaces, branches...',
          allLabel: 'All Worktrees',
          allCount: totalWorktrees,
          activeLabel: 'Active Worktrees',
          activeCount: activeWorktrees,
          footerStatus: activeWorkspace ? `${activeWorkspace.name} · Ready` : 'Ready',
        };
      }
    }
  }, [activeWorkbench, projectGroups, tasks, researchPath, researchDetection, activeWorkspace]);

  const hasActiveRecent = useMemo(
    () =>
      allSessions.some(
        (s) =>
          s.taskStatus === 'running' ||
          s.taskStatus === 'blocked' ||
          s.messages?.some((m) => m.runStatus === 'running' || m.runStatus === 'blocked')
      ) ||
      tasks.some(
        (t) =>
          t.status === 'running' ||
          t.status === 'Active' ||
          t.status === 'Blocked' ||
          String(t.status).toLowerCase() === 'running' ||
          String(t.status).toLowerCase() === 'blocked' ||
          t.metadata?.approval_required === 'true'
      ),
    [allSessions, tasks]
  );

  const handleSelectWorkspaceItem = async (wsId: string, wsPath: string) => {
    let ws = workspaces.find((w) => w.id === wsId || normPath(w.path) === normPath(wsPath));
    if (!ws) {
      try {
        ws = await daemonClient.createWorkspace({
          name: wsPath.split('/').filter(Boolean).pop() || 'worktree',
          path: wsPath,
          kind: { type: 'git', repo_path: wsPath, branch: 'main' },
          metadata: { project_id: currentProject },
        });
        await refreshWorkspaces();
      } catch (err) {
        console.warn('[WorkspaceSidebar] auto create workspace failed:', err);
      }
    }
    const resolvedId = ws?.id || wsId;
    onSelectWorkspace(resolvedId);

    revealWorktreeScreen({
      id: resolvedId,
      name: ws?.name || wsPath.split('/').filter(Boolean).pop() || 'Worktree',
      path: wsPath,
    });
  };

  const handleAddWorktree = async (wsId: string, branchName: string) => {
    const branch = branchName.trim();
    if (!branch) return;
    const parentWs = workspaces.find((w) => w.id === wsId);
    const basePath = parentWs?.path || currentProjectPath;
    setIsSubmittingWorktree(true);
    try {
      const created = await daemonClient.createWorkspace({
        name: branch,
        path: basePath,
        kind: { type: 'git', repo_path: basePath, branch },
        metadata: { parent_repo: basePath, branch, pack: 'engineering' },
      });
      await refreshWorkspaces();
      setCreatingWorktreeFor(null);
      setWorktreeBranchInput('');
      if (created?.path) {
        await handleSelectWorkspaceItem(created.id, created.path);
        onShowToast?.(`Created worktree on branch "${branch}".`);
      }
    } catch (err: any) {
      onShowToast?.(`Failed to create worktree: ${err?.message || err}`);
    } finally {
      setIsSubmittingWorktree(false);
    }
  };

  const handleAddRepoToProject = async () => {
    try {
      const chosen = await daemonClient.pickFolder();
      if (!chosen) return;
      const det = await daemonClient.detectWorkspace(chosen);
      const repoName = det?.name || chosen.split('/').filter(Boolean).pop() || 'Repository';
      const created = await daemonClient.createWorkspace({
        name: repoName,
        path: chosen,
        kind: { type: 'git', repo_path: chosen, branch: 'main' },
        metadata: { project_id: currentProject },
      });
      await refreshWorkspaces();
      if (created?.id) {
        onSelectWorkspace(created.id);
        setExpandedWorkspaces((prev) => ({ ...prev, [created.id]: true }));
        onShowToast?.(`Added and activated repository "${repoName}".`);
      }
    } catch (err: any) {
      onShowToast?.(`Failed to add repository: ${err?.message || err}`);
    }
  };

  if (isSidebarCollapsed) return null;

  return (
    <aside className="w-64 flex flex-col h-full bg-[#141518] border-r border-white/6 select-none shrink-0 overflow-hidden text-xs font-sans relative">
      {/* ── 1. HEADER: ADAPTIVE TO ACTIVE WORKBENCH ── */}
      <div className="p-2.5 pb-2 border-b border-white/6 shrink-0 space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1.5 text-white/90 font-semibold text-[12px] tracking-wide min-w-0">
            <workbenchConfig.icon className={`size-4 ${workbenchConfig.iconColor} shrink-0`} />
            <span className="truncate">{workbenchConfig.title}</span>
            <span className={`font-mono text-[9px] px-1.5 py-0.2 rounded border shrink-0 ${workbenchConfig.badgeClass}`}>
              {workbenchConfig.badge}
            </span>
          </div>

          <div className="flex items-center gap-0.5 text-white/40">
            {activeWorkbench === 'copilot' && onRequestCreateTask && (
              <Tooltip content="Create New Task (⌘N)" position="bottom">
                <button
                  type="button"
                  onClick={onRequestCreateTask}
                  className="p-1 hover:text-white hover:bg-white/8 rounded transition cursor-pointer text-white/60"
                  aria-label="New Task"
                >
                  <Plus className="size-3.5" />
                </button>
              </Tooltip>
            )}
            <Tooltip content="Recent Activity (Chats & Tasks)" position="bottom">
              <button
                type="button"
                onClick={() => setIsRecentOpen(!isRecentOpen)}
                className={`p-1 rounded transition cursor-pointer relative ${
                  isRecentOpen
                    ? 'text-white bg-white/12'
                    : 'text-white/50 hover:text-white hover:bg-white/8'
                }`}
                aria-label="Recent activity"
              >
                <Clock className="size-3.5" />
                {hasActiveRecent && (
                  <span className="absolute top-0.5 right-0.5 size-1.5 rounded-full bg-emerald-400 animate-pulse" />
                )}
              </button>
            </Tooltip>
            {activeWorkbench !== 'copilot' && (
              <Tooltip
                content={activeWorkbench === 'research' ? 'Open Study Folder (⌘O)' : 'Open Project Folder (⌘O)'}
                position="bottom"
              >
                <button
                  type="button"
                  onClick={handleOpenFolder}
                  className="p-1 hover:text-white hover:bg-white/8 rounded transition cursor-pointer"
                  aria-label="Open Folder"
                >
                  <FolderPlus className="size-3.5" />
                </button>
              </Tooltip>
            )}
            <Tooltip
              content={
                activeWorkbench === 'copilot'
                  ? 'Refresh Tasks'
                  : activeWorkbench === 'research'
                    ? 'Refresh Studies & Datasets'
                    : 'Refresh Git Status'
              }
              position="bottom"
            >
              <button
                type="button"
                onClick={() => {
                  void refreshWorkspaces();
                  onShowToast?.(
                    activeWorkbench === 'copilot'
                      ? 'Refreshed task registry.'
                      : activeWorkbench === 'research'
                        ? 'Refreshed studies and datasets.'
                        : 'Refreshed workspace and worktree registry.'
                  );
                }}
                className="p-1 hover:text-white hover:bg-white/8 rounded transition cursor-pointer"
                aria-label="Refresh"
              >
                <RefreshCw className="size-3.5" />
              </button>
            </Tooltip>
          </div>
        </div>

        {/* Search Bar & Instant Filter */}
        <div className="relative flex items-center">
          <Search className="size-3.5 text-white/35 absolute left-2 pointer-events-none" />
          <input
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder={workbenchConfig.searchPlaceholder}
            className="w-full bg-white/[0.04] hover:bg-white/[0.06] focus:bg-white/[0.08] border border-white/8 focus:border-white/20 rounded-md pl-7 pr-7 py-1 text-[11px] text-white placeholder-white/30 outline-none transition font-sans"
          />
          {filterQuery && (
            <button
              type="button"
              onClick={() => setFilterQuery('')}
              className="absolute right-2 text-white/40 hover:text-white text-[10px] cursor-pointer"
              aria-label="Clear filter"
            >
              ✕
            </button>
          )}
        </div>

        {/* Filter Badges */}
        <div className="flex items-center gap-1 pt-0.5 text-[10.5px]">
          <button
            type="button"
            onClick={() => setFilterActiveOnly(false)}
            className={`px-2 py-0.5 rounded transition cursor-pointer ${
              !filterActiveOnly
                ? 'bg-white/10 text-white font-medium'
                : 'text-white/40 hover:text-white/70'
            }`}
          >
            {workbenchConfig.allLabel} ({workbenchConfig.allCount})
          </button>
          <button
            type="button"
            onClick={() => setFilterActiveOnly(true)}
            className={`px-2 py-0.5 rounded transition flex items-center gap-1 cursor-pointer ${
              filterActiveOnly
                ? 'bg-white/10 text-white font-medium'
                : 'text-white/40 hover:text-white/70'
            }`}
          >
            <span className="size-1.5 rounded-full bg-emerald-400" />
            <span>
              {workbenchConfig.activeLabel} ({workbenchConfig.activeCount})
            </span>
          </button>
        </div>
      </div>

      {/* ── 2. MAIN CONTENT (ADAPTIVE TO ACTIVE WORKBENCH) ── */}
      <div className="flex-1 min-h-0 overflow-y-auto no-scrollbar p-2 space-y-2">
        {activeWorkbench === 'copilot' ? (
          <CopilotTaskHub
            tasks={tasks}
            selectedTaskId={selectedTaskId}
            onSelectTask={onSelectTask}
            onRequestCreateTask={onRequestCreateTask}
            filterActiveOnly={filterActiveOnly}
            formatRelativeTime={formatRelativeTime}
          />
        ) : activeWorkbench === 'research' ? (
          <ResearchStudyExplorer
            researchPath={researchPath}
            researchDetection={researchDetection}
            handleOpenStudyFolder={handleOpenStudyFolder}
          />
        ) : (
          <EngineeringWorkspaceTree
            projectGroups={projectGroups}
            expandedProjects={expandedProjects}
            setExpandedProjects={setExpandedProjects}
            expandedWorkspaces={expandedWorkspaces}
            setExpandedWorkspaces={setExpandedWorkspaces}
            activeWorkspaceId={activeWorkspaceId}
            activeWorkspace={activeWorkspace}
            creatingWorktreeFor={creatingWorktreeFor}
            setCreatingWorktreeFor={setCreatingWorktreeFor}
            worktreeBranchInput={worktreeBranchInput}
            setWorktreeBranchInput={setWorktreeBranchInput}
            isSubmittingWorktree={isSubmittingWorktree}
            handleOpenFolderDirect={handleOpenFolderDirect}
            handleAddRepoToProject={handleAddRepoToProject}
            handleSelectWorkspaceItem={handleSelectWorkspaceItem}
            handleAddWorktree={handleAddWorktree}
            normPath={normPath}
          />
        )}
      </div>

      {/* ── RECENT ACTIVITY OVERLAY ── */}
      <RecentActivityFlyout
        isOpen={isRecentOpen}
        onClose={() => setIsRecentOpen(false)}
        allSessions={allSessions}
        tasks={tasks}
        formatRelativeTime={formatRelativeTime}
        activeSessionId={activeSessionId}
        onSelectSession={onSelectSession}
        onSelectTask={onSelectTask}
        onDeleteSession={onDeleteSession}
      />

      {/* ── 3. FOOTER: ACTIVE CONTEXT & SYSTEM STATUS ── */}
      <div className="h-9 px-3 border-t border-white/6 flex items-center justify-between text-white/40 text-xs shrink-0 bg-[#101113]">
        <div className="flex items-center gap-1.5 truncate min-w-0 mr-2">
          <span className="size-1.5 rounded-full bg-emerald-400 shrink-0" />
          <span className="text-[10px] font-mono truncate text-white/60">
            {workbenchConfig.footerStatus}
          </span>
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <Tooltip content="Keyboard Shortcuts" position="top">
            <button
              type="button"
              onClick={() => (onOpenSettings ? onOpenSettings('shortcuts') : undefined)}
              className="p-1 hover:text-white hover:bg-white/8 rounded transition cursor-pointer"
              aria-label="Shortcuts"
            >
              <HelpCircle className="size-3.5" />
            </button>
          </Tooltip>
        </div>
      </div>
    </aside>
  );
};
