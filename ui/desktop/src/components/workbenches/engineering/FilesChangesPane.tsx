import React, { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import {
  Folder,
  FolderOpen,
  File,
  FileCode,
  FileText,
  RefreshCw,
  Search,
  ChevronRight,
  ChevronDown,
  ChevronUp,
  Copy,
  Check,
  Plus,
  Minus,
  Trash2,
  FolderTree,
  FolderGit2,
  Filter,
  PanelLeft,
  PanelLeftClose,
  Terminal,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import { CompanionFileViewer } from '@/components/viewer';
import type {
  ExecutionWorkspace,
  WorkspaceFileEntry,
  WorkspaceFileContent,
  WorkspaceDiffSummary,
  WorkspaceDiffEntry,
  WorkspaceFileDiff,
} from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export interface FilesChangesPaneProps {
  sessionId?: string;
  initialWorkspaceId?: string;
  initialFilePath?: string;
  initialViewMode?: 'content' | 'diff';
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (id: OrcaTabId, customUrl?: string, customTitle?: string) => void;
}

function getFileIcon(name: string) {
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
    case 'html':
    case 'css':
      return <FileCode className="h-3.5 w-3.5 text-white/60 shrink-0" />;
    case 'json':
    case 'yaml':
    case 'yml':
    case 'toml':
    case 'md':
    case 'txt':
      return <FileText className="h-3.5 w-3.5 text-white/50 shrink-0" />;
    default:
      return <File className="h-3.5 w-3.5 text-[var(--color-fg-muted)] shrink-0" />;
  }
}

interface TreeNode {
  entry?: WorkspaceFileEntry;
  name: string;
  path: string;
  isDir: boolean;
  children: Map<string, TreeNode>;
}

function checkIsDir(entry?: WorkspaceFileEntry): boolean {
  if (!entry) return false;
  return Boolean(
    (entry as any).isDir ??
    (entry as any).is_dir
  );
}

function buildFileTree(entries: WorkspaceFileEntry[]): TreeNode {
  const root: TreeNode = {
    name: 'root',
    path: '',
    isDir: true,
    children: new Map(),
  };

  for (const entry of entries) {
    const parts = entry.path.split('/').filter(Boolean);
    let current = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      const nodePath = parts.slice(0, i + 1).join('/');

      if (!current.children.has(part)) {
        current.children.set(part, {
          entry: isLast ? entry : undefined,
          name: part,
          path: nodePath,
          isDir: isLast ? checkIsDir(entry) : true,
          children: new Map(),
        });
      } else if (isLast && entry) {
        const existing = current.children.get(part)!;
        existing.entry = entry;
        existing.isDir = checkIsDir(entry);
      }
      current = current.children.get(part)!;
    }
  }

  return root;
}

