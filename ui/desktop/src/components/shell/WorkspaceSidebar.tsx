import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { useAppContext } from '@/context/AppContext';
import { useScreenContext } from '@/context/ScreenContext';
import {
  Boxes,
  FolderOpen,
  FolderPlus,
  GitBranch,
  Plus,
  ChevronDown,
  ChevronRight,
  ChevronUp,
  Search,
  RefreshCw,
  MessageSquare,
  HelpCircle,
  Trash2,
  Clock,
  CheckSquare,
  X,
  FlaskConical,
  Bot,
  Folder,
  FolderTree,
  FileCode,
  FileText,
  File,
} from 'lucide-react';

function getSidebarFileIcon(name: string) {
  const ext = name.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'ts':
    case 'tsx':
    case 'js':
    case 'jsx':
    case 'rs':
    case 'py':
    case 'go':
    case 'c':
    case 'cpp':
      return <FileCode className="size-3.5 text-sky-400 shrink-0" />;
    case 'json':
    case 'yaml':
    case 'yml':
    case 'toml':
      return <FileCode className="size-3.5 text-amber-300 shrink-0" />;
    case 'md':
    case 'txt':
      return <FileText className="size-3.5 text-white/50 shrink-0" />;
    default:
      return <File className="size-3.5 text-white/40 shrink-0" />;
  }
}
import type { Session } from '@/types';
import { sessionToWorkflowCard } from '@/lib/workflow';
import { daemonClient } from '@/api/daemon_client';
import type {
  ExecutionWorkspace,
  Task,
  WorkspaceDetectionResult,
  WorkspaceFileTree,
  WorkspaceFileEntry,
  WorkspaceDiffSummary,
  WorkspaceDiffEntry,
} from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';
import { Tooltip } from '@/components/common';

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
  sessions?: Session[];
  allSessions?: Session[];
  activeSessionId?: string;
  onSelectSession?: (id: string) => void;
  tasks?: Task[];
  selectedTaskId?: string | null;
  onSelectTask?: (id: string) => void;
  onClearTask?: () => void;
  listMode?: 'chats' | 'tasks';
  onChangeListMode?: (mode: 'chats' | 'tasks') => void;
  onOpenFullChatIn?: (mode: any, sessionId?: string) => void;
  onOpenSessionHere?: (id: string) => void;
  onDeleteSession?: (id: string) => void | Promise<void>;

  // Workbench
  activeWorkbench?: 'engineering' | 'research' | 'copilot' | string;
  onSwitchWorkbench?: (wb: any) => void;
  onSwitchMode?: (mode: any) => void;

  // Header button
  newTaskLabel?: string;
  newTaskIcon?: React.ElementType;
  newTaskIconColor?: string;
  onNewSession?: () => void;
  onRequestCreateTask?: () => void;

  // Nav Tools (optional)
  tools?: SidebarTool[];

  // Lists
  sessionsTitle?: string;
  emptySessionsMessage?: string;
  projectActiveColor?: string;

  // Footer Avatar
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
  onOpenFile?: (path: string, name: string) => void;
  onOpenTab?: (id: OrcaTabId, customUrl?: string, customTitle?: string) => void;
  activeFilePath?: string;
}

export interface WorktreeItem {
  id: string;
  name: string;
  branch: string;
  path: string;
  isMain: boolean;
  isDirty: boolean;
  dirtyCount: number;
  isRunning: boolean;
  isBlocked: boolean;
  sessionCount: number;
  agentActivity?: {
    status: 'running' | 'blocked' | 'completed';
    title: string;
    model?: string;
    timeAgo?: string;
  } | null;
}

export interface WorkspaceItem {
  id: string;
  name: string;
  path: string;
  isGit: boolean;
  githubOwner?: string | null;
  githubRepo?: string | null;
  githubAvatarUrl?: string | null;
  worktrees: WorktreeItem[];
}

export interface ProjectGroupItem {
  id: string;
  name: string;
  path: string;
  isMultiRepo: boolean;
  workspaces: WorkspaceItem[];
}

