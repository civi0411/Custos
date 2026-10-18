import React, { useState, useEffect, useRef, useMemo, useCallback } from 'react';
import {
  SlidersHorizontal,
  GitBranch,
  ListChecks,
  Cpu,
  Quote,
  ShieldCheck,
  CheckSquare,
  FileCheck,
  X,
  Search,
  RefreshCw,
  FileCode,
  Folder,
  CheckCircle2,
  Zap,
  HardDrive,
  BookOpen,
  Copy,
  Check,
  Play,
  ChevronRight,
  ChevronDown,
  Plus,
  Minus,
  RotateCcw,
  FileDiff,
  GitCommit,
  AlertCircle,
  Filter,
  ChevronLeft,
  ChevronUp,
  Maximize2,
  MessageSquare,
  ListTodo,
  Trash2,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { CompanionFileViewer } from '@/components/viewer';
import { Tooltip, ActionIcon } from '@/components/common';
import type {
  ExecutionWorkspace,
  Task,
  WorkspaceFileTree,
  WorkspaceFileEntry,
  WorkspaceFileContent,
  WorkspaceDiffSummary,
  WorkspaceDiffEntry,
  WorkspaceFileDiff,
} from '@/types/domain';
import type { Session } from '@/types';
import type { OrcaTab, OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export type RightSidebarTab =
  | 'chats'
  | 'tasks'
  // Engineering (Orca style: File explorer & Git changes)
  | 'files'
  | 'changes'
  // Common / Shared Inspector
  | 'inspector'
  | 'checks'
  // Research Auxiliary Lenses
  | 'engine'
  | 'variables'
  | 'citation'
  // Copilot Auxiliary Lenses
  | 'governance'
  | 'criteria'
  | 'brief';

export interface TabConfig {
  id: RightSidebarTab;
  label: string;
  icon: React.ElementType;
  title: string;
}

export const getTabsForMode = (workbenchMode: string): TabConfig[] => {
  if (workbenchMode === 'research') {
    return [
      { id: 'inspector', label: 'Provenance', icon: SlidersHorizontal, title: 'Hypothesis, Dataset & Scope' },
      { id: 'checks', label: 'Evidence', icon: ListChecks, title: 'Scientific Findings, Citations & DOIs' },
    ];
  }
  if (workbenchMode === 'copilot') {
    return [
      { id: 'inspector', label: 'Context', icon: SlidersHorizontal, title: 'Executive Session & Target Deliverable' },
      { id: 'governance', label: 'Approvals', icon: ShieldCheck, title: 'Human-in-the-Loop Safeguards & Pending Actions' },
    ];
  }
  // Engineering mode: 3 focused tabs (Files, Changes, Context)
  return [
    { id: 'files', label: 'Files', icon: Folder, title: 'Workspace File Explorer' },
    { id: 'changes', label: 'Changes', icon: GitBranch, title: 'Source Control & Git Diff' },
    { id: 'inspector', label: 'Context', icon: SlidersHorizontal, title: 'Worktree, Branch & Session Scope' },
  ];
};

export interface RightSidebarProps {
  isOpen: boolean;
  onClose: () => void;
  activeWorkspace?: ExecutionWorkspace | null;
  activeSession?: Session;
  activeTask?: Task | null;
  activeTab?: OrcaTab;
  onOpenTab?: (id: OrcaTabId, customUrl?: string, customTitle?: string) => void;
  mode?: string;
  // Sessions & Tasks
  sessions?: Session[];
  allSessions?: Session[];
  activeSessionId?: string;
  onSelectSession?: (id: string) => void;
  onNewSession?: () => void;
  onDeleteSession?: (id: string) => void | Promise<void>;
  tasks?: Task[];
  selectedTaskId?: string | null;
  onSelectTask?: (id: string) => void;
  onRequestCreateTask?: () => void;
}

export const RightSidebar: React.FC<RightSidebarProps> = ({
  isOpen,
  onClose,
  activeWorkspace,
  activeSession,
  activeTask,
  activeTab,
  onOpenTab,
  mode = 'engineering',
  sessions = [],
  allSessions = [],
  activeSessionId,
  onSelectSession,
  onNewSession,
  onDeleteSession,
  tasks = [],
  selectedTaskId,
  onSelectTask,
  onRequestCreateTask,
}) => {
  const currentTabs = useMemo(() => getTabsForMode(mode), [mode]);

  const [activeTabId, setActiveTabId] = useState<RightSidebarTab>(() => {
    try {
      const stored = localStorage.getItem('custos.right-sidebar-tab') as RightSidebarTab;
      if (stored) return stored;
      return mode === 'engineering' ? 'files' : 'inspector';
    } catch {
      return mode === 'engineering' ? 'files' : 'inspector';
    }
  });

  // Ensure active tab exists in current workbench mode
  useEffect(() => {
    if (!currentTabs.some((t) => t.id === activeTabId)) {
      setActiveTabId(currentTabs[0]?.id || 'inspector');
    }
  }, [currentTabs, activeTabId]);

  const handleSelectTab = (tab: RightSidebarTab) => {
    setActiveTabId(tab);
    try {
      localStorage.setItem('custos.right-sidebar-tab', tab);
    } catch {}
  };

  // Resizable width management
  const [sidebarWidth, setSidebarWidth] = useState<number>(() => {
    try {
      const saved = Number(localStorage.getItem('custos.right-sidebar-width'));
      return saved >= 260 && saved <= 760 ? saved : 320;
    } catch {
      return 320;
    }
  });

  const isDraggingRef = useRef(false);
  const containerRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    const handlePointerMove = (e: PointerEvent) => {
      if (!isDraggingRef.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = Math.max(260, Math.min(760, rect.right - e.clientX));
      setSidebarWidth(newWidth);
      window.dispatchEvent(new CustomEvent('custos:right-sidebar-resized', { detail: newWidth }));
    };

    const handlePointerUp = () => {
      if (isDraggingRef.current) {
        isDraggingRef.current = false;
        try {
          localStorage.setItem('custos.right-sidebar-width', String(sidebarWidth));
        } catch {}
        window.dispatchEvent(new CustomEvent('custos:right-sidebar-resized', { detail: sidebarWidth }));
      }
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    window.addEventListener('pointercancel', handlePointerUp);
    return () => {
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('pointercancel', handlePointerUp);
    };
  }, [sidebarWidth]);

  // File explorer state for Engineering Right Panel
  const [fileTree, setFileTree] = useState<WorkspaceFileTree | null>(null);
  const [filesLoading, setFilesLoading] = useState(false);
  const [fileSearchQuery, setFileSearchQuery] = useState('');
  const [onlyChangedFiles, setOnlyChangedFiles] = useState(false);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['']));

  const loadFileTree = useCallback(async () => {
    if (!activeWorkspace?.id) return;
    try {
      setFilesLoading(true);
      const tree = await daemonClient.getWorkspaceFileTree(activeWorkspace.id, '', 3);
      setFileTree(tree);
    } catch {
      // Quiet fail if workspace is uninitialized
    } finally {
      setFilesLoading(false);
    }
  }, [activeWorkspace?.id]);

  // Git diff state for Engineering Right Panel
  const [diffSummary, setDiffSummary] = useState<WorkspaceDiffSummary | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);

  // Commit composer & staging actions state
  const [commitMessage, setCommitMessage] = useState('');
  const [isCommitting, setIsCommitting] = useState(false);
  const [commitError, setCommitError] = useState<string | null>(null);
  const [commitSuccess, setCommitSuccess] = useState<string | null>(null);
  const [collapsedStaged, setCollapsedStaged] = useState(false);
  const [collapsedUnstaged, setCollapsedUnstaged] = useState(false);
  const [isStagingAction, setIsStagingAction] = useState(false);

  // Selected file diff inspector state (embedded right inside RightSidebar)
  const [selectedDiffPath, setSelectedDiffPath] = useState<string | null>(null);
  const [selectedDiffIsStaged, setSelectedDiffIsStaged] = useState<boolean>(false);
  const [fileDiff, setFileDiff] = useState<WorkspaceFileDiff | null>(null);
  const [loadingFileDiff, setLoadingFileDiff] = useState<boolean>(false);
  const [copiedDiff, setCopiedDiff] = useState<boolean>(false);

  // Selected file details inspector state (embedded right inside RightSidebar)
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const [selectedFileContent, setSelectedFileContent] = useState<WorkspaceFileContent | null>(null);
  const [loadingFileContent, setLoadingFileContent] = useState<boolean>(false);
  const [copiedFilePath, setCopiedFilePath] = useState<boolean>(false);

  const loadDiff = useCallback(async () => {
    if (!activeWorkspace?.id) return;
    try {
      setDiffLoading(true);
      const diff = await daemonClient.getWorkspaceDiff(activeWorkspace.id);
      setDiffSummary(diff);
    } catch {
      // Non-git workspace
    } finally {
      setDiffLoading(false);
    }
  }, [activeWorkspace?.id]);

  // Refresh both file tree and git diff status in parallel
  const handleRefreshWorkspaceFilesAndDiff = useCallback(async () => {
    await Promise.allSettled([loadFileTree(), loadDiff()]);
  }, [loadFileTree, loadDiff]);

  // Git diff map for instant status lookup in File Explorer
  const diffMap = useMemo(() => {
    const map = new Map<string, WorkspaceDiffEntry>();
    if (diffSummary?.files) {
      for (const f of diffSummary.files) {
        map.set(f.path, f);
      }
    }
    return map;
  }, [diffSummary]);

  // Fetch full file diff when inspecting a specific file in RightSidebar
  useEffect(() => {
    if (!activeWorkspace?.id || !selectedDiffPath) {
      setFileDiff(null);
      return;
    }
    let isMounted = true;
    setLoadingFileDiff(true);
    daemonClient
      .getWorkspaceFileDiff(activeWorkspace.id, selectedDiffPath, selectedDiffIsStaged)
      .then((diff) => {
        if (isMounted) setFileDiff(diff);
      })
      .catch((err) => {
        console.error('Failed to get file diff', err);
        if (isMounted) setFileDiff(null);
      })
      .finally(() => {
        if (isMounted) setLoadingFileDiff(false);
      });
    return () => {
      isMounted = false;
    };
  }, [activeWorkspace?.id, selectedDiffPath, selectedDiffIsStaged]);

  const openDiffInspector = useCallback(
    (path: string, isStaged: boolean) => {
      setSelectedDiffPath(path);
      setSelectedDiffIsStaged(isStaged);
      handleSelectTab('changes');
      if (sidebarWidth < 420) {
        setSidebarWidth(420);
      }
    },
    [sidebarWidth]
  );

  // Fetch full file content & details when inspecting a specific file in RightSidebar
  useEffect(() => {
    if (!activeWorkspace?.id || !selectedFilePath) {
      setSelectedFileContent(null);
      return;
    }
    let isMounted = true;
    setLoadingFileContent(true);
    daemonClient
      .readWorkspaceFile(activeWorkspace.id, selectedFilePath)
      .then((content) => {
        if (isMounted) setSelectedFileContent(content);
      })
      .catch((err) => {
        console.error('Failed to read workspace file in right sidebar', err);
        if (isMounted) setSelectedFileContent(null);
      })
      .finally(() => {
        if (isMounted) setLoadingFileContent(false);
      });
    return () => {
      isMounted = false;
    };
  }, [activeWorkspace?.id, selectedFilePath]);

  const openFileInspector = useCallback(
    (path: string) => {
      setSelectedFilePath(path);
      handleSelectTab('files');
      if (sidebarWidth < 420) {
        setSidebarWidth(420);
      }
    },
    [sidebarWidth]
  );

  const handleCopyFilePath = useCallback((path: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    navigator.clipboard.writeText(path);
    setCopiedFilePath(true);
    setTimeout(() => setCopiedFilePath(false), 1500);
  }, []);

  const changedFilesList = useMemo(() => diffSummary?.files || [], [diffSummary]);
  const currentDiffIndex = useMemo(() => {
    if (!selectedDiffPath || !changedFilesList.length) return -1;
    return changedFilesList.findIndex((f) => f.path === selectedDiffPath);
  }, [selectedDiffPath, changedFilesList]);

  const currentDiffEntry = useMemo(() => {
    if (!selectedDiffPath) return undefined;
    return diffMap.get(selectedDiffPath);
  }, [selectedDiffPath, diffMap]);

  const diffStats = useMemo(() => {
    if (!fileDiff?.diff) return null;
    let added = 0;
    let deleted = 0;
    for (const line of fileDiff.diff.split('\n')) {
      if (line.startsWith('+') && !line.startsWith('+++')) added++;
      else if (line.startsWith('-') && !line.startsWith('---')) deleted++;
    }
    return { added, deleted };
  }, [fileDiff]);

  const handlePrevDiff = () => {
    if (!changedFilesList.length) return;
    const prevIndex = currentDiffIndex <= 0 ? changedFilesList.length - 1 : currentDiffIndex - 1;
    const prevFile = changedFilesList[prevIndex];
    setSelectedDiffPath(prevFile.path);
    setSelectedDiffIsStaged(Boolean(prevFile.is_staged || prevFile.isStaged));
  };

  const handleNextDiff = () => {
    if (!changedFilesList.length) return;
    const nextIndex = currentDiffIndex >= changedFilesList.length - 1 ? 0 : currentDiffIndex + 1;
    const nextFile = changedFilesList[nextIndex];
    setSelectedDiffPath(nextFile.path);
    setSelectedDiffIsStaged(Boolean(nextFile.is_staged || nextFile.isStaged));
  };

  const handleCopyDiff = () => {
    if (!fileDiff?.diff) return;
    navigator.clipboard.writeText(fileDiff.diff);
    setCopiedDiff(true);
    setTimeout(() => setCopiedDiff(false), 2000);
  };

  // Staging / Unstaging / Discard / Commit operations
  const handleStageFile = async (filePath: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    if (!activeWorkspace?.id) return;
    try {
      setIsStagingAction(true);
      await daemonClient.stageWorkspaceFile(activeWorkspace.id, filePath);
      await loadDiff();
    } catch (err) {
      console.error('Failed to stage file:', err);
    } finally {
      setIsStagingAction(false);
    }
  };

  const handleUnstageFile = async (filePath: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    if (!activeWorkspace?.id) return;
    try {
      setIsStagingAction(true);
      await daemonClient.unstageWorkspaceFile(activeWorkspace.id, filePath);
      await loadDiff();
    } catch (err) {
      console.error('Failed to unstage file:', err);
    } finally {
      setIsStagingAction(false);
    }
  };

  const handleDiscardFile = async (filePath: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    if (!activeWorkspace?.id) return;
    if (!window.confirm(`Discard all local changes in "${filePath}"? This action cannot be undone.`)) return;
    try {
      setIsStagingAction(true);
      await daemonClient.discardWorkspaceFile(activeWorkspace.id, filePath);
      await Promise.allSettled([loadDiff(), loadFileTree()]);
    } catch (err) {
      console.error('Failed to discard file:', err);
    } finally {
      setIsStagingAction(false);
    }
  };

  const handleStageAll = async (e?: React.MouseEvent) => {
    e?.stopPropagation();
    if (!activeWorkspace?.id || !diffSummary?.files) return;
    const unstaged = diffSummary.files.filter((f) => !f.is_staged && !f.isStaged);
    if (!unstaged.length) return;
    try {
      setIsStagingAction(true);
      for (const f of unstaged) {
        await daemonClient.stageWorkspaceFile(activeWorkspace.id, f.path);
      }
      await loadDiff();
    } catch (err) {
      console.error('Failed to stage all files:', err);
    } finally {
      setIsStagingAction(false);
    }
  };

  const handleUnstageAll = async (e?: React.MouseEvent) => {
    e?.stopPropagation();
    if (!activeWorkspace?.id || !diffSummary?.files) return;
    const staged = diffSummary.files.filter((f) => f.is_staged || f.isStaged);
    if (!staged.length) return;
    try {
      setIsStagingAction(true);
      for (const f of staged) {
        await daemonClient.unstageWorkspaceFile(activeWorkspace.id, f.path);
      }
      await loadDiff();
    } catch (err) {
      console.error('Failed to unstage all files:', err);
    } finally {
      setIsStagingAction(false);
    }
  };

  const handleDiscardAll = async (e?: React.MouseEvent) => {
    e?.stopPropagation();
    if (!activeWorkspace?.id || !diffSummary?.files) return;
    const unstaged = diffSummary.files.filter((f) => !f.is_staged && !f.isStaged);
    if (!unstaged.length) return;
    if (!window.confirm(`Discard all local changes across ${unstaged.length} files? This action cannot be undone.`)) return;
    try {
      setIsStagingAction(true);
      for (const f of unstaged) {
        await daemonClient.discardWorkspaceFile(activeWorkspace.id, f.path);
      }
      await Promise.allSettled([loadDiff(), loadFileTree()]);
    } catch (err) {
      console.error('Failed to discard all files:', err);
    } finally {
      setIsStagingAction(false);
    }
  };

  const handleCommit = async () => {
    if (!activeWorkspace?.id || !commitMessage.trim() || isCommitting) return;
    try {
      setIsCommitting(true);
      setCommitError(null);
      await daemonClient.commitWorkspace(activeWorkspace.id, commitMessage.trim());
      setCommitMessage('');
      setCommitSuccess('Committed successfully');
      setTimeout(() => setCommitSuccess(null), 3000);
      await Promise.allSettled([loadDiff(), loadFileTree()]);
    } catch (err: any) {
      setCommitError(err?.message || 'Commit failed');
    } finally {
      setIsCommitting(false);
    }
  };

  // Live session metrics
  const [sessionMetrics, setSessionMetrics] = useState<{
    tokens_in?: number;
    tokens_out?: number;
    total_tokens?: number;
    estimated_cost_usd?: number;
    active_runs?: number;
  } | null>(null);

  useEffect(() => {
    if (!activeSession?.id) return;
    daemonClient.getSessionMetrics(activeSession.id).then(setSessionMetrics).catch(() => null);
  }, [activeSession?.id]);

  // Lazy load data on tab switch or workspace change
  useEffect(() => {
    if (!isOpen) return;
    if (activeTabId === 'files' || activeTabId === 'changes') {
      if (!fileTree) void loadFileTree();
      if (!diffSummary) void loadDiff();
    }
  }, [isOpen, activeTabId, fileTree, diffSummary, loadFileTree, loadDiff]);

  const toggleFolder = (path: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  // Flattened & filtered files, with optional Git changed files filter
  const filteredEntries = useMemo(() => {
    if (!fileTree?.entries) return [];
    let entries = fileTree.entries;
    if (onlyChangedFiles && diffSummary?.files) {
      const changedPaths = new Set(diffSummary.files.map((f) => f.path));
      entries = entries.filter((node: WorkspaceFileEntry) => {
        if (!node.is_dir) {
          return changedPaths.has(node.path);
        }
        const dirPrefix = node.path ? `${node.path}/` : '';
        return Array.from(changedPaths).some((p) => p.startsWith(dirPrefix));
      });
    }
    if (!fileSearchQuery.trim()) return entries;
    const query = fileSearchQuery.toLowerCase();
    return entries.filter(
      (node: WorkspaceFileEntry) =>
        node.name.toLowerCase().includes(query) || node.path.toLowerCase().includes(query)
    );
  }, [fileTree, fileSearchQuery, onlyChangedFiles, diffSummary]);

  // Citation copy state for Research
  const [copiedFormat, setCopiedFormat] = useState<string | null>(null);
  const handleCopyCitation = (format: string, text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedFormat(format);
    setTimeout(() => setCopiedFormat(null), 1800);
  };

  // Chats & Tasks filtering based on activeWorkspace
  const [chatSearchQuery, setChatSearchQuery] = useState('');

  const activeWorkspaceSessions = useMemo(() => {
    const list = allSessions && allSessions.length > 0 ? allSessions : sessions;
    if (!activeWorkspace?.id) return list;
    return list.filter((s) => s.workspaceId === activeWorkspace.id);
  }, [allSessions, sessions, activeWorkspace?.id]);

  const filteredChats = useMemo(() => {
    if (!chatSearchQuery.trim()) return activeWorkspaceSessions;
    const q = chatSearchQuery.toLowerCase();
    return activeWorkspaceSessions.filter((s) =>
      (s.title || '').toLowerCase().includes(q) || (s.model || '').toLowerCase().includes(q)
    );
  }, [activeWorkspaceSessions, chatSearchQuery]);

  const activeWorkspaceTasks = useMemo(() => {
    if (!tasks || tasks.length === 0) return [];
    if (!activeWorkspace?.id) return tasks;
    return tasks.filter((t) => t.metadata?.workspace_id === activeWorkspace.id);
  }, [tasks, activeWorkspace?.id]);

  if (!isOpen) return null;

  return (
    <aside
      ref={containerRef}
      style={{ width: `${sidebarWidth}px` }}
      className="relative flex flex-col h-full shrink-0 border-l border-white/8 bg-[#101114] text-white/80 select-none z-30 overflow-hidden"
      aria-label="Orca Tools and Inspector Sidebar"
    >
      {/* ── Active Left Drag Resize Handle ── */}
      <div
        className="absolute left-0 inset-y-0 w-1.5 -ml-0.5 cursor-col-resize z-40 hover:bg-workbench-accent active:bg-workbench-accent transition-colors"
        title="Drag to resize right sidebar (Double click to reset to 320px)"
        onDoubleClick={() => {
          setSidebarWidth(320);
          try { localStorage.setItem('custos.right-sidebar-width', '320'); } catch {}
          window.dispatchEvent(new CustomEvent('custos:right-sidebar-resized', { detail: 320 }));
        }}
        onPointerDown={(e) => {
          e.preventDefault();
          e.currentTarget.setPointerCapture(e.pointerId);
          isDraggingRef.current = true;
        }}
      />

      {/* ── Top Activity Bar: 38px Strip with Icon-Only Tab Switcher & Close Button ── */}
      <div className="h-10 shrink-0 flex items-center justify-between px-2.5 border-b border-white/8 bg-[#141518] text-xs">
        {/* Navigation Tabs (Icon Only with hover tooltip) */}
        <div className="flex items-center gap-1 p-0.5 rounded-lg bg-black/40 border border-white/8 text-xs shrink-0" role="tablist">
          {currentTabs.map((tab) => {
            const Icon = tab.icon;
            const isActive = activeTabId === tab.id;
            const changeCount = tab.id === 'changes' ? diffSummary?.files?.length : undefined;

            return (
              <Tooltip
                key={tab.id}
                content={`${tab.label} — ${tab.title}`}
                position="bottom"
              >
                <button
                  type="button"
                  onClick={() => handleSelectTab(tab.id)}
                  className={`relative size-7 flex items-center justify-center rounded-md transition cursor-pointer select-none ${
                    isActive
                      ? 'bg-white/14 text-white shadow-xs font-semibold'
                      : 'text-white/40 hover:text-white/80 hover:bg-white/6'
                  }`}
                  role="tab"
                  aria-selected={isActive}
                  aria-label={tab.label}
                >
                  <Icon className="size-3.5 shrink-0" />
                  {changeCount !== undefined && changeCount > 0 ? (
                    <span className="absolute -top-1 -right-1 px-1 min-w-[14px] h-3.5 flex items-center justify-center rounded-full bg-amber-500 text-black text-[9px] font-mono font-bold leading-none shadow-xs">
                      {changeCount}
                    </span>
                  ) : null}
                </button>
              </Tooltip>
            );
          })}
        </div>

        {/* Close Button */}
        <Tooltip content="Close Inspector Sidebar (⌥I)" position="bottom">
          <button
            type="button"
            onClick={onClose}
            className="size-7 flex items-center justify-center rounded-md text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer shrink-0 ml-1.5"
            aria-label="Close Inspector"
          >
            <X className="size-3.5" />
          </button>
        </Tooltip>
      </div>

      {/* ── Panel Body: Distinct Inspector Lenses ── */}
      <div className="flex-1 min-h-0 overflow-y-auto select-text p-3 text-xs">
        {/* ── 0A. WORKTREE CHAT SESSIONS ── */}
        {activeTabId === 'chats' && (
          <div className="flex flex-col h-full space-y-2">
            {/* Chats Tab Header */}
            <div className="flex items-center justify-between pb-2 border-b border-white/8 shrink-0">
              <div className="flex items-center gap-1.5 min-w-0">
                <MessageSquare className="size-3.5 text-emerald-400 shrink-0" />
                <span className="font-semibold text-white/90 text-xs">
                  Chats ({activeWorkspaceSessions.length})
                </span>
                {activeWorkspace && (
                  <span className="text-[10px] text-white/40 font-mono truncate max-w-[120px]" title={activeWorkspace.name}>
                    ({activeWorkspace.name})
                  </span>
                )}
              </div>

              <button
                type="button"
                onClick={() => onNewSession?.()}
                className="flex items-center gap-1 px-2 py-1 rounded bg-white/8 hover:bg-white/14 text-white text-[11px] font-medium transition cursor-pointer shrink-0"
                title="Start a new chat session in this workspace"
              >
                <Plus className="size-3 text-emerald-400" />
                <span>New Chat</span>
              </button>
            </div>

            {/* Search & Filter */}
            <div className="relative shrink-0">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3 text-white/30" />
              <input
                type="text"
                value={chatSearchQuery}
                onChange={(e) => setChatSearchQuery(e.target.value)}
                placeholder="Filter chats..."
                className="w-full bg-black/30 border border-white/8 rounded-md pl-7 pr-7 py-1 text-xs text-white placeholder-white/30 outline-none focus:border-white/20 transition"
              />
              {chatSearchQuery && (
                <button
                  type="button"
                  onClick={() => setChatSearchQuery('')}
                  className="absolute right-2 top-1/2 -translate-y-1/2 text-white/40 hover:text-white"
                >
                  <X className="size-3" />
                </button>
              )}
            </div>

            {/* Chat Session List */}
            <div className="flex-1 overflow-y-auto no-scrollbar space-y-1 pr-0.5">
              {filteredChats.length === 0 ? (
                <div className="py-8 text-center space-y-2">
                  <MessageSquare className="size-6 text-white/20 mx-auto" />
                  <p className="text-white/40 text-xs">
                    {chatSearchQuery ? 'No matching chats' : 'No chats in this worktree yet'}
                  </p>
                  <button
                    type="button"
                    onClick={() => onNewSession?.()}
                    className="text-xs text-workbench-accent hover:underline cursor-pointer"
                  >
                    + Start first conversation
                  </button>
                </div>
              ) : (
                filteredChats.map((sess) => {
                  const isActive = sess.id === activeSessionId;
                  const anyS = sess as any;
                  const isRunning = anyS.isRunning || anyS.status === 'active' || anyS.status === 'running';

                  return (
                    <div
                      key={sess.id}
                      onClick={() => onSelectSession?.(sess.id)}
                      className={`group flex items-start justify-between p-2 rounded-lg cursor-pointer transition select-none ${
                        isActive
                          ? 'bg-white/12 text-white border border-white/10 shadow-xs'
                          : 'text-white/70 hover:text-white hover:bg-white/6'
                      }`}
                    >
                      <div className="flex items-start gap-2 min-w-0 flex-1">
                        <div className="mt-0.5 shrink-0">
                          {isRunning ? (
                            <span className="size-2 rounded-full bg-emerald-400 animate-pulse inline-block" />
                          ) : (
                            <MessageSquare className={`size-3.5 ${isActive ? 'text-emerald-400' : 'text-white/40 group-hover:text-white/70'}`} />
                          )}
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="font-medium text-[11.5px] truncate">
                            {sess.title || 'Untitled Session'}
                          </div>
                          <div className="flex items-center gap-2 mt-0.5 text-[10px] text-white/40 font-mono">
                            {sess.model && <span className="truncate max-w-[90px]">{sess.model.split('/').pop()}</span>}
                            {((sess as any).updatedAt || sess.time || sess.created_at) && (
                              <span>{new Date((sess as any).updatedAt || sess.time || sess.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                            )}
                          </div>
                        </div>
                      </div>

                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onDeleteSession?.(sess.id);
                        }}
                        className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-rose-500/20 text-white/40 hover:text-rose-400 transition shrink-0 ml-1"
                        title="Delete session"
                      >
                        <Trash2 className="size-3" />
                      </button>
                    </div>
                  );
                })
              )}
            </div>
          </div>
        )}

        {/* ── 0B. WORKTREE TASKS & WORKFLOWS ── */}
        {activeTabId === 'tasks' && (
          <div className="flex flex-col h-full space-y-2">
            {/* Tasks Tab Header */}
            <div className="flex items-center justify-between pb-2 border-b border-white/8 shrink-0">
              <div className="flex items-center gap-1.5 min-w-0">
                <ListTodo className="size-3.5 text-workbench-accent shrink-0" />
                <span className="font-semibold text-white/90 text-xs">
                  Tasks ({activeWorkspaceTasks.length})
                </span>
                {activeWorkspace && (
                  <span className="text-[10px] text-white/40 font-mono truncate max-w-[120px]" title={activeWorkspace.name}>
                    ({activeWorkspace.name})
                  </span>
                )}
              </div>

              <button
                type="button"
                onClick={() => onRequestCreateTask?.()}
                className="flex items-center gap-1 px-2 py-1 rounded bg-white/8 hover:bg-white/14 text-white text-[11px] font-medium transition cursor-pointer shrink-0"
                title="Create a new task for this workspace"
              >
                <Plus className="size-3 text-workbench-accent" />
                <span>New Task</span>
              </button>
            </div>

            {/* Task List */}
            <div className="flex-1 overflow-y-auto no-scrollbar space-y-1.5 pr-0.5">
              {activeWorkspaceTasks.length === 0 ? (
                <div className="py-8 text-center space-y-2">
                  <ListTodo className="size-6 text-white/20 mx-auto" />
                  <p className="text-white/40 text-xs">No tasks in this workspace yet</p>
                  <button
                    type="button"
                    onClick={() => onRequestCreateTask?.()}
                    className="text-xs text-workbench-accent hover:underline cursor-pointer"
                  >
                    + Create your first task
                  </button>
                </div>
              ) : (
                activeWorkspaceTasks.map((task) => {
                  const isSelected = task.id === selectedTaskId;
                  const statusLower = String(task.status).toLowerCase();
                  const isRunning = statusLower === 'running' || statusLower === 'in_progress';
                  const isDone = statusLower === 'succeeded' || statusLower === 'done' || statusLower === 'completed';
                  const isFailed = statusLower === 'failed' || statusLower === 'error';

                  return (
                    <div
                      key={task.id}
                      onClick={() => onSelectTask?.(task.id)}
                      className={`group flex items-start justify-between p-2 rounded-lg cursor-pointer transition select-none ${
                        isSelected
                          ? 'bg-white/12 text-white border border-white/10 shadow-xs'
                          : 'text-white/70 hover:text-white hover:bg-white/6'
                      }`}
                    >
                      <div className="flex items-start gap-2 min-w-0 flex-1">
                        <div className="mt-0.5 shrink-0">
                          {isRunning ? (
                            <span className="size-2 rounded-full bg-emerald-400 animate-pulse inline-block" />
                          ) : isDone ? (
                            <CheckCircle2 className="size-3.5 text-emerald-400" />
                          ) : isFailed ? (
                            <AlertCircle className="size-3.5 text-rose-400" />
                          ) : (
                            <ListTodo className={`size-3.5 ${isSelected ? 'text-workbench-accent' : 'text-white/40'}`} />
                          )}
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="font-medium text-[11.5px] truncate">
                            {task.title || 'Untitled Task'}
                          </div>
                          <div className="flex items-center gap-2 mt-0.5 text-[10px] text-white/40 font-mono">
                            <span className={`px-1 py-0.2 rounded uppercase ${
                              isDone ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20' :
                              isRunning ? 'bg-sky-500/10 text-sky-400 border border-sky-500/20' :
                              isFailed ? 'bg-rose-500/10 text-rose-400 border border-rose-500/20' :
                              'bg-white/6 text-white/50'
                            }`}>
                              {task.status || 'Pending'}
                            </span>
                            {task.contract?.pack_id && <span>{task.contract.pack_id}</span>}
                          </div>
                        </div>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>
        )}

        {/* ── ENGINEERING: 1. WORKSPACE FILE EXPLORER & FILE DETAILS INSPECTOR ── */}
        {activeTabId === 'files' && (
          selectedFilePath ? (
            /* ── RICH EMBEDDED FILE CONTENT & DETAILS INSPECTOR ── */
            <div className="flex flex-col h-full space-y-2">
              {/* File Details Header */}
              <div className="flex items-center justify-between pb-2 border-b border-white/8 shrink-0">
                <div className="flex items-center gap-1.5 min-w-0">
                  <ActionIcon
                    icon={ChevronLeft}
                    tooltip="Back to file tree"
                    size="sm"
                    onClick={() => setSelectedFilePath(null)}
                    ariaLabel="Back to files"
                  />
                  <span className="text-white/20">/</span>
                  <FileCode className="size-3.5 text-sky-400 shrink-0" />
                  <span className="font-mono text-[11.5px] font-semibold text-white/95 truncate" title={selectedFilePath}>
                    {selectedFilePath.split('/').pop()}
                  </span>
                </div>

                <div className="flex items-center gap-1 shrink-0">
                  <ActionIcon
                    icon={copiedFilePath ? Check : Copy}
                    tooltip={copiedFilePath ? "Copied path to clipboard" : "Copy relative file path"}
                    size="sm"
                    onClick={(e) => handleCopyFilePath(selectedFilePath, e)}
                    className={copiedFilePath ? "text-emerald-400" : ""}
                    ariaLabel="Copy path"
                  />
                  {diffMap.has(selectedFilePath) && (
                    <ActionIcon
                      icon={FileDiff}
                      tooltip="Inspect Git Diff for this file"
                      size="sm"
                      onClick={() => {
                        const diffEntry = diffMap.get(selectedFilePath)!;
                        openDiffInspector(selectedFilePath, Boolean(diffEntry.is_staged || diffEntry.isStaged));
                      }}
                      className="text-amber-300 bg-amber-500/15 hover:bg-amber-500/25 border border-amber-500/30"
                      ariaLabel="Inspect Git Diff"
                    />
                  )}
                  <ActionIcon
                    icon={Maximize2}
                    tooltip="Open file in center editor canvas"
                    size="sm"
                    onClick={() =>
                      onOpenTab?.(
                        'files',
                        `custos://file?path=${encodeURIComponent(selectedFilePath)}`,
                        selectedFilePath.split('/').pop() || 'File'
                      )
                    }
                    ariaLabel="Open in editor canvas"
                  />
                </div>
              </div>

              {/* Subheader: Metadata chips (Size, Lines, Path) */}
              <div className="flex items-center justify-between text-[10.5px] text-white/50 font-mono px-1 shrink-0">
                <span className="truncate max-w-[200px]" title={selectedFilePath}>
                  {selectedFilePath}
                </span>
                <div className="flex items-center gap-2 shrink-0">
                  {selectedFileContent && (
                    <>
                      {selectedFileContent.line_count > 0 && (
                        <span>{selectedFileContent.line_count.toLocaleString()} lines</span>
                      )}
                      <span>{(selectedFileContent.size_bytes / 1024).toFixed(1)} KB</span>
                    </>
                  )}
                </div>
              </div>

              {/* Content View */}
              <div className="flex-1 overflow-hidden rounded border border-white/6 bg-black/40">
                {loadingFileContent ? (
                  <div className="flex items-center justify-center h-full text-white/40 text-xs">
                    <RefreshCw className="size-4 animate-spin mr-2" />
                    <span>Reading file content…</span>
                  </div>
                ) : selectedFileContent ? (
                  <CompanionFileViewer
                    path={selectedFilePath}
                    content={selectedFileContent.content}
                    isBinary={selectedFileContent.is_binary}
                    sizeBytes={selectedFileContent.size_bytes}
                    lineCount={selectedFileContent.line_count}
                    isReadOnly={true}
                    onOpenInBrowser={(url) => {
                      const fileName = selectedFilePath.split('/').pop() || 'Web Preview';
                      onOpenTab?.('browser', url, `${fileName} - Browser`);
                    }}
                  />
                ) : (
                  <div className="flex flex-col items-center justify-center h-full text-white/40 text-xs text-center p-4 space-y-2">
                    <FileCode className="size-8 stroke-1 text-white/20" />
                    <span>Could not load file content</span>
                    <button
                      type="button"
                      onClick={() => {
                        if (activeWorkspace?.id && selectedFilePath) {
                          setLoadingFileContent(true);
                          daemonClient
                            .readWorkspaceFile(activeWorkspace.id, selectedFilePath)
                            .then(setSelectedFileContent)
                            .catch(() => {})
                            .finally(() => setLoadingFileContent(false));
                        }
                      }}
                      className="px-2.5 py-1 rounded bg-white/10 hover:bg-white/20 text-white text-[10.5px] transition"
                    >
                      Retry
                    </button>
                  </div>
                )}
              </div>
            </div>
          ) : (
            <div className="flex flex-col h-full space-y-2">
              <div className="flex items-center gap-1.5">
                <div className="relative flex-1">
                  <Search className="size-3 absolute left-2 top-2 text-white/40" />
                  <input
                    type="text"
                    value={fileSearchQuery}
                    onChange={(e) => setFileSearchQuery(e.target.value)}
                    placeholder="Filter workspace files…"
                    className="w-full h-7 pl-6 pr-2 rounded bg-white/5 border border-white/8 text-[11.5px] text-white placeholder-white/30 focus:outline-none focus:border-workbench-accent transition"
                  />
                </div>
                <Tooltip
                  content={
                    onlyChangedFiles
                      ? 'Showing changed files only (Click to show all)'
                      : `Filter to changed files (${diffSummary?.files?.length ?? 0} files)`
                  }
                  position="bottom"
                >
                  <button
                    type="button"
                    onClick={() => setOnlyChangedFiles((prev) => !prev)}
                    className={`h-7 px-2 flex items-center gap-1 rounded text-[11px] font-medium transition cursor-pointer shrink-0 border ${
                      onlyChangedFiles
                        ? 'bg-amber-500/20 text-amber-300 border-amber-500/30 font-semibold'
                        : 'bg-white/5 hover:bg-white/10 text-white/60 hover:text-white border-white/8'
                    }`}
                    aria-label="Filter changed files"
                  >
                    <Filter className="size-3" />
                    <span>{diffSummary?.files?.length ?? 0}</span>
                  </button>
                </Tooltip>
                <ActionIcon
                  icon={RefreshCw}
                  tooltip="Refresh file tree & git status"
                  size="sm"
                  onClick={handleRefreshWorkspaceFilesAndDiff}
                  disabled={filesLoading || diffLoading}
                  className={filesLoading || diffLoading ? 'animate-spin' : ''}
                  ariaLabel="Refresh files and diff"
                />
              </div>

              <div className="flex-1 overflow-y-auto space-y-0.5 pt-1">
                {filesLoading && !fileTree ? (
                  <div className="flex items-center justify-center py-8 text-white/40 text-xs">
                    <span>Scanning files…</span>
                  </div>
                ) : filteredEntries.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-8 text-white/40 text-center text-xs">
                    <Folder className="size-8 stroke-1 text-white/20 mb-2" />
                    <span>No matching files in workspace</span>
                  </div>
                ) : (
                  filteredEntries.map((node: WorkspaceFileEntry) => {
                    const depth = Math.max(0, node.path.split('/').length - 1);
                    const diffEntry = !node.is_dir ? diffMap.get(node.path) : undefined;
                    return (
                      <div
                        key={node.path}
                        onClick={(e) => {
                          if (node.is_dir) {
                            toggleFolder(node.path);
                          } else if (e.altKey) {
                            openFileInspector(node.path);
                          } else {
                            // Open directly in Center Canvas (Orca/VS Code model)
                            onOpenTab?.('files', `custos://file?path=${encodeURIComponent(node.path)}`, node.name);
                          }
                        }}
                        className="group flex items-center gap-1.5 px-2 py-1 rounded hover:bg-white/6 cursor-pointer text-white/80 hover:text-white transition"
                        style={{ paddingLeft: `${Math.min(depth * 12 + 8, 48)}px` }}
                        title={node.is_dir ? node.name : `${node.name} (Click to open tab, ⌥Click to preview in panel)`}
                      >
                        {node.is_dir ? (
                          <>
                            {expandedFolders.has(node.path) ? (
                              <ChevronDown className="size-3 text-white/40 shrink-0" />
                            ) : (
                              <ChevronRight className="size-3 text-white/40 shrink-0" />
                            )}
                            <Folder className="size-3.5 text-amber-400/80 shrink-0" />
                          </>
                        ) : (
                          <>
                            <div className="size-3 shrink-0" />
                            <FileCode className="size-3.5 text-sky-400/70 shrink-0" />
                          </>
                        )}
                        <span className="truncate font-mono text-[11px] flex-1">{node.name}</span>

                        {/* Git diff status indicator badge & Diff button */}
                        {diffEntry && (
                          <div className="flex items-center gap-1 shrink-0 ml-auto pl-1">
                            {diffEntry.additions > 0 && (
                              <span className="font-mono text-[9px] text-emerald-400">+{diffEntry.additions}</span>
                            )}
                            {diffEntry.deletions > 0 && (
                              <span className="font-mono text-[9px] text-rose-400">-{diffEntry.deletions}</span>
                            )}
                            <span
                              className={`font-mono text-[9px] font-bold px-1 py-0.2 rounded border leading-none ${
                                diffEntry.status === 'M'
                                  ? 'text-amber-400 bg-amber-400/10 border-amber-400/30'
                                  : diffEntry.status === 'A' || diffEntry.status === '??'
                                  ? 'text-emerald-400 bg-emerald-400/10 border-emerald-400/30'
                                  : diffEntry.status === 'D'
                                  ? 'text-rose-400 bg-rose-400/10 border-rose-400/30'
                                  : 'text-sky-400 bg-sky-400/10 border-sky-400/30'
                              }`}
                              title={`Git Status: ${diffEntry.status}`}
                            >
                              {diffEntry.status}
                            </span>
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onOpenTab?.(
                                  'files',
                                  `custos://diff?path=${encodeURIComponent(node.path)}`,
                                  `Diff: ${node.name}`
                                );
                              }}
                              className="opacity-0 group-hover:opacity-100 size-5 flex items-center justify-center rounded hover:bg-white/10 text-white/50 hover:text-white transition cursor-pointer"
                              title="Open Diff in Center Canvas"
                            >
                              <FileDiff className="size-3 text-workbench-accent" />
                            </button>
                          </div>
                        )}

                        {/* Quick side panel preview button on hover */}
                        {!node.is_dir && (
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              openFileInspector(node.path);
                            }}
                            className="opacity-0 group-hover:opacity-100 size-5 flex items-center justify-center rounded hover:bg-white/10 text-white/40 hover:text-white transition cursor-pointer"
                            title="Preview file in side panel (⌥Click)"
                          >
                            <SlidersHorizontal className="size-3" />
                          </button>
                        )}
                      </div>
                    );
                  })
                )}
              </div>
            </div>
          )
        )}

        {/* ── ENGINEERING: 2. SOURCE CONTROL (ORCA STYLE COMMIT & CHANGES) ── */}
        {activeTabId === 'changes' && (
          selectedDiffPath ? (
            /* ── RICH EMBEDDED FILE DIFF INSPECTOR ── */
            <div className="flex flex-col h-full space-y-2">
              {/* Diff Header */}
              <div className="flex items-center justify-between pb-2 border-b border-white/8 shrink-0">
                <div className="flex items-center gap-1.5 min-w-0">
                  <ActionIcon
                    icon={ChevronLeft}
                    tooltip="Back to Changes list"
                    size="sm"
                    onClick={() => setSelectedDiffPath(null)}
                    ariaLabel="Back to changes"
                  />
                  <span className="text-white/20">|</span>
                  <span className="truncate font-mono text-white/95 text-[11.5px] font-semibold" title={selectedDiffPath}>
                    {selectedDiffPath.split('/').pop()}
                  </span>
                  {currentDiffEntry && (
                    <span
                      className={`font-mono text-[9px] font-bold px-1 py-0.2 rounded border leading-none ${
                        currentDiffEntry.status === 'M'
                          ? 'text-amber-400 bg-amber-400/10 border-amber-400/30'
                          : currentDiffEntry.status === 'A' || currentDiffEntry.status === '??'
                          ? 'text-emerald-400 bg-emerald-400/10 border-emerald-400/30'
                          : currentDiffEntry.status === 'D'
                          ? 'text-rose-400 bg-rose-400/10 border-rose-400/30'
                          : 'text-sky-400 bg-sky-400/10 border-sky-400/30'
                      }`}
                    >
                      {currentDiffEntry.status}
                    </span>
                  )}
                </div>

                <div className="flex items-center gap-1 shrink-0">
                  {/* Prev / Next file arrows */}
                  {changedFilesList.length > 1 && (
                    <div className="flex items-center gap-0.5 bg-white/5 border border-white/8 rounded px-1 py-0.5 text-[10px]">
                      <ActionIcon
                        icon={ChevronUp}
                        tooltip="Previous changed file"
                        size="xs"
                        onClick={handlePrevDiff}
                        ariaLabel="Previous changed file"
                      />
                      <span className="font-mono text-[9px] text-white/50 px-1 select-none">
                        {currentDiffIndex >= 0 ? currentDiffIndex + 1 : 1}/{changedFilesList.length}
                      </span>
                      <ActionIcon
                        icon={ChevronDown}
                        tooltip="Next changed file"
                        size="xs"
                        onClick={handleNextDiff}
                        ariaLabel="Next changed file"
                      />
                    </div>
                  )}

                  {/* Stage / Unstage Action */}
                  {selectedDiffIsStaged ? (
                    <ActionIcon
                      icon={Minus}
                      tooltip="Unstage file from commit"
                      size="sm"
                      onClick={(e) => void handleUnstageFile(selectedDiffPath, e)}
                      disabled={isStagingAction}
                      className="bg-white/10 hover:bg-white/15 text-white"
                      ariaLabel="Unstage file"
                    />
                  ) : (
                    <>
                      <ActionIcon
                        icon={Plus}
                        tooltip="Stage file for commit"
                        size="sm"
                        onClick={(e) => void handleStageFile(selectedDiffPath, e)}
                        disabled={isStagingAction}
                        className="bg-emerald-600/80 hover:bg-emerald-600 text-white"
                        ariaLabel="Stage file"
                      />
                      <ActionIcon
                        icon={RotateCcw}
                        tooltip="Discard local changes in this file"
                        size="sm"
                        variant="danger"
                        onClick={(e) => {
                          void handleDiscardFile(selectedDiffPath, e).then(() => {
                            setSelectedDiffPath(null);
                          });
                        }}
                        disabled={isStagingAction}
                        ariaLabel="Discard changes"
                      />
                    </>
                  )}

                  {/* Copy Diff Button */}
                  <ActionIcon
                    icon={copiedDiff ? Check : Copy}
                    tooltip={copiedDiff ? "Copied raw diff!" : "Copy raw git diff"}
                    size="sm"
                    onClick={handleCopyDiff}
                    className={copiedDiff ? "text-emerald-400" : ""}
                    ariaLabel="Copy raw diff"
                  />

                  {/* Pop out to Center Canvas Button */}
                  <ActionIcon
                    icon={Maximize2}
                    tooltip="Open diff in workbench canvas"
                    size="sm"
                    onClick={() =>
                      onOpenTab?.(
                        'files',
                        `custos://diff?path=${encodeURIComponent(selectedDiffPath)}`,
                        `Diff: ${selectedDiffPath.split('/').pop()}`
                      )
                    }
                    ariaLabel="Open in canvas"
                  />
                </div>
              </div>

              {/* Subheader: Full file path & Diff line stats */}
              <div className="flex items-center justify-between text-[10.5px] text-white/40 font-mono px-1 shrink-0">
                <span className="truncate max-w-[220px]" title={selectedDiffPath}>
                  {selectedDiffPath}
                </span>
                {diffStats && (
                  <span className="flex items-center gap-1 shrink-0">
                    <span className="text-emerald-400 font-semibold">+{diffStats.added}</span>
                    <span className="text-rose-400 font-semibold">-{diffStats.deleted}</span>
                  </span>
                )}
              </div>

              {/* Diff Content View */}
              <div className="flex-1 overflow-auto rounded border border-white/6 bg-black/40 p-2 font-mono text-[11px] leading-relaxed selection:bg-white/20 select-text">
                {loadingFileDiff ? (
                  <div className="flex items-center justify-center py-12 text-white/40 text-xs">
                    <RefreshCw className="size-4 animate-spin mr-2" />
                    <span>Loading diff…</span>
                  </div>
                ) : fileDiff && fileDiff.diff ? (
                  <div className="space-y-0.5">
                    {fileDiff.diff.split('\n').map((line, idx) => {
                      let lineClass = 'text-white/60';
                      let bgClass = '';
                      if (line.startsWith('@@')) {
                        lineClass = 'text-cyan-400 font-semibold';
                        bgClass = 'bg-cyan-950/30 border-y border-cyan-800/20';
                      } else if (line.startsWith('+') && !line.startsWith('+++')) {
                        lineClass = 'text-emerald-300';
                        bgClass = 'bg-emerald-950/40';
                      } else if (line.startsWith('-') && !line.startsWith('---')) {
                        lineClass = 'text-rose-300';
                        bgClass = 'bg-rose-950/40';
                      }
                      return (
                        <div key={idx} className={`px-2 py-0.5 flex items-start ${bgClass}`}>
                          <span className="w-6 shrink-0 text-white/25 text-right pr-2 select-none text-[10px]">
                            {idx + 1}
                          </span>
                          <span className={`${lineClass} whitespace-pre-wrap break-all flex-1`}>{line}</span>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <div className="flex flex-col items-center justify-center py-12 text-white/40 text-xs text-center">
                    <FileCode className="size-8 stroke-1 text-white/20 mb-2" />
                    <span>No differences found for this file</span>
                  </div>
                )}
              </div>
            </div>
          ) : (
            /* ── OVERVIEW: COMMIT COMPOSER & CHANGE SECTIONS ── */
            <div className="flex flex-col h-full space-y-3">
              {/* Header: Branch name & Line Delta Stats */}
              <div className="flex items-center justify-between pb-1.5 border-b border-white/8 shrink-0">
                <div className="flex items-center gap-1.5 text-xs text-white/80 min-w-0">
                  <GitBranch className="size-3.5 text-workbench-accent shrink-0" />
                  <span className="font-mono font-medium text-white/95 truncate">
                    {diffSummary?.branch || activeWorkspace?.active_branch || 'HEAD'}
                  </span>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  {diffSummary && (
                    <span className="font-mono text-[10px] text-white/50">
                      <span className="text-emerald-400">+{diffSummary.total_additions || 0}</span>
                      {' / '}
                      <span className="text-rose-400">-{diffSummary.total_deletions || 0}</span>
                    </span>
                  )}
                  <button
                    type="button"
                    onClick={handleRefreshWorkspaceFilesAndDiff}
                    disabled={diffLoading || isStagingAction}
                    className="size-6 flex items-center justify-center rounded text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer"
                    title="Refresh git status"
                  >
                    <RefreshCw className={`size-3 ${diffLoading || isStagingAction ? 'animate-spin' : ''}`} />
                  </button>
                </div>
              </div>

              {/* Commit Message Box (Orca style Commit Composer) */}
              <div className="space-y-1.5 shrink-0">
                <div className="relative">
                  <textarea
                    rows={3}
                    value={commitMessage}
                    onChange={(e) => setCommitMessage(e.target.value)}
                    onKeyDown={(e) => {
                      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
                        void handleCommit();
                      }
                    }}
                    placeholder="Message (⌘Enter to commit)"
                    className="w-full resize-none p-2 rounded-md bg-white/5 border border-white/10 text-xs text-white placeholder-white/30 focus:outline-none focus:border-workbench-accent focus:ring-1 focus:ring-workbench-accent/40 transition"
                  />
                </div>

                {commitError && (
                  <div className="flex items-center gap-1.5 p-2 rounded bg-rose-500/10 border border-rose-500/20 text-rose-300 text-[11px]">
                    <AlertCircle className="size-3.5 shrink-0 text-rose-400" />
                    <span className="truncate">{commitError}</span>
                  </div>
                )}

                {commitSuccess && (
                  <div className="flex items-center gap-1.5 p-2 rounded bg-emerald-500/10 border border-emerald-500/20 text-emerald-300 text-[11px]">
                    <CheckCircle2 className="size-3.5 shrink-0 text-emerald-400" />
                    <span>{commitSuccess}</span>
                  </div>
                )}

                <button
                  type="button"
                  onClick={handleCommit}
                  disabled={isCommitting || !commitMessage.trim() || !diffSummary?.files?.length}
                  className="w-full flex items-center justify-center gap-1.5 py-1.5 px-3 rounded-md bg-white/10 hover:bg-white/20 disabled:opacity-35 disabled:hover:bg-white/10 text-white text-xs font-medium transition cursor-pointer disabled:cursor-not-allowed shadow-xs"
                >
                  {isCommitting ? (
                    <>
                      <RefreshCw className="size-3.5 animate-spin" />
                      <span>Committing changes…</span>
                    </>
                  ) : (
                    <>
                      <GitCommit className="size-3.5" />
                      <span>
                        {diffSummary?.files?.some((f) => f.is_staged || f.isStaged)
                          ? `Commit Staged (${diffSummary.files.filter((f) => f.is_staged || f.isStaged).length})`
                          : `Commit to ${diffSummary?.branch || activeWorkspace?.active_branch || 'HEAD'}`}
                      </span>
                      <span className="ml-auto font-mono text-[10px] text-white/40">⌘↵</span>
                    </>
                  )}
                </button>
              </div>

              {/* Changes Lists (Staged Changes & Working Tree Changes) */}
              <div className="flex-1 overflow-y-auto space-y-3 pt-1">
                {diffLoading && !diffSummary ? (
                  <div className="flex items-center justify-center py-8 text-white/40 text-xs">
                    <span>Checking git status…</span>
                  </div>
                ) : !diffSummary || !diffSummary.files || diffSummary.files.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-10 text-white/40 text-center text-xs">
                    <CheckCircle2 className="size-8 stroke-1 text-emerald-400/70 mb-2" />
                    <span className="text-white/85 font-medium">Working tree clean</span>
                    <span className="text-[11px] text-white/40 mt-0.5">No staged or uncommitted changes</span>
                  </div>
                ) : (
                  <>
                    {/* STAGED CHANGES SECTION */}
                    {diffSummary.files.some((f) => f.is_staged || f.isStaged) && (
                      <div className="space-y-1">
                        <div className="flex items-center justify-between text-[11px] font-semibold text-white/60 px-1 py-0.5 uppercase tracking-wider">
                          <button
                            type="button"
                            onClick={() => setCollapsedStaged((prev) => !prev)}
                            className="flex items-center gap-1 hover:text-white transition cursor-pointer"
                          >
                            {collapsedStaged ? <ChevronRight className="size-3" /> : <ChevronDown className="size-3" />}
                            <span>Staged Changes ({diffSummary.files.filter((f) => f.is_staged || f.isStaged).length})</span>
                          </button>
                          <ActionIcon
                            icon={Minus}
                            tooltip="Unstage all staged files"
                            size="sm"
                            onClick={handleUnstageAll}
                            disabled={isStagingAction}
                            ariaLabel="Unstage all"
                          />
                        </div>

                        {!collapsedStaged && (
                          <div className="space-y-0.5">
                            {diffSummary.files
                              .filter((f) => f.is_staged || f.isStaged)
                              .map((file: WorkspaceDiffEntry) => (
                                <div
                                  key={`staged-${file.path}`}
                                  onClick={() => openDiffInspector(file.path, true)}
                                  onDoubleClick={(e) => {
                                    e.stopPropagation();
                                    onOpenTab?.(
                                      'files',
                                      `custos://diff?path=${encodeURIComponent(file.path)}`,
                                      `Diff: ${file.path.split('/').pop()}`
                                    );
                                  }}
                                  className="group flex items-center justify-between px-2 py-1.5 rounded bg-white/[0.04] hover:bg-white/8 border border-white/6 transition cursor-pointer"
                                  title="Click to inspect in panel, double-click to open in canvas"
                                >
                                  <div className="flex items-center gap-1.5 min-w-0 flex-1">
                                    <span
                                      className={`font-mono text-[9px] font-bold px-1 py-0.2 rounded border leading-none ${
                                        file.status === 'M'
                                          ? 'text-amber-400 bg-amber-400/10 border-amber-400/30'
                                          : file.status === 'A' || file.status === '??'
                                          ? 'text-emerald-400 bg-emerald-400/10 border-emerald-400/30'
                                          : file.status === 'D'
                                          ? 'text-rose-400 bg-rose-400/10 border-rose-400/30'
                                          : 'text-sky-400 bg-sky-400/10 border-sky-400/30'
                                      }`}
                                    >
                                      {file.status}
                                    </span>
                                    <span className="truncate font-mono text-[11px] text-white/90">{file.path}</span>
                                  </div>
                                  <div className="flex items-center gap-1 shrink-0 ml-1">
                                    <div className="flex items-center gap-1 font-mono text-[9.5px]">
                                      {file.additions > 0 && <span className="text-emerald-400">+{file.additions}</span>}
                                      {file.deletions > 0 && <span className="text-rose-400">-{file.deletions}</span>}
                                    </div>
                                    <ActionIcon
                                      icon={Maximize2}
                                      tooltip="Open diff in canvas"
                                      size="xs"
                                      onClick={(e) => {
                                        e.stopPropagation();
                                        onOpenTab?.(
                                          'files',
                                          `custos://diff?path=${encodeURIComponent(file.path)}`,
                                          `Diff: ${file.path.split('/').pop()}`
                                        );
                                      }}
                                      className="opacity-0 group-hover:opacity-100"
                                      ariaLabel="Open in canvas"
                                    />
                                    <ActionIcon
                                      icon={Minus}
                                      tooltip="Unstage file"
                                      size="xs"
                                      onClick={(e) => void handleUnstageFile(file.path, e)}
                                      disabled={isStagingAction}
                                      className="opacity-0 group-hover:opacity-100"
                                      ariaLabel="Unstage file"
                                    />
                                  </div>
                                </div>
                              ))}
                          </div>
                        )}
                      </div>
                    )}

                    {/* UNSTAGED / WORKING TREE CHANGES SECTION */}
                    {diffSummary.files.some((f) => !f.is_staged && !f.isStaged) && (
                      <div className="space-y-1">
                        <div className="flex items-center justify-between text-[11px] font-semibold text-white/60 px-1 py-0.5 uppercase tracking-wider">
                          <button
                            type="button"
                            onClick={() => setCollapsedUnstaged((prev) => !prev)}
                            className="flex items-center gap-1 hover:text-white transition cursor-pointer"
                          >
                            {collapsedUnstaged ? <ChevronRight className="size-3" /> : <ChevronDown className="size-3" />}
                            <span>Changes ({diffSummary.files.filter((f) => !f.is_staged && !f.isStaged).length})</span>
                          </button>
                          <div className="flex items-center gap-1">
                            <ActionIcon
                              icon={Plus}
                              tooltip="Stage all modified files"
                              size="sm"
                              onClick={handleStageAll}
                              disabled={isStagingAction}
                              ariaLabel="Stage all"
                            />
                            <ActionIcon
                              icon={RotateCcw}
                              tooltip="Discard all unstaged local changes"
                              size="sm"
                              variant="danger"
                              onClick={handleDiscardAll}
                              disabled={isStagingAction}
                              ariaLabel="Discard all"
                            />
                          </div>
                        </div>

                        {!collapsedUnstaged && (
                          <div className="space-y-0.5">
                            {diffSummary.files
                              .filter((f) => !f.is_staged && !f.isStaged)
                              .map((file: WorkspaceDiffEntry) => (
                                <div
                                  key={`unstaged-${file.path}`}
                                  onClick={() => openDiffInspector(file.path, false)}
                                  onDoubleClick={(e) => {
                                    e.stopPropagation();
                                    onOpenTab?.(
                                      'files',
                                      `custos://diff?path=${encodeURIComponent(file.path)}`,
                                      `Diff: ${file.path.split('/').pop()}`
                                    );
                                  }}
                                  className="group flex items-center justify-between px-2 py-1.5 rounded bg-white/[0.04] hover:bg-white/8 border border-white/6 transition cursor-pointer"
                                  title="Click to inspect in panel, double-click to open in canvas"
                                >
                                  <div className="flex items-center gap-1.5 min-w-0 flex-1">
                                    <span
                                      className={`font-mono text-[9px] font-bold px-1 py-0.2 rounded border leading-none ${
                                        file.status === 'M'
                                          ? 'text-amber-400 bg-amber-400/10 border-amber-400/30'
                                          : file.status === 'A' || file.status === '??'
                                          ? 'text-emerald-400 bg-emerald-400/10 border-emerald-400/30'
                                          : file.status === 'D'
                                          ? 'text-rose-400 bg-rose-400/10 border-rose-400/30'
                                          : 'text-sky-400 bg-sky-400/10 border-sky-400/30'
                                      }`}
                                    >
                                      {file.status}
                                    </span>
                                    <span className="truncate font-mono text-[11px] text-white/90">{file.path}</span>
                                  </div>
                                  <div className="flex items-center gap-1 shrink-0 ml-1">
                                    <div className="flex items-center gap-1 font-mono text-[9.5px]">
                                      {file.additions > 0 && <span className="text-emerald-400">+{file.additions}</span>}
                                      {file.deletions > 0 && <span className="text-rose-400">-{file.deletions}</span>}
                                    </div>
                                    <ActionIcon
                                      icon={Maximize2}
                                      tooltip="Open diff in canvas"
                                      size="xs"
                                      onClick={(e) => {
                                        e.stopPropagation();
                                        onOpenTab?.(
                                          'files',
                                          `custos://diff?path=${encodeURIComponent(file.path)}`,
                                          `Diff: ${file.path.split('/').pop()}`
                                        );
                                      }}
                                      className="opacity-0 group-hover:opacity-100"
                                      ariaLabel="Open in canvas"
                                    />
                                    <ActionIcon
                                      icon={Plus}
                                      tooltip="Stage file"
                                      size="xs"
                                      onClick={(e) => void handleStageFile(file.path, e)}
                                      disabled={isStagingAction}
                                      className="opacity-0 group-hover:opacity-100"
                                      ariaLabel="Stage file"
                                    />
                                    <ActionIcon
                                      icon={RotateCcw}
                                      tooltip="Discard local changes"
                                      size="xs"
                                      variant="danger"
                                      onClick={(e) => void handleDiscardFile(file.path, e)}
                                      disabled={isStagingAction}
                                      className="opacity-0 group-hover:opacity-100"
                                      ariaLabel="Discard changes"
                                    />
                                  </div>
                                </div>
                              ))}
                          </div>
                        )}
                      </div>
                    )}
                  </>
                )}
              </div>
            </div>
          )
        )}

        {/* ── 3. CONTEXT / PROVENANCE INSPECTOR (All Workbenches) ── */}
        {activeTabId === 'inspector' && (
          <div className="space-y-4">
            {/* Research Investigation Context */}
            {mode === 'research' && (
              <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
                <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                  <span className="flex items-center gap-1.5">
                    <BookOpen className="size-3 text-workbench-accent" />
                    Investigation Context
                  </span>
                  <span className="font-mono text-[9px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 font-semibold">
                    Live Ledger
                  </span>
                </div>
                <p className="font-medium text-white/95 text-xs">
                  Biomolecular Structure & Variant Impact
                </p>
                <p className="text-[11px] text-white/50 leading-relaxed">
                  AlphaFold structure predictions correlated with published assays and PDB experimental coordinates.
                </p>
                <div className="flex items-center justify-between pt-1 border-t border-white/6 text-[10.5px]">
                  <span className="text-white/40">Reproducibility Seed</span>
                  <span className="font-mono text-white/80">42 (Deterministic)</span>
                </div>
              </div>
            )}

            {/* Copilot Executive Governance Context */}
            {mode === 'copilot' && (
              <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
                <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                  <span className="flex items-center gap-1.5">
                    <ShieldCheck className="size-3 text-workbench-accent" />
                    Governance Policy
                  </span>
                  <span className="font-mono text-[9px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 font-semibold">
                    Enforced
                  </span>
                </div>
                <p className="font-medium text-white/95 text-xs">
                  Human-in-the-Loop Action Contract
                </p>
                <p className="text-[11px] text-white/50 leading-relaxed">
                  Autonomous assistant operates in draft-and-stage mode. External transmissions require manual dispatch sign-off.
                </p>
                <div className="flex items-center justify-between pt-1 border-t border-white/6 text-[10.5px]">
                  <span className="text-white/40">Autonomy Bound</span>
                  <span className="font-mono text-white/80">Guarded Outbox Only</span>
                </div>
              </div>
            )}

            {/* Workspace Section */}
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span className="flex items-center gap-1.5">
                  <HardDrive className="size-3 text-workbench-accent" />
                  Workspace Scope
                </span>
                <span className="font-mono text-[9.5px] text-white/40">{activeWorkspace?.id || 'none'}</span>
              </div>
              <p className="font-semibold text-white/95 text-[12.5px] truncate">
                {activeWorkspace?.name || 'Default Execution Workspace'}
              </p>
              {activeWorkspace?.path && (
                <p className="font-mono text-[10.5px] text-white/50 break-all bg-black/30 p-1.5 rounded border border-white/6">
                  {activeWorkspace.path}
                </p>
              )}
              {activeWorkspace?.active_branch && (
                <div className="flex items-center gap-1.5 text-white/60 text-[11px]">
                  <GitBranch className="size-3 text-white/40" />
                  <span>Branch:</span>
                  <span className="font-mono text-white/90">{activeWorkspace.active_branch}</span>
                </div>
              )}
            </div>

            {/* Task Contract Section */}
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span className="flex items-center gap-1.5">
                  <CheckSquare className="size-3 text-amber-400" />
                  Task Objective
                </span>
                {activeTask?.status && (
                  <span className="font-mono text-[9px] px-1.5 py-0.5 rounded capitalize bg-white/10 text-white/70">
                    {activeTask.status}
                  </span>
                )}
              </div>
              <p className="font-medium text-white/90 text-xs">
                {activeTask?.title || 'No active Task bound to this context'}
              </p>
              {(activeTask?.contract?.description || activeTask?.metadata?.description) && (
                <p className="text-[11px] text-white/50 line-clamp-3 leading-relaxed">
                  {String(activeTask.contract?.description || activeTask.metadata?.description)}
                </p>
              )}
            </div>

            {/* Conversation & Model Metrics */}
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span className="flex items-center gap-1.5">
                  <Zap className="size-3 text-sky-400" />
                  AI Session & Usage
                </span>
                <span className="font-mono text-[10px] text-white/40 capitalize">{mode}</span>
              </div>
              <p className="font-medium text-white/90 truncate">{activeSession?.title || 'Active Session'}</p>
              <div className="grid grid-cols-2 gap-2 pt-1 font-mono text-[10.5px]">
                <div className="bg-black/25 p-1.5 rounded border border-white/6">
                  <span className="text-white/40 block text-[9.5px]">Model</span>
                  <span className="text-white/90 truncate block">{activeSession?.model || 'Mediated'}</span>
                </div>
                <div className="bg-black/25 p-1.5 rounded border border-white/6">
                  <span className="text-white/40 block text-[9.5px]">Tokens</span>
                  <span className="text-white/90 block">
                    {sessionMetrics?.total_tokens ? `${sessionMetrics.total_tokens.toLocaleString()}` : '0'}
                  </span>
                </div>
              </div>
            </div>

            {/* Active Tab Provenance */}
            {activeTab && (
              <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
                <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                  <span>Focused Canvas Tab</span>
                  <span className="font-mono text-[10px] text-workbench-accent">{activeTab.type}</span>
                </div>
                <p className="font-medium text-white/90 truncate">{activeTab.title}</p>
                {activeTab.url && (
                  <p className="font-mono text-[10px] text-white/40 truncate bg-black/25 p-1.5 rounded">
                    {activeTab.url}
                  </p>
                )}
              </div>
            )}
          </div>
        )}

        {/* ── RESEARCH: COMPUTE ENGINE & HARDWARE ── */}
        {activeTabId === 'engine' && (
          <div className="space-y-3">
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2.5">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span className="flex items-center gap-1.5">
                  <Cpu className="size-3 text-emerald-400" />
                  Jupyter Kernel Runtime
                </span>
                <span className="flex items-center gap-1 text-emerald-400 font-mono text-[10px]">
                  <CheckCircle2 className="size-2.5" />
                  Idle
                </span>
              </div>
              <div className="space-y-1.5 text-xs font-mono">
                <div className="flex items-center justify-between py-1 border-b border-white/6">
                  <span className="text-white/60">Kernel Engine</span>
                  <span className="text-white/90 text-[11px]">IPython 8.24 (Py 3.11)</span>
                </div>
                <div className="flex items-center justify-between py-1 border-b border-white/6">
                  <span className="text-white/60">Hardware Accelerator</span>
                  <span className="text-emerald-400 text-[11px]">Apple Metal MPS (3.2 GB)</span>
                </div>
                <div className="flex items-center justify-between py-1 border-b border-white/6">
                  <span className="text-white/60">Mol* WebGL</span>
                  <span className="text-emerald-400 text-[11px]">Hardware Accelerated</span>
                </div>
                <div className="flex items-center justify-between py-1">
                  <span className="text-white/60">Durable Ledger</span>
                  <span className="text-white/90 text-[11px]">Deterministic Seeds</span>
                </div>
              </div>
            </div>

            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <span className="text-[10.5px] uppercase tracking-wider font-semibold text-white/50">
                Scientific API Sidecars
              </span>
              <div className="space-y-1 font-mono text-[11px]">
                <div className="flex items-center justify-between p-1.5 rounded bg-black/25 border border-white/6">
                  <span className="text-white/80">AlphaFold DB API</span>
                  <span className="text-emerald-400 text-[10px]">Connected</span>
                </div>
                <div className="flex items-center justify-between p-1.5 rounded bg-black/25 border border-white/6">
                  <span className="text-white/80">NCBI / PubMed REST</span>
                  <span className="text-emerald-400 text-[10px]">Online</span>
                </div>
                <div className="flex items-center justify-between p-1.5 rounded bg-black/25 border border-white/6">
                  <span className="text-white/80">RCSB PDB Coordinates</span>
                  <span className="text-emerald-400 text-[10px]">Online</span>
                </div>
              </div>
            </div>

            <button
              type="button"
              onClick={() => onOpenTab?.('lab')}
              className="w-full py-2 px-3 rounded-lg border border-white/10 bg-white/5 hover:bg-white/10 text-white text-xs font-medium transition cursor-pointer flex items-center justify-center gap-2"
            >
              <Play className="size-3.5 text-workbench-accent" />
              <span>Inspect Interactive Lab Notebook</span>
            </button>
          </div>
        )}

        {/* ── RESEARCH: VARIABLES & NOTEBOOK OUTLINE ── */}
        {activeTabId === 'variables' && (
          <div className="space-y-3">
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span>Kernel Namespace (4 Variables)</span>
                <span className="font-mono text-emerald-400 text-[10px]">Active</span>
              </div>
              <div className="space-y-1 pt-1 font-mono text-[11px]">
                <div className="p-1.5 rounded bg-black/25 border border-white/6">
                  <div className="flex items-center justify-between text-sky-400">
                    <span className="font-semibold">df</span>
                    <span className="text-[10px] text-white/40">pandas.DataFrame</span>
                  </div>
                  <div className="text-[10px] text-white/50">1,200 rows × 14 columns</div>
                </div>

                <div className="p-1.5 rounded bg-black/25 border border-white/6">
                  <div className="flex items-center justify-between text-purple-400">
                    <span className="font-semibold">embeddings</span>
                    <span className="text-[10px] text-white/40">torch.Tensor</span>
                  </div>
                  <div className="text-[10px] text-white/50">Shape: [1200, 768] · Float32</div>
                </div>

                <div className="p-1.5 rounded bg-black/25 border border-white/6">
                  <div className="flex items-center justify-between text-emerald-400">
                    <span className="font-semibold">p_values</span>
                    <span className="text-[10px] text-white/40">numpy.ndarray</span>
                  </div>
                  <div className="text-[10px] text-white/50">Min: 0.00012, Max: 0.048</div>
                </div>

                <div className="p-1.5 rounded bg-black/25 border border-white/6">
                  <div className="flex items-center justify-between text-amber-400">
                    <span className="font-semibold">model</span>
                    <span className="text-[10px] text-white/40">PyTorch Module</span>
                  </div>
                  <div className="text-[10px] text-white/50">14.2M parameters · Eval mode</div>
                </div>
              </div>
            </div>

            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <span className="text-[10.5px] uppercase tracking-wider font-semibold text-white/50">
                Notebook Sections
              </span>
              <div className="space-y-1 text-xs">
                <div className="p-1.5 rounded bg-white/[0.04] hover:bg-white/8 border border-white/6 cursor-pointer transition">
                  <div className="text-sky-400 font-mono text-[10px]">§ 1.0</div>
                  <div className="text-white/90 truncate font-medium">Problem Formulation & Hypotheses</div>
                </div>
                <div className="p-1.5 rounded bg-white/[0.04] hover:bg-white/8 border border-white/6 cursor-pointer transition">
                  <div className="text-sky-400 font-mono text-[10px]">§ 2.0</div>
                  <div className="text-white/90 truncate font-medium">Dataset Preprocessing & Normalization</div>
                </div>
                <div className="p-1.5 rounded bg-white/[0.04] hover:bg-white/8 border border-white/6 cursor-pointer transition">
                  <div className="text-sky-400 font-mono text-[10px]">§ 3.0</div>
                  <div className="text-white/90 truncate font-medium">Structural Variant Inference & P-Values</div>
                </div>
                <div className="p-1.5 rounded bg-white/[0.04] hover:bg-white/8 border border-white/6 cursor-pointer transition">
                  <div className="text-sky-400 font-mono text-[10px]">§ 4.0</div>
                  <div className="text-white/90 truncate font-medium">Residual Plots & Figure Export</div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* ── RESEARCH: CITATION & DOI FORMATTER ── */}
        {activeTabId === 'citation' && (
          <div className="space-y-3">
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span className="flex items-center gap-1.5">
                  <Quote className="size-3 text-workbench-accent" />
                  Active Citation
                </span>
                <span className="font-mono text-[10px] text-white/40">Nature 2024</span>
              </div>
              <p className="font-medium text-white/95 text-xs">
                Accurate structure prediction of biomolecular interactions
              </p>
              <p className="text-[11px] text-white/50">
                Abramson, Adler, Dunger, Evans, Green, Pritzel et al.
              </p>
              <div className="font-mono text-[10px] text-white/40 bg-black/25 p-1 rounded">
                doi:10.1038/s41586-024-07487-w
              </div>
            </div>

            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-[10.5px] uppercase tracking-wider font-semibold text-white/50">
                  BibTeX Record
                </span>
                {(activeSession as any)?.metadata?.bibtex && (
                  <button
                    type="button"
                    onClick={() =>
                      handleCopyCitation(
                        'bibtex',
                        (activeSession as any)?.metadata?.bibtex
                      )
                    }
                    className="flex items-center gap-1 text-[10.5px] text-workbench-accent hover:underline cursor-pointer"
                  >
                    {copiedFormat === 'bibtex' ? <Check className="size-3 text-emerald-400" /> : <Copy className="size-3" />}
                    <span>{copiedFormat === 'bibtex' ? 'Copied!' : 'Copy BibTeX'}</span>
                  </button>
                )}
              </div>
              {(activeSession as any)?.metadata?.bibtex ? (
                <pre className="p-2 rounded bg-black/40 border border-white/6 font-mono text-[10px] text-white/70 overflow-x-auto no-scrollbar">
                  {(activeSession as any)?.metadata?.bibtex}
                </pre>
              ) : (
                <div className="p-3 text-center text-xs text-white/40 border border-dashed border-white/10 rounded-lg">
                  No BibTeX citation available for current session.
                </div>
              )}
            </div>
          </div>
        )}

        {/* ── COPILOT: GOVERNANCE & AUTONOMY BOUNDS ── */}
        {activeTabId === 'governance' && (
          <div className="space-y-3">
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2.5">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span className="flex items-center gap-1.5">
                  <ShieldCheck className="size-3 text-emerald-400" />
                  Policy Engine
                </span>
                <span className="font-mono text-[10px] text-emerald-400">Strict Human-in-Loop</span>
              </div>
              <p className="text-xs text-white/90 leading-relaxed font-medium">
                Autonomous co-worker operates under stage-and-draft boundary. No external email, message, or PR comment is sent without explicit human approval.
              </p>
              <div className="space-y-1.5 text-xs pt-1 font-mono">
                <div className="flex items-center justify-between py-1 border-b border-white/6">
                  <span className="text-white/60">External Send</span>
                  <span className="text-rose-400 text-[11px]">Requires Signature</span>
                </div>
                <div className="flex items-center justify-between py-1 border-b border-white/6">
                  <span className="text-white/60">Calendar Writes</span>
                  <span className="text-amber-400 text-[11px]">Staged Draft</span>
                </div>
                <div className="flex items-center justify-between py-1">
                  <span className="text-white/60">Audit Log</span>
                  <span className="text-emerald-400 text-[11px]">Cryptographic Record</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* ── COPILOT: ACCEPTANCE CRITERIA CHECKLIST ── */}
        {activeTabId === 'criteria' && (
          <div className="space-y-3">
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span>Task Acceptance</span>
                <span className="font-mono text-emerald-400 text-[10px]">{activeTask?.status || 'No task'}</span>
              </div>
              <p className="font-medium text-white/90 text-xs">
                {activeTask?.title || 'No active task selected'}
              </p>
            </div>

            {activeTask?.contract?.criteria && activeTask.contract.criteria.length > 0 ? (
              <div className="space-y-1.5">
                <span className="text-[10.5px] font-semibold text-white/50 uppercase tracking-wider px-1">
                  Criteria Checklist
                </span>
                <div className="space-y-1.5 text-xs">
                  {activeTask.contract.criteria.map((crit: any, idx: number) => (
                    <div key={idx} className="flex items-start gap-2.5 p-2 rounded bg-white/[0.04] border border-white/6">
                      <CheckCircle2 className="size-3.5 text-emerald-400 mt-0.5 shrink-0" />
                      <span className="text-[11.5px] text-white/90">
                        {typeof crit === 'string' ? crit : crit?.description || JSON.stringify(crit)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className="p-3 text-center text-xs text-white/40 border border-dashed border-white/10 rounded-lg">
                No acceptance criteria defined for this task.
              </div>
            )}
          </div>
        )}

        {/* ── COPILOT: EXECUTIVE BRIEF & ACTION ITEMS ── */}
        {activeTabId === 'brief' && (
          <div className="space-y-3">
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span className="flex items-center gap-1.5">
                  <FileCheck className="size-3 text-workbench-accent" />
                  Context Summary
                </span>
              </div>
              <p className="text-xs text-white/85 leading-relaxed">
                {activeSession?.summary || activeTask?.contract?.description || (activeTask?.metadata as any)?.description || 'No summary generated yet for current session.'}
              </p>
            </div>
          </div>
        )}

        {/* ── DIAGNOSTICS & SYSTEM CHECKS (All Workbenches) ── */}
        {activeTabId === 'checks' && (
          <div className="space-y-3">
            <div className="rounded-lg border border-white/8 bg-white/[0.04] p-3 space-y-2.5">
              <div className="flex items-center justify-between text-white/50 text-[10.5px] uppercase tracking-wider font-semibold">
                <span>Workspace Verification</span>
                <span className="flex items-center gap-1 text-emerald-400 font-mono text-[10px]">
                  <CheckCircle2 className="size-3" />
                  Active
                </span>
              </div>
              <div className="space-y-2 text-xs">
                <div className="flex items-center justify-between py-1 border-b border-white/6">
                  <span className="text-white/60">Workspace</span>
                  <span className="font-mono text-white/90 text-[11px] truncate max-w-[150px]">
                    {activeWorkspace?.name || 'Unbound'}
                  </span>
                </div>
                <div className="flex items-center justify-between py-1 border-b border-white/6">
                  <span className="text-white/60">Path</span>
                  <span className="font-mono text-white/50 text-[10.5px] truncate max-w-[150px]" title={activeWorkspace?.path}>
                    {activeWorkspace?.path || '.'}
                  </span>
                </div>
                <div className="flex items-center justify-between py-1 border-b border-white/6">
                  <span className="text-white/60">Branch</span>
                  <span className="font-mono text-sky-400 text-[11px]">
                    {activeWorkspace?.active_branch || 'main'}
                  </span>
                </div>
                <div className="flex items-center justify-between py-1 border-b border-white/6">
                  <span className="text-white/60">Dirty Files</span>
                  <span className="font-mono text-amber-300 text-[11px]">
                    {diffSummary?.files?.length ?? (activeWorkspace?.dirty_manifest?.modified_files?.length || 0)} files
                  </span>
                </div>
                <div className="flex items-center justify-between py-1">
                  <span className="text-white/60">Daemon Connection</span>
                  <span className="font-mono text-emerald-400 text-[11px]">Online</span>
                </div>
              </div>

              <button
                type="button"
                onClick={() => onOpenTab?.('terminal')}
                className="w-full mt-2 py-2 px-3 rounded-lg border border-white/10 bg-white/5 hover:bg-white/10 text-white text-xs font-medium transition cursor-pointer flex items-center justify-center gap-2"
              >
                <Zap className="size-3.5 text-workbench-accent" />
                <span>Open Interactive Terminal</span>
              </button>
            </div>
          </div>
        )}
      </div>
    </aside>
  );
};