export const FilesChangesPane: React.FC<FilesChangesPaneProps> = ({
  sessionId: _sessionId,
  initialWorkspaceId,
  initialFilePath,
  initialViewMode,
  activeWorkspaceId: propActiveWorkspaceId,
  onSelectWorkspace: _propOnSelectWorkspace,
  onShowToast,
  onOpenTab,
}) => {
  const { handleOpenFolderDirect, activeWorkspaceId: ctxActiveWorkspaceId } = useAppContext();
  const activeWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;

  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>(activeWorkspaceId || initialWorkspaceId || '');
  const appliedInitialWorkspace = useRef<string | undefined>(undefined);
  const treeGeneration = useRef(0);
  const [loading, setLoading] = useState<boolean>(true);
  const [filterQuery, setFilterQuery] = useState<string>('');
  const [onlyChanged, setOnlyChanged] = useState<boolean>(false);
  const [isExplorerOpen, setIsExplorerOpen] = useState<boolean>(!initialFilePath);

  // Files & Diff state
  const [fileEntries, setFileEntries] = useState<WorkspaceFileEntry[]>([]);
  const [diffSummary, setDiffSummary] = useState<WorkspaceDiffSummary | null>(null);
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());

  // Active selected file (never a folder)
  const [selectedPath, setSelectedPath] = useState<string | null>(initialFilePath || null);
  const [fileContent, setFileContent] = useState<WorkspaceFileContent | null>(null);
  const [fileDiff, setFileDiff] = useState<WorkspaceFileDiff | null>(null);
  const [loadingContent, setLoadingContent] = useState<boolean>(false);

  useEffect(() => {
    if (initialFilePath && initialFilePath !== selectedPath) {
      setSelectedPath(initialFilePath);
    }
  }, [initialFilePath]);

  // Actions
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [copied, setCopied] = useState<boolean>(false);
  const [fileViewMode, setFileViewMode] = useState<'content' | 'diff'>(initialViewMode || 'content');

  const loadWorkspaces = useCallback(async () => {
    try {
      setLoading(true);
      const list = await daemonClient.listWorkspaces();
      const activeList = Array.isArray(list) ? list.filter((w) => w.status !== 'archived') : [];
      setWorkspaces(activeList);
      if (!selectedWorkspaceId && activeList.length > 0) {
        setSelectedWorkspaceId(activeList[0].id);
      }
    } catch {
      setWorkspaces([]);
      if (!selectedWorkspaceId) setSelectedWorkspaceId('');
    } finally {
      setLoading(false);
    }
  }, [selectedWorkspaceId]);

  useEffect(() => {
    loadWorkspaces();
  }, [loadWorkspaces]);

  // Synchronize when activeWorkspaceId changes externally (e.g. from Left Sidebar click)
  useEffect(() => {
    if (activeWorkspaceId && activeWorkspaceId !== selectedWorkspaceId) {
      setSelectedWorkspaceId(activeWorkspaceId);
      setSelectedPath(null);
      setFileContent(null);
      setFileDiff(null);
    }
  }, [activeWorkspaceId, selectedWorkspaceId]);

  useEffect(() => {
    if (initialWorkspaceId && appliedInitialWorkspace.current !== initialWorkspaceId && workspaces.some((workspace) => workspace.id === initialWorkspaceId)) {
      appliedInitialWorkspace.current = initialWorkspaceId;
      setSelectedWorkspaceId(initialWorkspaceId);
      setSelectedPath(null);
      setFileContent(null);
      setFileDiff(null);
    }
  }, [initialWorkspaceId, workspaces]);

  const loadData = useCallback(async () => {
    const generation = ++treeGeneration.current;
    if (!selectedWorkspaceId) {
      setFileEntries([]);
      setDiffSummary(null);
      setSelectedPath(null);
      return;
    }
    try {
      const [tree, diff] = await Promise.all([
        daemonClient.getWorkspaceFileTree(selectedWorkspaceId),
        daemonClient.getWorkspaceDiff(selectedWorkspaceId),
      ]);
      if (generation !== treeGeneration.current) return;
      const files: WorkspaceFileEntry[] = (tree && Array.isArray(tree.entries)) ? tree.entries : [];
      setFileEntries(files);
      setDiffSummary(diff || null);

      // Expand top-level dirs
      const initialExpanded = new Set<string>();
      files.forEach((f: WorkspaceFileEntry) => {
        if (checkIsDir(f)) {
          const depth = f.path.split('/').length;
          if (depth <= 2) initialExpanded.add(f.path);
        }
      });
      setExpandedDirs(initialExpanded);

      setSelectedPath(previous => previous || diff?.files?.[0]?.path || files.find(file => !checkIsDir(file))?.path || null);
    } catch {
      if (generation !== treeGeneration.current) return;
      setFileEntries([]);
      setDiffSummary(null);
      setSelectedPath(null);
    }
  }, [selectedWorkspaceId]);

  useEffect(() => {
    void loadData();
    return () => { treeGeneration.current++; };
  }, [loadData]);

  // Changed files map
  const changedFilesMap = useMemo(() => {
    const map = new Map<string, WorkspaceDiffEntry>();
    if (diffSummary?.files) {
      diffSummary.files.forEach((f) => map.set(f.path, f));
    }
    return map;
  }, [diffSummary]);

  // Aggregated change counts for every folder in hierarchy
  const folderChangeCounts = useMemo(() => {
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

  // Load content and diff for selected file
  useEffect(() => {
    if (!selectedWorkspaceId || !selectedPath) {
      setFileContent(null);
      setFileDiff(null);
      setLoadingContent(false);
      return;
    }

    let isMounted = true;
    setLoadingContent(true);

    const diffEntry = diffSummary?.files.find((f) => f.path === selectedPath);
    if (initialViewMode) {
      setFileViewMode(initialViewMode);
    } else if (diffEntry) {
      setFileViewMode('diff');
    } else {
      setFileViewMode('content');
    }

    const promises: Promise<any>[] = [];

    // Fetch full file content
    promises.push(
      daemonClient
        .readWorkspaceFile(
          selectedWorkspaceId,
          selectedPath,
          /\.(pdf|png|jpe?g|gif|webp|avif)$/i.test(selectedPath) ? 10 * 1024 * 1024 : undefined
        )
        .then((content) => {
          if (isMounted) setFileContent(content);
        })
        .catch(() => {
          if (isMounted) setFileContent(null);
        })
    );

    // If diff entry exists, fetch file diff
    if (diffEntry) {
      promises.push(
        daemonClient
          .getWorkspaceFileDiff(selectedWorkspaceId, selectedPath, diffEntry.is_staged)
          .then((diff) => {
            if (isMounted) setFileDiff(diff);
          })
          .catch(() => {
            if (isMounted) setFileDiff(null);
          })
      );
    } else {
      setFileDiff(null);
    }

    Promise.allSettled(promises).finally(() => {
      if (isMounted) setLoadingContent(false);
    });

    return () => {
      isMounted = false;
    };
  }, [selectedWorkspaceId, selectedPath, diffSummary]);

  const filteredEntries = useMemo(() => {
    return fileEntries.filter((entry) => {
      if (onlyChanged) {
        if (checkIsDir(entry)) {
          const count = folderChangeCounts.get(entry.path) || 0;
          if (count === 0) return false;
        } else if (!changedFilesMap.has(entry.path)) {
          return false;
        }
      }
      if (!filterQuery) return true;
      return entry.path.toLowerCase().includes(filterQuery.toLowerCase());
    });
  }, [fileEntries, onlyChanged, changedFilesMap, folderChangeCounts, filterQuery]);

  const treeRoot = useMemo(() => buildFileTree(filteredEntries), [filteredEntries]);

  const toggleDir = (dirPath: string) => {
    setExpandedDirs((prev) => {
      const next = new Set(prev);
      if (next.has(dirPath)) next.delete(dirPath);
      else next.add(dirPath);
      return next;
    });
  };

  const changedFileList = useMemo(() => diffSummary?.files || [], [diffSummary]);
  const currentDiffIndex = changedFileList.findIndex((f) => f.path === selectedPath);

  const handlePrevDiff = () => {
    if (changedFileList.length === 0) return;
    const prev = currentDiffIndex <= 0 ? changedFileList.length - 1 : currentDiffIndex - 1;
    setSelectedPath(changedFileList[prev].path);
  };

  const handleNextDiff = () => {
    if (changedFileList.length === 0) return;
    const next = currentDiffIndex >= changedFileList.length - 1 ? 0 : currentDiffIndex + 1;
    setSelectedPath(changedFileList[next].path);
  };

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

  const currentDiffEntry = useMemo(() => {
    return selectedPath ? changedFilesMap.get(selectedPath) : undefined;
  }, [selectedPath, changedFilesMap]);

  const handlePrimaryAction = async () => {
    if (!selectedWorkspaceId || !selectedPath) return;
    setActionInProgress('primary');
    try {
      if (currentDiffEntry?.is_staged) {
        await daemonClient.unstageWorkspaceFile(selectedWorkspaceId, selectedPath);
        onShowToast?.(`Unstaged ${selectedPath}`);
      } else {
        await daemonClient.stageWorkspaceFile(selectedWorkspaceId, selectedPath);
        onShowToast?.(`Staged ${selectedPath}`);
      }
      await loadData();
    } catch (err: any) {
      onShowToast?.(`Action failed: ${err?.message || err}`);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleDiscardCurrent = async () => {
    if (!selectedWorkspaceId || !selectedPath) return;
    if (!confirm(`Discard all changes in ${selectedPath}?`)) return;
    setActionInProgress('discard');
    try {
      await daemonClient.discardWorkspaceFile(selectedWorkspaceId, selectedPath);
      onShowToast?.(`Discarded changes in ${selectedPath}`);
      await loadData();
    } catch (err: any) {
      onShowToast?.(`Failed to discard: ${err?.message || err}`);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleCopyPath = () => {
    if (!selectedPath) return;
    navigator.clipboard.writeText(selectedPath);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
    onShowToast?.('Copied file path to clipboard');
  };

  const renderTree = (node: TreeNode, depth = 0) => {
    const isExpanded = expandedDirs.has(node.path);
    const hasDiff = changedFilesMap.get(node.path);
    const isSelected = selectedPath === node.path;
    const folderChangeCount = node.isDir ? folderChangeCounts.get(node.path) || 0 : 0;

    const sortedChildren = Array.from(node.children.values()).sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    });

    // Root container directly renders its children
    if (node.path === '') {
      return (
        <div className="space-y-0.5">
          {sortedChildren.map((child) => renderTree(child, 0))}
        </div>
      );
    }

    if (node.isDir) {
      if (onlyChanged && folderChangeCount === 0) {
        return null;
      }

      return (
        <div key={node.path}>
          <div
            role="button"
            tabIndex={0}
            onClick={() => toggleDir(node.path)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                toggleDir(node.path);
              }
            }}
            style={{ paddingLeft: `${depth * 12 + 6}px` }}
            className="flex w-full items-center justify-between py-1 px-1.5 text-left text-xs transition-colors rounded cursor-pointer select-none text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] group"
          >
            <div className="flex items-center gap-1.5 min-w-0 pr-2">
              <span className="p-0.5 text-[var(--color-fg-subtle)]">
                {isExpanded ? (
                  <ChevronDown className="h-3 w-3" />
                ) : (
                  <ChevronRight className="h-3 w-3" />
                )}
              </span>
              {isExpanded ? (
                <FolderOpen className="h-3.5 w-3.5 shrink-0 text-white/60" />
              ) : (
                <Folder className="h-3.5 w-3.5 shrink-0 text-white/40" />
              )}
              <span className="truncate">{node.name}</span>
            </div>
            {folderChangeCount > 0 && (
              <span className="text-[10px] font-mono font-medium text-white/60 mr-1">
                {folderChangeCount}
              </span>
            )}
          </div>
          {isExpanded && (
            <div>
              {sortedChildren.map((child) => renderTree(child, depth + 1))}
            </div>
          )}
        </div>
      );
    }

    // Git Status coloring matching standard IDEs:
    // Modified: amber/yellow text + M
    // Added / Untracked: emerald/green text + U or A
    // Deleted: rose/red text + D
    let nameColorClass = 'text-[var(--color-editor-fg)]';
    let statusLetter = '';
    let statusColor = '';

    if (hasDiff) {
      if (hasDiff.status === 'added' || hasDiff.status === '??') {
        nameColorClass = 'text-white font-medium';
        statusLetter = hasDiff.status === '??' ? 'U' : 'A';
        statusColor = 'text-white/70';
      } else if (hasDiff.status === 'deleted') {
        nameColorClass = 'text-white/40 line-through';
        statusLetter = 'D';
        statusColor = 'text-white/40';
      } else {
        nameColorClass = 'text-white font-medium';
        statusLetter = 'M';
        statusColor = 'text-white/70';
      }
    }

    return (
      <div
        key={node.path}
        role="button"
        tabIndex={0}
        onClick={() => setSelectedPath(node.path)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            setSelectedPath(node.path);
          }
        }}
        style={{ paddingLeft: `${depth * 12 + 18}px` }}
        className={`flex w-full items-center justify-between py-1 px-1.5 text-left text-xs transition-colors rounded cursor-pointer select-none group ${
          isSelected
            ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
            : 'hover:bg-[var(--color-surface-1)]'
        }`}
      >
        <div className="flex items-center gap-1.5 min-w-0 pr-2">
          {getFileIcon(node.name)}
          <span className={`truncate ${nameColorClass}`}>{node.name}</span>
        </div>
        {statusLetter && (
          <span className={`mr-1 shrink-0 font-mono text-[10px] font-bold ${statusColor}`}>
            {statusLetter}
          </span>
        )}
      </div>
    );
  };

  return (
    <div className="wb-split flex h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {workspaces.length === 0 && !loading ? (
        <div className="flex-1 flex items-center justify-center p-6 bg-[var(--color-canvas)]">
          <div className="max-w-md w-full rounded-2xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-6 text-center space-y-4 shadow-xl">
            <div className="w-12 h-12 rounded-xl bg-white/[0.04] border border-white/10 text-white/70 flex items-center justify-center mx-auto">
              <FolderGit2 className="w-6 h-6" />
            </div>
            <div className="space-y-1">
              <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">No Workspace Opened</h3>
              <p className="text-xs text-[var(--color-fg-muted)] leading-relaxed">
                Open a project folder or repository from your machine to browse files and review changes.
              </p>
            </div>
            <button
              onClick={handleOpenFolderDirect}
              className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-white text-black font-semibold text-xs hover:bg-white/90 shadow-sm transition"
            >
              <FolderOpen className="w-4 h-4 text-black" />
              <span>Open Project Folder</span>
            </button>
          </div>
        </div>
      ) : (
        <>
          {/* Optional In-Canvas Mini File Navigator (Collapsible, default collapsed when file is open) */}
          {isExplorerOpen && (
            <div className="wb-master w-60 shrink-0 border-r border-[var(--color-border-subtle)] flex flex-col bg-[var(--color-surface-1)]/30 z-10 transition-all">
              {/* Explorer Header */}
              <div className="flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-2.5 bg-[var(--color-surface-1)]/50">
                <div className="flex items-center gap-1.5 truncate">
                  <FolderTree className="h-3.5 w-3.5 text-white/60 shrink-0" />
                  <span className="text-xs font-semibold text-[var(--color-editor-fg)] truncate">
                    Files Navigator
                  </span>
                  {changedFilesMap.size > 0 && (
                    <span className="text-[10px] font-mono text-white/60 font-medium">
                      ({changedFilesMap.size})
                    </span>
                  )}
                </div>

                <div className="flex items-center gap-0.5">
                  <button
                    type="button"
                    onClick={() => setOnlyChanged(!onlyChanged)}
                    className={`px-1.5 py-0.5 rounded text-[10px] font-medium transition flex items-center gap-1 ${
                      onlyChanged
                        ? 'bg-white/10 text-white border border-white/15'
                        : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
                    }`}
                    title={onlyChanged ? 'Show all files' : 'Show only changed files'}
                  >
                    <Filter className="h-3 w-3" />
                    <span>{onlyChanged ? 'Changed' : 'All'}</span>
                  </button>
                  <button
                    type="button"
                    onClick={loadData}
                    className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
                    title="Refresh files"
                  >
                    <RefreshCw className="h-3 w-3" />
                  </button>
                  <button
                    type="button"
                    onClick={() => setIsExplorerOpen(false)}
                    className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
                    title="Close files navigator"
                  >
                    <PanelLeftClose className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>

              {/* Filter Box */}
              <div className="p-1.5 border-b border-[var(--color-border-subtle)]">
                <div className="relative">
                  <Search className="absolute left-2 top-1.5 h-3 w-3 text-[var(--color-fg-subtle)]" />
                  <input
                    type="text"
                    placeholder="Filter files..."
                    value={filterQuery}
                    onChange={(e) => setFilterQuery(e.target.value)}
                    className="w-full rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] pl-6 pr-2 py-0.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-white/20"
                  />
                </div>
              </div>

              {/* Tree */}
              <div className="flex-1 overflow-y-auto p-1 font-mono text-xs">
                {renderTree(treeRoot)}
                {fileEntries.length === 0 && !loading && (
                  <div className="p-4 text-center text-xs text-[var(--color-fg-subtle)]">
                    No files found.
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Main Column: Full-width Code & Diff Canvas */}
          <div className="flex-1 flex flex-col min-w-0 bg-[var(--color-canvas)] h-full overflow-hidden">
            {selectedPath ? (
              loadingContent ? (
                <div className="flex h-full items-center justify-center text-xs text-[var(--color-fg-subtle)]">
                  <RefreshCw className="mr-2 h-4 w-4 animate-spin text-white/60" />
                  <span>Loading file details...</span>
                </div>
              ) : fileDiff && fileViewMode === 'diff' ? (
                <>
                  {/* Diff Header */}
                  <div className="flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-3 text-xs bg-[var(--color-surface-1)]/40 font-mono">
                    <div className="flex items-center gap-2 truncate text-[11px] text-[var(--color-fg-muted)]">
                      <button
                        type="button"
                        onClick={() => setIsExplorerOpen((prev) => !prev)}
                        className={`p-1 rounded transition mr-0.5 ${
                          isExplorerOpen
                            ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
                            : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
                        }`}
                        title={isExplorerOpen ? 'Hide Files Navigator' : 'Show Files Navigator'}
                      >
                        {isExplorerOpen ? <PanelLeftClose className="h-3.5 w-3.5" /> : <PanelLeft className="h-3.5 w-3.5" />}
                      </button>
                      <span className="text-[var(--color-border-subtle)]">|</span>
                      {getFileIcon(selectedPath)}
                      <span className="text-[var(--color-editor-fg)] font-semibold truncate">{selectedPath}</span>
                      {currentDiffEntry && (
                        <span className="text-[10px] font-mono text-white/70">
                          {currentDiffEntry.is_staged ? '• Staged' : '• Modified'}
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-1.5">
                      {/* View Mode Toggle */}
                      {fileContent && (
                        <div className="flex items-center rounded bg-[var(--color-surface-2)] p-0.5 border border-[var(--color-border-subtle)] text-[10px] mr-1">
                          <button
                            type="button"
                            onClick={() => setFileViewMode('diff')}
                            className="px-2 py-0.5 rounded transition bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-xs font-semibold"
                          >
                            Git Diff
                          </button>
                          <button
                            type="button"
                            onClick={() => setFileViewMode('content')}
                            className="px-2 py-0.5 rounded transition text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
                          >
                            Code Details
                          </button>
                        </div>
                      )}

                      {diffStats && (
                        <span className="flex items-center gap-1 font-mono text-[10px] px-1.5 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] mr-0.5">
                          <span className="text-white/90 font-semibold font-mono">+{diffStats.added}</span>
                          <span className="text-white/50 font-semibold font-mono">-{diffStats.deleted}</span>
                        </span>
                      )}

                      {changedFileList.length > 1 && (
                        <div className="flex items-center gap-0.5 px-1 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] text-[10px] text-[var(--color-fg-muted)] mr-0.5">
                          <button
                            type="button"
                            onClick={handlePrevDiff}
                            className="hover:text-[var(--color-editor-fg)] transition p-0.5"
                            title="Previous changed file"
                          >
                            <ChevronUp className="h-3 w-3" />
                          </button>
                          <span className="font-mono text-[9px] px-1 select-none">
                            {currentDiffIndex >= 0 ? currentDiffIndex + 1 : 1}/{changedFileList.length}
                          </span>
                          <button
                            type="button"
                            onClick={handleNextDiff}
                            className="hover:text-[var(--color-editor-fg)] transition p-0.5"
                            title="Next changed file"
                          >
                            <ChevronDown className="h-3 w-3" />
                          </button>
                        </div>
                      )}

                      {currentDiffEntry && (
                        <>
                          <button
                            type="button"
                            disabled={!!actionInProgress}
                            onClick={handlePrimaryAction}
                            className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] transition disabled:opacity-50 ${
                              currentDiffEntry.is_staged
                                ? 'border border-white/15 bg-white/5 text-white/80 hover:bg-white/10 hover:text-white font-medium'
                                : 'bg-white text-black hover:bg-white/90 font-semibold shadow-sm'
                            }`}
                          >
                            {currentDiffEntry.is_staged ? (
                              <>
                                <Minus className="h-2.5 w-2.5" />
                                <span>Unstage</span>
                              </>
                            ) : (
                              <>
                                <Plus className="h-2.5 w-2.5" />
                                <span>Stage</span>
                              </>
                            )}
                          </button>
                          <button
                            type="button"
                            disabled={!!actionInProgress}
                            onClick={handleDiscardCurrent}
                            className="flex items-center gap-1 px-2 py-0.5 rounded text-[10px] text-white/60 hover:text-white hover:bg-white/10 transition"
                            title="Discard changes"
                          >
                            <Trash2 className="h-2.5 w-2.5" />
                            <span>Discard</span>
                          </button>
                        </>
                      )}

                      <button
                        type="button"
                        onClick={handleCopyPath}
                        className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
                        title="Copy file path"
                      >
                        {copied ? <Check className="h-3 w-3 text-white" /> : <Copy className="h-3 w-3" />}
                      </button>
                    </div>
                  </div>

                  {/* Diff Viewer Body */}
                  <div className="flex-1 overflow-auto p-3 font-mono text-xs">
                    <div className="space-y-0.5">
                      {fileDiff.diff ? (
                        fileDiff.diff.split('\n').map((line: string, idx: number) => {
                          const isAdd = line.startsWith('+') && !line.startsWith('+++');
                          const isDel = line.startsWith('-') && !line.startsWith('---');
                          const isHunk = line.startsWith('@@');

                          let lineClass = 'text-[var(--color-fg-muted)]';
                          let bgClass = '';
                          if (isAdd) {
                            lineClass = 'text-emerald-400';
                            bgClass = 'bg-emerald-500/10';
                          } else if (isDel) {
                            lineClass = 'text-rose-400';
                            bgClass = 'bg-rose-500/10';
                          } else if (isHunk) {
                            lineClass = 'text-blue-400 font-semibold';
                            bgClass = 'bg-blue-500/5';
                          }

                          return (
                            <div
                              key={idx}
                              className={`flex px-2 py-0.5 whitespace-pre rounded-xs ${lineClass} ${bgClass}`}
                            >
                              <span className="w-8 shrink-0 select-none text-[10px] text-[var(--color-fg-subtle)] text-right pr-2">
                                {idx + 1}
                              </span>
                              <span>{line}</span>
                            </div>
                          );
                        })
                      ) : (
                        <div className="text-[var(--color-fg-subtle)]">No diff changes detected in file.</div>
                      )}
                    </div>
                  </div>
                </>
              ) : fileContent ? (
                <CompanionFileViewer
                  path={selectedPath}
                  content={fileContent.content}
                  isBinary={fileContent.is_binary}
                  truncated={fileContent.truncated}
                  sizeBytes={fileContent.size_bytes}
                  lineCount={fileContent.line_count}
                  isReadOnly={true}
                  onOpenInBrowser={(url) => {
                    const fileName = selectedPath.split('/').pop() || 'Web Preview';
                    onOpenTab?.('browser', url, `${fileName} - Browser`);
                    onShowToast?.(`Opened ${fileName} in Browser Workbench`);
                  }}
                  extraHeaderActions={
                    <div className="flex items-center gap-1.5">
                      <button
                        type="button"
                        onClick={() => setIsExplorerOpen((prev) => !prev)}
                        className={`p-1 rounded transition mr-0.5 ${
                          isExplorerOpen
                            ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
                            : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
                        }`}
                        title={isExplorerOpen ? 'Hide Files Navigator' : 'Show Files Navigator'}
                      >
                        {isExplorerOpen ? <PanelLeftClose className="h-3.5 w-3.5" /> : <PanelLeft className="h-3.5 w-3.5" />}
                      </button>
                      <span className="text-[var(--color-border-subtle)]">|</span>
                      {fileDiff && (
                        <div className="flex items-center rounded bg-[var(--color-surface-2)] p-0.5 border border-[var(--color-border-subtle)] text-[10px]">
                          <button
                            type="button"
                            onClick={() => setFileViewMode('diff')}
                            className={`px-2 py-0.5 rounded transition ${
                              fileViewMode === 'diff'
                                ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-xs font-semibold'
                                : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                            }`}
                          >
                            Git Diff
                          </button>
                          <button
                            type="button"
                            onClick={() => setFileViewMode('content')}
                            className={`px-2 py-0.5 rounded transition ${
                              fileViewMode === 'content'
                                ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-xs font-semibold'
                                : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                            }`}
                          >
                            Code Details
                          </button>
                        </div>
                      )}
                      {currentDiffEntry && (
                        <button
                          type="button"
                          disabled={!!actionInProgress}
                          onClick={handlePrimaryAction}
                          className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium transition disabled:opacity-50 ${
                            currentDiffEntry.is_staged
                              ? 'bg-zinc-700 hover:bg-zinc-600 text-zinc-200'
                              : 'bg-white/10 hover:bg-white/15 text-white border border-white/10'
                          }`}
                        >
                          {currentDiffEntry.is_staged ? 'Unstage' : 'Stage'}
                        </button>
                      )}
                    </div>
                  }
                />
              ) : (
                <div className="flex flex-col items-center justify-center h-full text-xs text-[var(--color-fg-subtle)] space-y-2 p-6 text-center">
                  <FileCode className="w-8 h-8 opacity-40 text-white/40" />
                  <p className="font-semibold text-[var(--color-editor-fg)]">{selectedPath}</p>
                  <p className="text-[11px] text-[var(--color-fg-muted)]">
                    File content is unavailable or unreadable.
                  </p>
                  <button
                    type="button"
                    onClick={() => {
                      if (selectedPath) {
                        setLoadingContent(true);
                        daemonClient
                          .readWorkspaceFile(selectedWorkspaceId, selectedPath)
                          .then(setFileContent)
                          .catch(() => {})
                          .finally(() => setLoadingContent(false));
                      }
                    }}
                    className="px-3 py-1 rounded bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-[11px] text-[var(--color-editor-fg)] border border-[var(--color-border-subtle)] transition"
                  >
                    Retry
                  </button>
                </div>
              )
            ) : (
              /* Center Canvas Clean Welcome & Hub */
              <div className="flex-1 flex flex-col items-center justify-center p-8 text-center text-xs text-[var(--color-fg-subtle)] space-y-4">
                <div className="w-12 h-12 rounded-xl bg-white/[0.04] border border-white/10 text-white/70 flex items-center justify-center mx-auto">
                  <FileCode className="w-6 h-6" />
                </div>
                <div className="space-y-1">
                  <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">
                    Workspace Editor Canvas
                  </h3>
                  <p className="text-xs text-[var(--color-fg-muted)] max-w-sm">
                    Select any file to inspect code with full syntax highlighting, line numbers, and git diff.
                  </p>
                </div>

                <div className="flex items-center gap-2 pt-2">
                  <button
                    type="button"
                    onClick={() => setIsExplorerOpen(true)}
                    className="px-3 py-1.5 rounded-lg bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-[11px] font-medium text-[var(--color-editor-fg)] border border-[var(--color-border-subtle)] transition flex items-center gap-1.5"
                  >
                    <FolderTree className="w-3.5 h-3.5 text-white/60" />
                    <span>Open In-Canvas Navigator</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => onOpenTab?.('terminal')}
                    className="px-3 py-1.5 rounded-lg bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-[11px] font-medium text-[var(--color-editor-fg)] border border-[var(--color-border-subtle)] transition flex items-center gap-1.5"
                  >
                    <Terminal className="w-3.5 h-3.5 text-white/60" />
                    <span>Open Terminal</span>
                  </button>
                </div>

                {changedFileList.length > 0 && (
                  <div className="w-full max-w-md pt-4 text-left border-t border-[var(--color-border-subtle)] mt-4">
                    <div className="text-[11px] font-medium text-[var(--color-fg-muted)] mb-2 flex items-center justify-between">
                      <span>Modified files ({changedFileList.length}):</span>
                    </div>
                    <div className="space-y-1 max-h-48 overflow-y-auto">
                      {changedFileList.map((f) => (
                        <div
                          key={f.path}
                          onClick={() => setSelectedPath(f.path)}
                          className="flex items-center justify-between px-2.5 py-1.5 rounded bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)] cursor-pointer text-[11px] font-mono transition border border-[var(--color-border-subtle)]/50 group"
                        >
                          <div className="flex items-center gap-1.5 truncate">
                            {getFileIcon(f.path)}
                            <span className="text-[var(--color-editor-fg)] group-hover:text-white transition truncate">{f.path}</span>
                          </div>
                          <div className="flex items-center gap-1 shrink-0 font-mono text-[10px]">
                            {f.additions > 0 && <span className="text-white/80 font-medium">+{f.additions}</span>}
                            {f.deletions > 0 && <span className="text-white/40 font-medium">-{f.deletions}</span>}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
};