export const WorkspaceSidebar: React.FC<WorkspaceSidebarProps> = ({
  currentProject,
  projectNames: _projectNames,
  onSelectProject: _onSelectProject,
  sessions = [],
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
  onOpenFile,
  onOpenTab,
  activeFilePath,
  activeWorkbench = 'engineering',
}) => {
  const {
    handleOpenFolderDirect,
    workspaces: ctxWorkspaces,
    activeWorkspaceId: ctxActiveWorkspaceId,
    setActiveWorkspaceId: ctxSetActiveWorkspaceId,
    refreshWorkspaces: ctxRefreshWorkspaces,
    setIsInspectorOpen,
    activeDetection,
    tasks: ctxTasks,
  } = useAppContext();

  const { revealWorktreeScreen } = useScreenContext();

  const workspaces = propWorkspaces ?? ctxWorkspaces;
  const activeWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const onSelectWorkspace = propOnSelectWorkspace ?? ctxSetActiveWorkspaceId;
  const refreshWorkspaces = propRefreshWorkspaces ?? ctxRefreshWorkspaces;
  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId);
  const tasks = propTasks ?? ctxTasks ?? [];

  // Recent Activity flyout state & filters
  const [isRecentOpen, setIsRecentOpen] = useState(false);
  const [recentFilter, setRecentFilter] = useState<'all' | 'chats' | 'tasks'>('all');
  const [recentSearch, setRecentSearch] = useState('');

  // Active Workspace File Tree (Left Sidebar Explorer)
  const [fileTree, setFileTree] = useState<WorkspaceFileTree | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [filesLoading, setFilesLoading] = useState(false);
  const [diffSummary, setDiffSummary] = useState<WorkspaceDiffSummary | null>(null);

  const loadFileTree = useCallback(async () => {
    if (!activeWorkspaceId) {
      setFileTree(null);
      setDiffSummary(null);
      return;
    }
    try {
      setFilesLoading(true);
      const [tree, diff] = await Promise.all([
        daemonClient.getWorkspaceFileTree(activeWorkspaceId),
        daemonClient.getWorkspaceDiff(activeWorkspaceId).catch(() => null),
      ]);
      setFileTree(tree);
      setDiffSummary(diff);
      if (tree?.entries) {
        const topLevelDirs = new Set<string>();
        tree.entries.forEach((f) => {
          if (f.is_dir && f.path.split('/').length <= 1) {
            topLevelDirs.add(f.path);
          }
        });
        setExpandedFolders((prev) => (prev.size > 0 ? prev : topLevelDirs));
      }
    } catch {
      setFileTree(null);
      setDiffSummary(null);
    } finally {
      setFilesLoading(false);
    }
  }, [activeWorkspaceId]);

  useEffect(() => {
    if (activeWorkbench === 'engineering' && activeWorkspaceId) {
      void loadFileTree();
    }
  }, [activeWorkbench, activeWorkspaceId, loadFileTree]);

  const toggleFolder = useCallback((path: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }, []);

  const diffMap = useMemo(() => {
    const map = new Map<string, WorkspaceDiffEntry>();
    if (diffSummary?.files) {
      diffSummary.files.forEach((f) => map.set(f.path, f));
    }
    return map;
  }, [diffSummary]);

  const isNodeVisible = useCallback((node: WorkspaceFileEntry, expanded: Set<string>) => {
    const parts = node.path.split('/');
    if (parts.length <= 1) return true;
    for (let i = 1; i < parts.length; i++) {
      const parent = parts.slice(0, i).join('/');
      if (!expanded.has(parent)) return false;
    }
    return true;
  }, []);

  // Search & Filters
  const [filterQuery, setFilterQuery] = useState('');
  const [filterActiveOnly, setFilterActiveOnly] = useState(false);

  const fileEntriesForTree = useMemo(() => {
    if (!fileTree?.entries) return [];
    if (!filterQuery.trim()) return fileTree.entries;
    const q = filterQuery.toLowerCase();
    return fileTree.entries.filter(
      (node) => node.name.toLowerCase().includes(q) || node.path.toLowerCase().includes(q)
    );
  }, [fileTree, filterQuery]);

  // Synchronize sidebar when switched externally (e.g. ActivityRail)
  useEffect(() => {
    const handleSwitchTab = (e: Event) => {
      const customEvent = e as CustomEvent<'worktrees' | 'sessions' | 'chats' | 'tasks'>;
      if (customEvent.detail) {
        if (customEvent.detail === 'worktrees') {
          setIsRecentOpen(false);
        } else if (customEvent.detail === 'chats') {
          setIsRecentOpen(true);
          setRecentFilter('chats');
        } else if (customEvent.detail === 'tasks') {
          setIsRecentOpen(true);
          setRecentFilter('tasks');
        } else {
          setIsRecentOpen(true);
          setRecentFilter('all');
        }
      }
    };
    window.addEventListener('custos:switch-sidebar-tab', handleSwitchTab);
    return () => window.removeEventListener('custos:switch-sidebar-tab', handleSwitchTab);
  }, []);

  // Worktree creation state
  const [creatingWorktreeFor, setCreatingWorktreeFor] = useState<string | null>(null);
  const [worktreeBranchInput, setWorktreeBranchInput] = useState('');
  const [isSubmittingWorktree, setIsSubmittingWorktree] = useState(false);

  // Live workspace detection state (ensures zero mock / zero delay)
  const [detection, setDetection] = useState<WorkspaceDetectionResult | null>(activeDetection);

  useEffect(() => {
    if (activeDetection) setDetection(activeDetection);
  }, [activeDetection]);

  useEffect(() => {
    let isMounted = true;
    const loadDetection = async () => {
      try {
        const currentPath =
          localStorage.getItem('custos_current_project_path') || '/Users/mac/Project/AgentHub';
        const det = await daemonClient.detectWorkspace(currentPath);
        if (isMounted && det) {
          setDetection(det);
        }
      } catch (err) {
        console.warn('[WorkspaceSidebar] Live detection failed:', err);
      }
    };
    void loadDetection();
    return () => {
      isMounted = false;
    };
  }, []);

  // Research study detection state (strictly isolated from engineering code projects)
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

  // Extract latest agent activity for a worktree from real sessions
  const getAgentActivity = (wsId: string, wsPath: string) => {
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
  };

  // Build the complete 3-tier hierarchy: Projects -> Workspaces (repos) -> Worktrees
  const projectGroups = useMemo<ProjectGroupItem[]>(() => {
    const effectiveDetection = detection || activeDetection;
    const projectName = currentProject || effectiveDetection?.name || 'AgentHub';
    const projectPath =
      effectiveDetection?.root_path ||
      effectiveDetection?.path ||
      localStorage.getItem('custos_current_project_path') ||
      '/Users/mac/Project/AgentHub';
    const isMultiRepo = Boolean(
      effectiveDetection?.sub_repositories && effectiveDetection.sub_repositories.length > 0
    );

    const projectWorkspaces: WorkspaceItem[] = [];
    const otherWorkspaces: WorkspaceItem[] = [];

    // Map existing daemon workspaces by path
    const wsByPath = new Map<string, ExecutionWorkspace>();
    workspaces.forEach((ws) => {
      if (ws.status !== 'archived') wsByPath.set(ws.path, ws);
    });

    // 1. If multi-repo project detected (e.g. AgentHub with child repos)
    if (effectiveDetection?.sub_repositories && effectiveDetection.sub_repositories.length > 0) {
      effectiveDetection.sub_repositories.forEach((sub: any) => {
        const matchedWs = wsByPath.get(sub.path);
        const wsId = matchedWs?.id || `sub_${sub.name}`;
        const mainBranch = sub.branch || matchedWs?.active_branch || 'main';
        const daemonDirtyCount = matchedWs?.dirty_manifest?.modified_files?.length ?? 0;
        const dirtyCount = Math.max(sub.dirty_count ?? 0, daemonDirtyCount);
        const isDirty = sub.is_dirty || Boolean(matchedWs?.dirty_manifest?.is_dirty) || dirtyCount > 0;

        const worktrees: WorktreeItem[] = [];

        // 1a. Primary worktree (main checkout)
        const primarySessions = allSessions.filter((s) => s.workspaceId === wsId || s.workspaceId === sub.path);
        worktrees.push({
          id: wsId,
          name: mainBranch,
          branch: mainBranch,
          path: sub.path,
          isMain: true,
          isDirty,
          dirtyCount,
          isRunning: primarySessions.some((s) => s.taskStatus === 'running'),
          isBlocked: primarySessions.some((s) => s.taskStatus === 'blocked'),
          sessionCount: primarySessions.length,
          agentActivity: getAgentActivity(wsId, sub.path),
        });

        // 1b. Secondary worktrees (from detection or daemon)
        if (sub.worktrees) {
          sub.worktrees.forEach((wt: any) => {
            if (!wt.is_main) {
              const wtSessions = allSessions.filter((s) => s.workspaceId === wt.path || s.workspaceId === `wt_${wt.path}`);
              worktrees.push({
                id: `wt_${wt.path}`,
                name: wt.branch || wt.path.split('/').pop() || 'worktree',
                branch: wt.branch || 'worktree',
                path: wt.path,
                isMain: false,
                isDirty: false,
                dirtyCount: 0,
                isRunning: wtSessions.some((s) => s.taskStatus === 'running'),
                isBlocked: wtSessions.some((s) => s.taskStatus === 'blocked'),
                sessionCount: wtSessions.length,
                agentActivity: getAgentActivity(`wt_${wt.path}`, wt.path),
              });
            }
          });
        }

        // Also attach daemon worktrees linked to this repository
        workspaces.forEach((w) => {
          if (w.lineage?.parent_workspace_id === wsId && !worktrees.some((existing) => existing.path === w.path)) {
            const wSessions = allSessions.filter((s) => s.workspaceId === w.id);
            worktrees.push({
              id: w.id,
              name: w.name || w.path.split('/').pop() || 'worktree',
              branch: w.active_branch || 'worktree',
              path: w.path,
              isMain: false,
              isDirty: Boolean(w.dirty_manifest?.is_dirty),
              dirtyCount: w.dirty_manifest?.modified_files?.length || 0,
              isRunning: wSessions.some((s) => s.taskStatus === 'running'),
              isBlocked: wSessions.some((s) => s.taskStatus === 'blocked'),
              sessionCount: wSessions.length,
              agentActivity: getAgentActivity(w.id, w.path),
            });
          }
        });

        projectWorkspaces.push({
          id: wsId,
          name: sub.name,
          path: sub.path,
          isGit: sub.is_git,
          githubOwner: sub.github_owner,
          githubRepo: sub.github_repo,
          githubAvatarUrl: sub.github_avatar_url,
          worktrees,
        });
      });
    } else if (effectiveDetection?.is_git) {
      // 2. Single-repository Project: the repo is the Workspace, its worktrees are the child items
      const repoPath = effectiveDetection.root_path || effectiveDetection.path;
      const repoName = effectiveDetection.name || repoPath.split('/').pop() || 'Repository';
      const mainBranch = effectiveDetection.branch || 'main';
      const dirtyCount = effectiveDetection.dirty_count || 0;
      const isDirty = effectiveDetection.is_dirty || dirtyCount > 0;

      const worktrees: WorktreeItem[] = [];
      const primarySessions = allSessions.filter((s) => s.workspaceId === effectiveDetection.path);

      worktrees.push({
        id: activeWorkspaceId || 'primary_ws',
        name: mainBranch,
        branch: mainBranch,
        path: repoPath,
        isMain: true,
        isDirty,
        dirtyCount,
        isRunning: primarySessions.some((s) => s.taskStatus === 'running'),
        isBlocked: primarySessions.some((s) => s.taskStatus === 'blocked'),
        sessionCount: primarySessions.length,
        agentActivity: getAgentActivity(activeWorkspaceId || '', repoPath),
      });

      if (effectiveDetection.worktrees) {
        effectiveDetection.worktrees.forEach((wt: any) => {
          if (!wt.is_main) {
            worktrees.push({
              id: `wt_${wt.path}`,
              name: wt.branch || wt.path.split('/').pop() || 'worktree',
              branch: wt.branch || 'worktree',
              path: wt.path,
              isMain: false,
              isDirty: false,
              dirtyCount: 0,
              isRunning: false,
              isBlocked: false,
              sessionCount: 0,
              agentActivity: null,
            });
          }
        });
      }

      projectWorkspaces.push({
        id: activeWorkspaceId || 'primary_repo',
        name: repoName,
        path: repoPath,
        isGit: true,
        githubOwner: effectiveDetection.github_owner,
        githubRepo: effectiveDetection.github_repo,
        githubAvatarUrl: effectiveDetection.github_avatar_url,
        worktrees,
      });
    }

    // Sort: Preferred repo (Custos) first, then alphabetical
    projectWorkspaces.sort((a, b) => {
      if (a.name === 'Custos') return -1;
      if (b.name === 'Custos') return 1;
      return a.name.localeCompare(b.name);
    });

    const groups: ProjectGroupItem[] = [];

    // Main Project Group
    groups.push({
      id: 'proj_agenthub',
      name: projectName,
      path: projectPath,
      isMultiRepo,
      workspaces: projectWorkspaces,
    });

    // Check for any workspaces outside the main project (standalone folders/checkouts)
    workspaces.forEach((ws) => {
      if (ws.status === 'archived') return;
      if (projectPath && ws.path.startsWith(projectPath)) return;
      if (ws.lineage?.parent_workspace_id) return;

      const wsBranch = ws.active_branch || (ws.kind?.type === 'git' ? ws.kind.branch : undefined) || 'main';
      const dirtyCount = ws.dirty_manifest?.modified_files?.length || 0;
      otherWorkspaces.push({
        id: ws.id,
        name: ws.name,
        path: ws.path,
        isGit: ws.kind?.type === 'git',
        worktrees: [
          {
            id: ws.id,
            name: wsBranch,
            branch: wsBranch,
            path: ws.path,
            isMain: true,
            isDirty: Boolean(ws.dirty_manifest?.is_dirty),
            dirtyCount,
            isRunning: false,
            isBlocked: false,
            sessionCount: allSessions.filter((s) => s.workspaceId === ws.id).length,
            agentActivity: getAgentActivity(ws.id, ws.path),
          },
        ],
      });
    });

    if (otherWorkspaces.length > 0) {
      groups.push({
        id: 'proj_other',
        name: 'Other Projects',
        path: '',
        isMultiRepo: false,
        workspaces: otherWorkspaces,
      });
    }

    // Apply quick filters
    let result = groups;

    if (filterActiveOnly) {
      result = result
        .map((g) => ({
          ...g,
          workspaces: g.workspaces
            .map((ws) => ({
              ...ws,
              worktrees: ws.worktrees.filter(
                (wt) => wt.isDirty || wt.isRunning || wt.isBlocked || wt.agentActivity != null
              ),
            }))
            .filter((ws) => ws.worktrees.length > 0),
        }))
        .filter((g) => g.workspaces.length > 0);
    }

    if (filterQuery.trim()) {
      const q = filterQuery.toLowerCase();
      result = result
        .map((g) => ({
          ...g,
          workspaces: g.workspaces
            .map((ws) => ({
              ...ws,
              worktrees: ws.worktrees.filter(
                (wt) =>
                  ws.name.toLowerCase().includes(q) ||
                  wt.name.toLowerCase().includes(q) ||
                  wt.branch.toLowerCase().includes(q)
              ),
            }))
            .filter((ws) => ws.worktrees.length > 0),
        }))
        .filter((g) => g.workspaces.length > 0);
    }

    return result;
  }, [workspaces, allSessions, activeDetection, currentProject, filterQuery, filterActiveOnly, activeWorkspaceId]);

  // Count total worktrees and active worktrees for filter chips
  const { totalWorktreesCount, activeWorktreesCount } = useMemo(() => {
    let total = 0;
    let active = 0;
    projectGroups.forEach((g) => {
      g.workspaces.forEach((w) => {
        w.worktrees.forEach((wt) => {
          total++;
          if (wt.isDirty || wt.isRunning || wt.isBlocked || wt.agentActivity != null) {
            active++;
          }
        });
      });
    });
    return { totalWorktreesCount: total, activeWorktreesCount: active };
  }, [projectGroups]);

  // Flattened active worktrees collection for compact branch/worktree switcher
  const allWorktrees = useMemo(() => {
    const list: WorktreeItem[] = [];
    projectGroups.forEach((group) => {
      group.workspaces.forEach((ws) => {
        list.push(...ws.worktrees);
      });
    });
    return list;
  }, [projectGroups]);

  const activeWt = useMemo(() => {
    return (
      allWorktrees.find(
        (wt) => wt.id === activeWorkspaceId || (activeWorkspace && wt.path === activeWorkspace.path)
      ) || allWorktrees[0]
    );
  }, [allWorktrees, activeWorkspaceId, activeWorkspace]);

  const [isWorktreesExpanded, setIsWorktreesExpanded] = useState(false);

  const collapseAllFolders = useCallback(() => {
    setExpandedFolders(new Set());
  }, []);

  const folderDiffCounts = useMemo(() => {
    const counts = new Map<string, number>();
    if (!diffSummary?.files) return counts;
    for (const diff of diffSummary.files) {
      const parts = diff.path.split('/');
      let prefix = '';
      for (let i = 0; i < parts.length - 1; i++) {
        prefix = prefix ? `${prefix}/${parts[i]}` : parts[i];
        counts.set(prefix, (counts.get(prefix) || 0) + 1);
      }
    }
    return counts;
  }, [diffSummary]);

  // Convert sessions to unified WorkflowCards for sessions view
  const workflowCards = useMemo(() => {
    const tasksById = Object.fromEntries(tasks.map((task) => [task.id, task]));
    const list = sessions.length > 0 ? sessions : allSessions;
    return list.map((s) => sessionToWorkflowCard(s, { tasksById }));
  }, [sessions, allSessions, tasks]);

  // Unified collection of recent sessions & tasks sorted chronologically (most recent first)
  const recentItems = useMemo(() => {
    const list: Array<{
      id: string;
      title: string;
      kind: 'chat' | 'task';
      status: 'running' | 'blocked' | 'done' | 'idle';
      timestamp: number;
      timeAgo: string;
      sessionId?: string;
      taskId?: string;
    }> = [];

    const now = Date.now();
    const formatTimeAgo = (ts: number) => {
      if (!ts) return 'recently';
      const diffSec = Math.floor((now - ts) / 1000);
      if (diffSec < 60) return 'just now';
      if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
      if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
      return `${Math.floor(diffSec / 86400)}d ago`;
    };

    // 1. Map workflow cards (sessions)
    for (const card of workflowCards) {
      const ts = new Date(card.updatedAt || card.createdAt || 0).getTime() || 0;
      const isTask = card.hasTask && !card.hasChat;
      list.push({
        id: card.id,
        title: card.title || 'Untitled Session',
        kind: isTask ? 'task' : 'chat',
        status:
          card.status === 'running'
            ? 'running'
            : card.status === 'blocked'
              ? 'blocked'
              : card.status === 'done'
                ? 'done'
                : 'idle',
        timestamp: ts,
        timeAgo: formatTimeAgo(ts),
        sessionId: card.id,
        taskId: card.taskId,
      });
    }

    // 2. Also map any independent tasks from `tasks` that don't have a workflowCard
    const existingTaskIds = new Set(workflowCards.map((c) => c.taskId).filter(Boolean));
    for (const t of tasks) {
      if (!existingTaskIds.has(t.id)) {
        const ts = new Date(t.updated_at || t.created_at || 0).getTime() || 0;
        const rawStatus = String(t.status).toLowerCase();
        const status =
          rawStatus === 'running' || rawStatus === 'active'
            ? 'running'
            : rawStatus === 'blocked'
              ? 'blocked'
              : rawStatus === 'completed' || rawStatus === 'done'
                ? 'done'
                : 'idle';
        list.push({
          id: t.id,
          title: t.title || t.id,
          kind: 'task',
          status,
          timestamp: ts,
          timeAgo: formatTimeAgo(ts),
          taskId: t.id,
        });
      }
    }

    // Sort descending by timestamp (latest first)
    return list.sort((a, b) => b.timestamp - a.timestamp);
  }, [workflowCards, tasks]);

  const filteredRecentItems = useMemo(() => {
    return recentItems.filter((item) => {
      if (recentFilter === 'chats' && item.kind !== 'chat') return false;
      if (recentFilter === 'tasks' && item.kind !== 'task') return false;
      if (recentSearch.trim()) {
        const q = recentSearch.toLowerCase();
        return item.title.toLowerCase().includes(q);
      }
      return true;
    });
  }, [recentItems, recentFilter, recentSearch]);

  const activeRecentCount = useMemo(
    () => recentItems.filter((i) => i.status === 'running').length,
    [recentItems]
  );

  const formatRelativeTime = (tsString?: string) => {
    if (!tsString) return '';
    const ts = new Date(tsString).getTime();
    if (!ts || isNaN(ts)) return '';
    const diffSec = Math.floor((Date.now() - ts) / 1000);
    if (diffSec < 60) return 'just now';
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
    if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
    return `${Math.floor(diffSec / 86400)}d ago`;
  };

  const copilotGroupedTasks = useMemo(() => {
    const query = filterQuery.trim().toLowerCase();
    const filtered = tasks.filter((t) => {
      if (!query) return true;
      const title = (t.title || t.id).toLowerCase();
      const pack = (t.contract?.pack_id || t.contract?.pack || '').toLowerCase();
      return title.includes(query) || pack.includes(query);
    });

    const needsApproval: Task[] = [];
    const inProgress: Task[] = [];
    const standbyCompleted: Task[] = [];

    filtered.forEach((t) => {
      const s = String(t.status).toLowerCase();
      if (s === 'blocked') {
        needsApproval.push(t);
      } else if (s === 'running' || s === 'queued' || s === 'active' || s === 'draft') {
        inProgress.push(t);
      } else {
        if (!filterActiveOnly) {
          standbyCompleted.push(t);
        }
      }
    });

    return { needsApproval, inProgress, standbyCompleted };
  }, [tasks, filterQuery, filterActiveOnly]);

  const workbenchConfig = useMemo(() => {
    const wb = activeWorkbench || 'engineering';
    if (wb === 'research') {
      const studyName =
        researchDetection?.name ||
        (researchPath ? researchPath.split('/').filter(Boolean).pop() : null);
      return {
        title: 'Studies & Datasets',
        icon: FlaskConical,
        iconColor: 'text-white/70',
        badge: 'Lab',
        badgeClass: 'bg-white/[0.06] text-white/60 border-white/10',
        searchPlaceholder: 'Filter studies, datasets, notebooks...',
        allLabel: 'Studies',
        activeLabel: 'Active',
        allCount: researchPath ? 1 : 0,
        activeCount: researchPath ? 1 : 0,
        footerStatus: studyName ? `Research: ${studyName}` : 'Research: No study active',
      };
    }
    if (wb === 'copilot') {
      const runningCount = tasks.filter((t) => {
        const s = String(t.status).toLowerCase();
        return s === 'running' || s === 'queued' || s === 'active';
      }).length;
      return {
        title: 'Task & Operations Hub',
        icon: Bot,
        iconColor: 'text-white/70',
        badge: 'Tasks',
        badgeClass: 'bg-white/[0.06] text-white/60 border-white/10',
        searchPlaceholder: 'Filter tasks & operations...',
        allLabel: 'All Tasks',
        activeLabel: 'Running',
        allCount: tasks.length,
        activeCount: runningCount,
        footerStatus: `Copilot: ${tasks.length} tasks monitored`,
      };
    }
    return {
      title: activeWorkspace?.name || 'Workspace Explorer',
      icon: Boxes,
      iconColor: 'text-white/70',
      badge: 'Git',
      badgeClass: 'bg-white/[0.06] text-white/60 border-white/10',
      searchPlaceholder: 'Filter files or folders...',
      allLabel: 'All',
      activeLabel: 'Active',
      allCount: totalWorktreesCount,
      activeCount: activeWorktreesCount,
      footerStatus: `Git: ${activeWorkspace ? activeWorkspace.name : 'Custos Engine'}`,
    };
  }, [
    activeWorkbench,
    researchPath,
    researchDetection,
    activeWorkspace,
    tasks,
    totalWorktreesCount,
    activeWorktreesCount,
  ]);

  const handleSelectRecentItem = (item: (typeof recentItems)[0]) => {
    if (item.sessionId) {
      onSelectSession?.(item.sessionId);
    } else if (item.taskId) {
      onSelectTask?.(item.taskId);
    } else {
      onSelectSession?.(item.id);
    }
    setIsRecentOpen(false);
  };

  const handleDeleteRecentItem = (e: React.MouseEvent, item: (typeof recentItems)[0]) => {
    e.stopPropagation();
    if (item.sessionId && onDeleteSession) {
      if (window.confirm(`Delete "${item.title}"?`)) {
        void onDeleteSession(item.sessionId);
      }
    }
  };

  // Handler to create a new worktree
  const handleAddWorktree = async (workspaceId: string, branch: string) => {
    if (!branch.trim()) return;
    setIsSubmittingWorktree(true);
    try {
      let targetRepoPath: string | undefined;

      // Find workspace repo path
      for (const group of projectGroups) {
        const found = group.workspaces.find((w) => w.id === workspaceId);
        if (found) {
          targetRepoPath = found.path;
          break;
        }
      }

      if (!targetRepoPath) {
        const parentWs = workspaces.find((w) => w.id === workspaceId);
        targetRepoPath = parentWs?.path;
      }

      if (!targetRepoPath) throw new Error('Repository path not found');

      const sanitizedBranch = branch.trim().replace(/[\/\\]/g, '-');
      const targetPath = `${targetRepoPath.replace(/[\\/]+$/, '')}.worktrees/${sanitizedBranch}`;

      const created = await daemonClient.createWorkspace({
        name: branch.trim(),
        kind: {
          type: 'git',
          repo_path: targetRepoPath,
          branch: branch.trim(),
        },
        path: targetPath,
      });

      onShowToast?.(`Created Git worktree "${branch.trim()}" successfully.`);
      await refreshWorkspaces();
      if (created?.id) {
        onSelectWorkspace?.(created.id);
        setIsInspectorOpen(true);
      }
      setCreatingWorktreeFor(null);
      setWorktreeBranchInput('');
    } catch (err: any) {
      onShowToast?.(`Failed to create worktree: ${err?.message || err}`);
    } finally {
      setIsSubmittingWorktree(false);
    }
  };

  const handleSelectWorkspaceItem = async (id: string, path?: string) => {
    let targetWorkspaceId = id;
    let targetBranch: string | undefined;
    let targetName: string | undefined;
    let targetPath = path;

    const existing = workspaces.find((w) => w.id === id || (path && w.path === path));
    if (existing) {
      targetWorkspaceId = existing.id;
      targetBranch = existing.active_branch || (existing.kind?.type === 'git' ? existing.kind.branch : undefined);
      targetName = existing.name;
      targetPath = existing.path;
      onSelectWorkspace(existing.id);
    } else if (path) {
      try {
        const detected = await daemonClient.detectWorkspace(path);
        if (!detected.is_git || !detected.branch) {
          throw new Error('Could not detect a valid Git branch in this checkout.');
        }
        const checkoutPath = detected.root_path;
        const repositoryPath =
          detected.worktrees?.find((worktree) => worktree.is_main && !worktree.is_bare)?.path || checkoutPath;
        const created = await daemonClient.createWorkspace({
          name: path.split('/').pop() || 'workspace',
          kind: { type: 'git', repo_path: checkoutPath, branch: detected.branch },
          path: checkoutPath,
          metadata: { git_repository_path: repositoryPath, registered_existing_checkout: true },
        });
        await refreshWorkspaces();
        if (created?.id) {
          targetWorkspaceId = created.id;
          targetBranch = detected.branch;
          targetName = created.name;
          targetPath = checkoutPath;
          onSelectWorkspace(created.id);
        }
      } catch (cause) {
        onShowToast?.(`Could not load checkout: ${cause instanceof Error ? cause.message : String(cause)}`);
        return;
      }
    } else {
      onSelectWorkspace(id);
    }

    // Materialize the anchored working Screen for this worktree
    revealWorktreeScreen({
      id: targetWorkspaceId,
      name: targetName || 'workspace',
      branch: targetBranch,
      path: targetPath,
    });
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
                {activeRecentCount > 0 && (
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
          /* ── COPILOT TASK & OPERATIONS HUB ── */
          tasks.length === 0 ? (
            <div className="p-4 text-center space-y-3 border border-dashed border-white/10 rounded-xl my-4">
              <Bot className="size-7 text-white/40 mx-auto" />
              <div className="space-y-1">
                <div className="text-white/80 font-medium text-xs">No active tasks or operations</div>
                <div className="text-white/40 text-[11px] leading-relaxed">
                  Create an autonomous task or copilot mission to supervise.
                </div>
              </div>
              {onRequestCreateTask && (
                <button
                  type="button"
                  onClick={onRequestCreateTask}
                  className="px-3 py-1.5 bg-white/[0.06] hover:bg-white/[0.1] text-white/90 border border-white/10 text-xs rounded-lg transition font-medium cursor-pointer flex items-center gap-1.5 mx-auto"
                >
                  <Plus className="size-3.5" />
                  <span>New Task</span>
                </button>
              )}
            </div>
          ) : (
            <div className="space-y-3">
              {/* Needs Review Section */}
              {copilotGroupedTasks.needsApproval.length > 0 && (
                <div className="space-y-1.5">
                  <div className="flex items-center gap-1.5 px-1 text-[11px] font-semibold text-amber-300">
                    <span className="size-2 rounded-full bg-amber-400 animate-pulse" />
                    <span>Needs Review ({copilotGroupedTasks.needsApproval.length})</span>
                  </div>
                  <div className="space-y-1.5">
                    {copilotGroupedTasks.needsApproval.map((task) => {
                      const isSelected = selectedTaskId === task.id;
                      return (
                        <div
                          key={task.id}
                          onClick={() => onSelectTask?.(task.id)}
                          className={`p-2.5 rounded-xl border transition cursor-pointer space-y-1.5 ${
                            isSelected
                              ? 'bg-amber-500/15 border-amber-500/50 text-white shadow-xs'
                              : 'bg-amber-500/5 hover:bg-amber-500/10 border-amber-500/25 text-white/90'
                          }`}
                        >
                          <div className="flex items-center justify-between gap-1 text-[10px]">
                            <span className="font-mono px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-300 font-semibold uppercase tracking-wider text-[9px]">
                              {task.contract?.pack_id || 'approval'}
                            </span>
                            <span className="text-amber-400 font-mono text-[9px]">Action Needed</span>
                          </div>
                          <div className="text-xs font-medium line-clamp-2">{task.title || task.id}</div>
                          <div className="flex items-center justify-between text-[10px] text-white/40 pt-1 border-t border-white/6 font-mono">
                            <span>{task.id.slice(0, 8)}</span>
                            <span className="text-amber-300 font-sans font-medium hover:underline">Review ➔</span>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}

              {/* In Progress Section */}
              {copilotGroupedTasks.inProgress.length > 0 && (
                <div className="space-y-1.5">
                  <div className="flex items-center gap-1.5 px-1 text-[11px] font-semibold text-emerald-300">
                    <span className="size-2 rounded-full bg-emerald-400 animate-pulse" />
                    <span>In Progress ({copilotGroupedTasks.inProgress.length})</span>
                  </div>
                  <div className="space-y-1.5">
                    {copilotGroupedTasks.inProgress.map((task) => {
                      const isSelected = selectedTaskId === task.id;
                      return (
                        <div
                          key={task.id}
                          onClick={() => onSelectTask?.(task.id)}
                          className={`p-2.5 rounded-xl border transition cursor-pointer space-y-1.5 ${
                            isSelected
                              ? 'bg-white/[0.08] border-emerald-500/40 text-white shadow-xs'
                              : 'bg-white/[0.02] hover:bg-white/[0.05] border-white/8 text-white/80'
                          }`}
                        >
                          <div className="flex items-center justify-between gap-1 text-[10px]">
                            <span className="font-mono px-1.5 py-0.5 rounded bg-white/8 text-white/60 text-[9px]">
                              {task.contract?.pack_id || 'task'}
                            </span>
                            <span className="flex items-center gap-1 text-emerald-400 text-[10px] font-mono">
                              <span className="size-1.5 rounded-full bg-emerald-400 animate-pulse" />
                              {String(task.status).toLowerCase()}
                            </span>
                          </div>
                          <div className="text-xs font-medium line-clamp-2 text-white/95">{task.title || task.id}</div>
                          <div className="flex items-center justify-between text-[10px] text-white/40 pt-0.5 font-mono">
                            <span>{task.id.slice(0, 8)}</span>
                            <span>{formatRelativeTime(task.updated_at || task.created_at)}</span>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}

              {/* Completed & Standby Section */}
              {copilotGroupedTasks.standbyCompleted.length > 0 && !filterActiveOnly && (
                <div className="space-y-1.5">
                  <div className="flex items-center gap-1.5 px-1 text-[11px] font-semibold text-white/50">
                    <CheckSquare className="size-3 text-white/40" />
                    <span>Completed & Standby ({copilotGroupedTasks.standbyCompleted.length})</span>
                  </div>
                  <div className="space-y-1">
                    {copilotGroupedTasks.standbyCompleted.map((task) => {
                      const isSelected = selectedTaskId === task.id;
                      const isDone =
                        String(task.status).toLowerCase() === 'done' ||
                        String(task.status).toLowerCase() === 'succeeded';
                      return (
                        <div
                          key={task.id}
                          onClick={() => onSelectTask?.(task.id)}
                          className={`px-2.5 py-1.5 rounded-lg border transition cursor-pointer flex items-center justify-between text-xs ${
                            isSelected
                              ? 'bg-white/10 border-white/20 text-white'
                              : 'hover:bg-white/[0.03] border-transparent text-white/60'
                          }`}
                        >
                          <div className="flex items-center gap-2 min-w-0 mr-2">
                            <span
                              className={`size-1.5 rounded-full shrink-0 ${isDone ? 'bg-white/40' : 'bg-rose-400/60'}`}
                            />
                            <span className="truncate">{task.title || task.id}</span>
                          </div>
                          <span className="text-[10px] text-white/30 font-mono shrink-0">
                            {formatRelativeTime(task.updated_at || task.created_at)}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          )
        ) : activeWorkbench === 'research' ? (
          /* ── RESEARCH STUDIES & DATASETS VIEW (ISOLATED WORKSPACE) ── */
          !researchPath ? (
            <div className="p-4 text-center space-y-3 border border-dashed border-white/10 rounded-xl my-4">
              <FlaskConical className="size-7 text-white/40 mx-auto" />
              <div className="space-y-1">
                <div className="text-white/80 font-medium text-xs">No research study opened</div>
                <div className="text-white/40 text-[11px] leading-relaxed">
                  Open a study folder, scientific datasets, or experiment notebooks.
                </div>
              </div>
              <button
                type="button"
                onClick={handleOpenStudyFolder}
                className="px-3 py-1.5 bg-white/[0.06] hover:bg-white/[0.1] text-white/90 border border-white/10 text-xs rounded-lg transition font-medium cursor-pointer flex items-center gap-1.5 mx-auto"
              >
                <FolderPlus className="size-3.5" />
                <span>Open Study Folder (⌘O)</span>
              </button>
            </div>
          ) : (
            <div className="space-y-2">
              <div className="p-2.5 rounded-xl border border-white/10 bg-white/[0.03] space-y-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 min-w-0">
                    <FlaskConical className="size-4 text-white/70 shrink-0" />
                    <span className="font-bold text-xs text-white truncate">
                      {researchDetection?.name || researchPath.split('/').filter(Boolean).pop()}
                    </span>
                  </div>
                  <span className="text-[9px] font-mono px-1.5 py-0.2 rounded bg-white/[0.06] text-white/60 border border-white/10">
                    Study
                  </span>
                </div>

                <div className="text-[10px] text-white/40 font-mono truncate" title={researchPath}>
                  {researchPath}
                </div>

                <div className="pt-1.5 border-t border-white/6 flex items-center justify-between">
                  <span className="text-[10.5px] text-white/60">Study workspace ready</span>
                  <button
                    type="button"
                    onClick={handleOpenStudyFolder}
                    className="text-[10px] text-white/60 hover:text-white transition underline cursor-pointer"
                  >
                    Switch study
                  </button>
                </div>
              </div>
            </div>
          )
        ) : (
          /* ── ENGINEERING WORKSPACE EXPLORER (CLEAN VS CODE / ZED STYLE) ── */
          projectGroups.length === 0 ? (
            <div className="p-4 text-center space-y-2.5 border border-dashed border-white/10 rounded-lg my-2">
              <FolderOpen className="size-6 text-white/30 mx-auto" />
              <div className="text-white/50 text-xs">No projects currently open</div>
              <button
                type="button"
                onClick={handleOpenFolderDirect}
                className="px-3 py-1.5 bg-white/8 hover:bg-white/14 text-white text-xs rounded-md transition font-medium cursor-pointer"
              >
                Open Project Folder
              </button>
            </div>
          ) : (
            <div className="flex-1 min-h-0 flex flex-col -m-2 overflow-hidden">
              {/* ── 1. Top Section: Active Branch / Worktree Selector Bar ── */}
              <div className="px-2.5 py-1.5 border-b border-white/8 bg-white/[0.02] flex items-center justify-between shrink-0 select-none">
                <button
                  type="button"
                  onClick={() => setIsWorktreesExpanded((prev) => !prev)}
                  className="flex items-center gap-1.5 min-w-0 flex-1 text-left hover:text-white transition group/wt cursor-pointer"
                  title={`Active branch: ${activeWt?.branch || activeWorkspace?.name || 'main'} · Click to switch worktree`}
                >
                  <GitBranch className="size-3.5 text-white/70 group-hover/wt:text-white shrink-0" />
                  <span className="font-mono text-[11px] font-semibold text-white/90 group-hover/wt:text-white truncate">
                    {activeWt?.branch || activeWorkspace?.name || 'main'}
                  </span>
                  {activeWt?.isMain && (
                    <span className="px-1 py-0.2 rounded text-[8.5px] font-mono bg-white/[0.06] text-white/50 border border-white/10 shrink-0">
                      primary
                    </span>
                  )}
                  {activeWt?.dirtyCount ? (
                    <span className="font-mono text-[9px] text-amber-400 font-bold shrink-0">
                      {activeWt.dirtyCount}*
                    </span>
                  ) : null}
                  <ChevronDown
                    className={`size-3 text-white/40 group-hover/wt:text-white/70 transition-transform shrink-0 ${
                      isWorktreesExpanded ? 'rotate-180' : ''
                    }`}
                  />
                </button>

                <div className="flex items-center gap-1 shrink-0">
                  <button
                    type="button"
                    onClick={() => {
                      setIsWorktreesExpanded(true);
                      if (activeWorkspace) setCreatingWorktreeFor(activeWorkspace.id);
                    }}
                    className="p-1 rounded text-white/40 hover:text-white hover:bg-white/8 transition cursor-pointer"
                    title="Create new worktree"
                    aria-label="Create new worktree"
                  >
                    <Plus className="size-3" />
                  </button>
                </div>
              </div>

              {/* Collapsible Worktrees Dropdown */}
              {isWorktreesExpanded && (
                <div className="border-b border-white/8 bg-black/40 p-2 space-y-1.5 shrink-0 max-h-56 overflow-y-auto no-scrollbar shadow-inner">
                  <div className="flex items-center justify-between px-1 text-[10px] text-white/50 uppercase tracking-wider font-semibold">
                    <span>Worktrees ({allWorktrees.length})</span>
                    <button
                      type="button"
                      onClick={() => {
                        if (activeWorkspace) setCreatingWorktreeFor(activeWorkspace.id);
                      }}
                      className="text-[10px] text-white/60 hover:text-white flex items-center gap-1 cursor-pointer"
                    >
                      <Plus className="size-2.5" />
                      <span>New</span>
                    </button>
                  </div>

                  {allWorktrees.map((wt) => {
                    const isSelected =
                      wt.id === activeWorkspaceId ||
                      (activeWorkspace && wt.path === activeWorkspace.path);
                    return (
                      <div
                        key={wt.id}
                        onClick={() => {
                          void handleSelectWorkspaceItem(wt.id, wt.path);
                          setIsWorktreesExpanded(false);
                        }}
                        className={`flex items-center justify-between px-2 py-1.5 rounded-md cursor-pointer transition text-xs select-none ${
                          isSelected
                            ? 'bg-white/12 text-white border border-white/15'
                            : 'hover:bg-white/[0.04] text-white/70 hover:text-white border border-transparent'
                        }`}
                        title={`${wt.name} (${wt.path})`}
                      >
                        <div className="flex items-center gap-1.5 min-w-0">
                          {wt.isDirty ? (
                            <span className="size-1.5 rounded-full bg-amber-400 shrink-0" />
                          ) : (
                            <span className="size-1.5 rounded-full bg-emerald-400 shrink-0" />
                          )}
                          <span className="font-mono text-[11px] truncate">
                            {wt.branch || wt.name}
                          </span>
                          {wt.isMain && (
                            <span className="px-1 text-[8.5px] font-mono rounded bg-white/[0.06] text-white/50 border border-white/10 shrink-0">
                              primary
                            </span>
                          )}
                        </div>
                        {wt.dirtyCount > 0 && (
                          <span className="text-amber-400 font-mono text-[9.5px] font-semibold shrink-0">
                            {wt.dirtyCount}*
                          </span>
                        )}
                      </div>
                    );
                  })}

                  {/* Inline Create Worktree Form */}
                  {creatingWorktreeFor && (
                    <form
                      onSubmit={(e) => {
                        e.preventDefault();
                        if (activeWorkspace) void handleAddWorktree(activeWorkspace.id, worktreeBranchInput);
                      }}
                      className="mt-1 p-2 rounded-lg bg-black/60 border border-white/10 space-y-1.5 font-sans"
                    >
                      <div className="text-[10px] text-white/50 font-medium">New worktree branch:</div>
                      <input
                        type="text"
                        value={worktreeBranchInput}
                        onChange={(e) => setWorktreeBranchInput(e.target.value)}
                        placeholder="branch-name (e.g. feat-ui)"
                        autoFocus
                        disabled={isSubmittingWorktree}
                        className="w-full bg-black/70 border border-white/10 rounded px-2 py-1 text-xs text-white placeholder-white/30 outline-none font-mono"
                      />
                      <div className="flex justify-end gap-1.5 text-[10px]">
                        <button
                          type="button"
                          disabled={isSubmittingWorktree}
                          onClick={() => {
                            setCreatingWorktreeFor(null);
                            setWorktreeBranchInput('');
                          }}
                          className="px-2 py-0.5 text-white/50 hover:text-white cursor-pointer"
                        >
                          Cancel
                        </button>
                        <button
                          type="submit"
                          disabled={!worktreeBranchInput.trim() || isSubmittingWorktree}
                          className="px-2.5 py-0.5 bg-white/20 hover:bg-white/30 text-white rounded font-medium disabled:opacity-50 cursor-pointer"
                        >
                          {isSubmittingWorktree ? 'Creating...' : 'Create'}
                        </button>
                      </div>
                    </form>
                  )}
                </div>
              )}

              {/* ── 2. Dedicated Workspace File Explorer (Primary Tree) ── */}
              <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
                {/* Explorer Sub-Header */}
                <div className="px-2.5 py-1.5 border-b border-white/6 flex items-center justify-between shrink-0 bg-white/[0.01]">
                  <div className="flex items-center gap-1.5 min-w-0">
                    <FolderTree className="size-3.5 text-amber-400 shrink-0" />
                    <span className="text-[10.5px] font-semibold text-white/75 tracking-wider uppercase">
                      Explorer
                    </span>
                    {fileTree?.entries && (
                      <span className="font-mono text-[9px] text-white/40">
                        ({fileTree.entries.filter((e) => !e.is_dir).length})
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-1 shrink-0">
                    <button
                      type="button"
                      onClick={() => void loadFileTree()}
                      className="p-1 rounded text-white/40 hover:text-white hover:bg-white/8 transition cursor-pointer"
                      title="Refresh file tree"
                    >
                      <RefreshCw className={`size-3 ${filesLoading ? 'animate-spin' : ''}`} />
                    </button>
                    <button
                      type="button"
                      onClick={collapseAllFolders}
                      className="p-1 rounded text-white/40 hover:text-white hover:bg-white/8 transition cursor-pointer"
                      title="Collapse all folders"
                    >
                      <ChevronUp className="size-3" />
                    </button>
                  </div>
                </div>

                {/* File Tree List */}
                <div className="flex-1 overflow-y-auto no-scrollbar py-1 space-y-0.5 select-none">
                  {filesLoading && !fileTree ? (
                    <div className="text-[11px] text-white/40 py-4 px-3 flex items-center gap-2">
                      <RefreshCw className="size-3 animate-spin text-white/50" />
                      <span>Scanning workspace files…</span>
                    </div>
                  ) : !fileTree || fileTree.entries.length === 0 ? (
                    <div className="text-[11px] text-white/30 py-4 px-3 italic">
                      No files found in workspace
                    </div>
                  ) : (
                    fileEntriesForTree
                      .filter((node) => isNodeVisible(node, expandedFolders))
                      .map((node) => {
                        const depth = Math.max(0, node.path.split('/').length - 1);
                        const isExpanded = expandedFolders.has(node.path);
                        const diffEntry = !node.is_dir ? diffMap.get(node.path) : undefined;
                        const folderChanges = node.is_dir ? folderDiffCounts.get(node.path) || 0 : 0;
                        const isFileActive =
                          !node.is_dir &&
                          activeFilePath &&
                          (node.path === activeFilePath || activeFilePath.endsWith(node.path));

                        return (
                          <div
                            key={node.path}
                            onClick={(e) => {
                              e.stopPropagation();
                              if (node.is_dir) {
                                toggleFolder(node.path);
                              } else {
                                onOpenFile?.(node.path, node.name);
                                onOpenTab?.(
                                  'files',
                                  `custos://file?path=${encodeURIComponent(node.path)}`,
                                  node.name
                                );
                              }
                            }}
                            className={`group flex items-center gap-1.5 py-1 px-1.5 rounded cursor-pointer transition select-none ${
                              isFileActive
                                ? 'bg-white/12 text-white font-medium border-l-2 border-amber-400 pl-1'
                                : 'text-white/70 hover:text-white hover:bg-white/[0.05]'
                            }`}
                            style={{ paddingLeft: `${Math.min(depth * 14 + (isFileActive ? 4 : 6), 56)}px` }}
                            title={node.path}
                          >
                            {node.is_dir ? (
                              <>
                                <span className="text-white/40 group-hover:text-white/70 transition shrink-0">
                                  {isExpanded ? (
                                    <ChevronDown className="size-3" />
                                  ) : (
                                    <ChevronRight className="size-3" />
                                  )}
                                </span>
                                {isExpanded ? (
                                  <FolderOpen className="size-3.5 text-amber-400/90 shrink-0" />
                                ) : (
                                  <Folder className="size-3.5 text-amber-400/80 shrink-0" />
                                )}
                                <span className="truncate font-sans text-[11.5px] leading-tight flex-1">
                                  {node.name}
                                </span>
                                {folderChanges > 0 && (
                                  <span className="font-mono text-[9px] text-amber-400/80 bg-amber-400/10 px-1 rounded shrink-0">
                                    {folderChanges}
                                  </span>
                                )}
                              </>
                            ) : (
                              <>
                                <span className="w-3 shrink-0" />
                                {getSidebarFileIcon(node.name)}
                                <span
                                  className={`truncate font-mono text-[11px] leading-tight flex-1 ${
                                    isFileActive ? 'text-white font-medium' : ''
                                  }`}
                                >
                                  {node.name}
                                </span>
                                {diffEntry && (
                                  <span
                                    className={`font-mono text-[8.5px] font-bold px-1 rounded border leading-none shrink-0 ${
                                      diffEntry.status === 'M'
                                        ? 'text-amber-400 bg-amber-400/10 border-amber-400/30'
                                        : diffEntry.status === 'A' || diffEntry.status === '??'
                                        ? 'text-emerald-400 bg-emerald-400/10 border-emerald-400/30'
                                        : 'text-rose-400 bg-rose-400/10 border-rose-400/30'
                                    }`}
                                  >
                                    {diffEntry.status === '??' ? 'U' : diffEntry.status}
                                  </span>
                                )}
                              </>
                            )}
                          </div>
                        );
                      })
                  )}
                </div>
              </div>
            </div>
          )
        )}
      </div>

      {/* ── RECENT ACTIVITY OVERLAY ── */}
      {isRecentOpen && (
        <div className="absolute inset-x-0 top-[41px] bottom-9 z-40 bg-[#141518]/98 backdrop-blur-md flex flex-col border-b border-white/8 shadow-2xl animate-in fade-in duration-150">
          {/* Header of Recent Activity */}
          <div className="p-2.5 pb-2 border-b border-white/6 flex items-center justify-between">
            <div className="flex items-center gap-1.5">
              <Clock className="size-3.5 text-white/60" />
              <span className="text-[11px] font-semibold text-white/90">Recent Activity</span>
              <span className="text-[9px] font-mono px-1.5 py-0.2 rounded-full bg-white/8 text-white/50">
                {recentItems.length}
              </span>
            </div>
            <button
              type="button"
              onClick={() => setIsRecentOpen(false)}
              className="p-1 text-white/40 hover:text-white hover:bg-white/8 rounded transition cursor-pointer"
              aria-label="Close recent activity"
            >
              <X className="size-3.5" />
            </button>
          </div>

          {/* Quick Filters: All | Chats | Tasks */}
          <div className="px-2.5 pt-2 pb-1.5 flex items-center gap-1">
            <button
              type="button"
              onClick={() => setRecentFilter('all')}
              className={`px-2 py-0.5 rounded text-[10.5px] transition cursor-pointer font-medium ${
                recentFilter === 'all'
                  ? 'bg-white/12 text-white'
                  : 'text-white/40 hover:text-white/70'
              }`}
            >
              All ({recentItems.length})
            </button>
            <button
              type="button"
              onClick={() => setRecentFilter('chats')}
              className={`px-2 py-0.5 rounded text-[10.5px] transition cursor-pointer font-medium flex items-center gap-1 ${
                recentFilter === 'chats'
                  ? 'bg-white/12 text-white'
                  : 'text-white/40 hover:text-white/70'
              }`}
            >
              <MessageSquare className="size-2.5" />
              <span>Chats ({recentItems.filter((i) => i.kind === 'chat').length})</span>
            </button>
            <button
              type="button"
              onClick={() => setRecentFilter('tasks')}
              className={`px-2 py-0.5 rounded text-[10.5px] transition cursor-pointer font-medium flex items-center gap-1 ${
                recentFilter === 'tasks'
                  ? 'bg-white/12 text-white'
                  : 'text-white/40 hover:text-white/70'
              }`}
            >
              <CheckSquare className="size-2.5" />
              <span>Tasks ({recentItems.filter((i) => i.kind === 'task').length})</span>
            </button>
          </div>

          {/* Search Input for Recent Activity */}
          <div className="px-2.5 pb-2">
            <div className="relative flex items-center">
              <Search className="size-3 text-white/35 absolute left-2 pointer-events-none" />
              <input
                type="text"
                value={recentSearch}
                onChange={(e) => setRecentSearch(e.target.value)}
                placeholder="Search recent activity..."
                className="w-full bg-[#18191d] border border-white/8 rounded-md pl-6 pr-6 py-1 text-[11px] text-white placeholder-white/30 focus:outline-hidden focus:border-white/20 transition"
              />
              {recentSearch && (
                <button
                  type="button"
                  onClick={() => setRecentSearch('')}
                  className="absolute right-2 text-white/30 hover:text-white/70"
                >
                  ✕
                </button>
              )}
            </div>
          </div>

          {/* List of Recent Items */}
          <div className="flex-1 overflow-y-auto no-scrollbar p-2 space-y-1">
            {filteredRecentItems.length === 0 ? (
              <div className="p-4 text-center text-white/40 text-xs">
                No recent activity found
              </div>
            ) : (
              filteredRecentItems.map((item) => {
                const isActive = item.sessionId === activeSessionId;
                const isRunning = item.status === 'running';
                const isBlocked = item.status === 'blocked';

                return (
                  <div
                    key={`${item.kind}-${item.id}`}
                    onClick={() => handleSelectRecentItem(item)}
                    className={`group flex items-center justify-between px-2.5 py-1.5 rounded-lg cursor-pointer transition text-xs ${
                      isActive
                        ? 'bg-white/12 text-white font-medium border border-white/10 shadow-xs'
                        : 'text-white/70 hover:text-white hover:bg-white/5 border border-transparent'
                    }`}
                  >
                    <div className="flex items-center gap-2 min-w-0 flex-1 mr-1">
                      <span
                        className={`size-1.5 rounded-full shrink-0 ${
                          isRunning
                            ? 'bg-emerald-400 animate-pulse'
                            : isBlocked
                              ? 'bg-amber-400'
                              : 'bg-white/25'
                        }`}
                      />
                      {item.kind === 'chat' ? (
                        <MessageSquare className="size-3 text-white/40 shrink-0" />
                      ) : (
                        <CheckSquare className="size-3 text-white/40 shrink-0" />
                      )}
                      <span className="truncate">{item.title}</span>
                    </div>

                    <div className="flex items-center gap-1.5 shrink-0">
                      <span className="text-[10px] text-white/30 font-mono">
                        {item.timeAgo}
                      </span>
                      {item.sessionId && onDeleteSession && (
                        <button
                          type="button"
                          onClick={(e) => handleDeleteRecentItem(e, item)}
                          className="opacity-0 group-hover:opacity-100 p-0.5 rounded text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer"
                          title="Delete session"
                        >
                          <Trash2 className="size-3" />
                        </button>
                      )}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>
      )}

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
              onClick={() => window.dispatchEvent(new CustomEvent('custos:open-shortcuts'))}
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
