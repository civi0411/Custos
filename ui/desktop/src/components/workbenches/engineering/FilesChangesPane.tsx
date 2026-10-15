import React, { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import {
  Folder,
  FolderOpen,
  File,
  FileCode,
  FileText,
  FileDiff,
  RefreshCw,
  Search,
  ChevronRight,
  ChevronDown,
  Copy,
  Check,
  Plus,
  Minus,
  Trash2,
  FolderTree,
  FolderGit2,
  Filter,
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
      return <FileCode className="h-3.5 w-3.5 text-blue-400 shrink-0" />;
    case 'json':
    case 'yaml':
    case 'yml':
    case 'toml':
      return <FileText className="h-3.5 w-3.5 text-amber-400 shrink-0" />;
    case 'md':
    case 'txt':
      return <FileText className="h-3.5 w-3.5 text-emerald-400 shrink-0" />;
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
  activeWorkspaceId: propActiveWorkspaceId,
  onSelectWorkspace: propOnSelectWorkspace,
  onShowToast,
  onOpenTab: _onOpenTab,
}) => {
  const { handleOpenFolderDirect, activeWorkspaceId: ctxActiveWorkspaceId, setActiveWorkspaceId: ctxSetActiveWorkspaceId } = useAppContext();
  const activeWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const onSelectWorkspace = propOnSelectWorkspace ?? ctxSetActiveWorkspaceId;

  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>(activeWorkspaceId || initialWorkspaceId || '');
  const appliedInitialWorkspace = useRef<string | undefined>(undefined);
  const [loading, setLoading] = useState<boolean>(true);
  const [filterQuery, setFilterQuery] = useState<string>('');
  const [onlyChanged, setOnlyChanged] = useState<boolean>(false);

  // Files & Diff state
  const [fileEntries, setFileEntries] = useState<WorkspaceFileEntry[]>([]);
  const [diffSummary, setDiffSummary] = useState<WorkspaceDiffSummary | null>(null);
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());

  // Active selected file (never a folder)
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState<WorkspaceFileContent | null>(null);
  const [fileDiff, setFileDiff] = useState<WorkspaceFileDiff | null>(null);
  const [loadingContent, setLoadingContent] = useState<boolean>(false);

  // Actions
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [copied, setCopied] = useState<boolean>(false);

  const loadWorkspaces = useCallback(async () => {
    try {
      setLoading(true);
      const list = await daemonClient.listWorkspaces();
      const activeList = list.filter((w) => w.status !== 'archived');
      setWorkspaces(activeList);
      if (activeList.length > 0 && !selectedWorkspaceId && !activeWorkspaceId) {
        setSelectedWorkspaceId(activeList[0].id);
        onSelectWorkspace?.(activeList[0].id);
      }
    } catch (err: any) {
      onShowToast?.(`Failed to load workspaces: ${err?.message || err}`);
    } finally {
      setLoading(false);
    }
  }, [selectedWorkspaceId, activeWorkspaceId, onSelectWorkspace, onShowToast]);

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
    if (!selectedWorkspaceId) return;
    try {
      const [tree, diff] = await Promise.all([
        daemonClient.getWorkspaceFileTree(selectedWorkspaceId),
        daemonClient.getWorkspaceDiff(selectedWorkspaceId),
      ]);
      const files: WorkspaceFileEntry[] = tree.entries || [];
      setFileEntries(files);
      setDiffSummary(diff);

      // Expand top-level dirs
      const initialExpanded = new Set<string>();
      files.forEach((f: WorkspaceFileEntry) => {
        if (checkIsDir(f)) {
          const depth = f.path.split('/').length;
          if (depth <= 2) initialExpanded.add(f.path);
        }
      });
      setExpandedDirs(initialExpanded);

      // Default selection to first changed file
      if (selectedPath === null) {
        if (diff && diff.files && diff.files.length > 0) {
          setSelectedPath(diff.files[0].path);
        } else if (files.length > 0) {
          const firstFile = files.find((f) => !checkIsDir(f));
          if (firstFile) setSelectedPath(firstFile.path);
        }
      }
    } catch (err: any) {
      setFileEntries([]);
      setDiffSummary(null);
      setFileContent(null);
      setFileDiff(null);
      onShowToast?.(`Failed to load workspace files: ${err?.message || err}`);
    }
  }, [selectedWorkspaceId, selectedPath, onShowToast]);

  useEffect(() => {
    loadData();
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

  // Load content or diff for selected file
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

    if (diffEntry) {
      daemonClient
        .getWorkspaceFileDiff(selectedWorkspaceId, selectedPath, diffEntry.is_staged)
        .then((diff) => {
          if (isMounted) {
            setFileDiff(diff);
            setFileContent(null);
          }
        })
        .catch(() => {
          return daemonClient
            .readWorkspaceFile(selectedWorkspaceId, selectedPath)
            .then((content) => {
              if (isMounted) {
                setFileContent(content);
                setFileDiff(null);
              }
            });
        })
        .finally(() => {
          if (isMounted) setLoadingContent(false);
        });
    } else {
      daemonClient
        .readWorkspaceFile(selectedWorkspaceId, selectedPath)
        .then((content) => {
          if (isMounted) {
            setFileContent(content);
            setFileDiff(null);
          }
        })
        .catch(() => {
          if (isMounted) {
            setFileContent(null);
            setFileDiff(null);
          }
        })
        .finally(() => {
          if (isMounted) setLoadingContent(false);
        });
    }

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
                <FolderOpen className="h-3.5 w-3.5 shrink-0 text-amber-500/90" />
              ) : (
                <Folder className="h-3.5 w-3.5 shrink-0 text-amber-500/70" />
              )}
              <span className="truncate">{node.name}</span>
            </div>
            {folderChangeCount > 0 && (
              <span className="text-[10px] font-mono font-medium text-amber-400/80 mr-1">
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
        nameColorClass = 'text-emerald-400 font-medium';
        statusLetter = hasDiff.status === '??' ? 'U' : 'A';
        statusColor = 'text-emerald-400';
      } else if (hasDiff.status === 'deleted') {
        nameColorClass = 'text-rose-400 line-through';
        statusLetter = 'D';
        statusColor = 'text-rose-400';
      } else {
        nameColorClass = 'text-amber-400 font-medium';
        statusLetter = 'M';
        statusColor = 'text-amber-400';
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
    <div className="flex h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {workspaces.length === 0 && !loading ? (
        <div className="flex-1 flex items-center justify-center p-6 bg-[var(--color-canvas)]">
          <div className="max-w-md w-full rounded-2xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-6 text-center space-y-4 shadow-xl">
            <div className="w-12 h-12 rounded-xl bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 flex items-center justify-center mx-auto">
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
              className="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-medium text-xs transition shadow-lg shadow-emerald-950/20"
            >
              <FolderOpen className="w-4 h-4" />
              <span>Open Project Folder from Disk...</span>
            </button>
          </div>
        </div>
      ) : (
        <>
          {/* Left Column: Explorer Sidebar */}
          <div className="w-64 shrink-0 border-r border-[var(--color-border-subtle)] flex flex-col bg-[var(--color-surface-1)]/20">
            {/* Sidebar Clean Header */}
            <div className="flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-2.5 bg-[var(--color-surface-1)]/40">
              <div className="flex items-center gap-1.5 truncate">
                <FolderTree className="h-3.5 w-3.5 text-[var(--color-fg-subtle)] shrink-0" />
                <select
                  aria-label="Files workspace"
                  value={selectedWorkspaceId}
                  onChange={(event) => {
                    const newId = event.target.value;
                    setSelectedWorkspaceId(newId);
                    onSelectWorkspace?.(newId);
                    setSelectedPath(null);
                    setFileContent(null);
                    setFileDiff(null);
                  }}
                  className="min-w-0 max-w-40 bg-transparent text-xs font-semibold text-[var(--color-editor-fg)] outline-none"
                >
                  {workspaces.map((workspace) => (
                    <option key={workspace.id} value={workspace.id}>{workspace.name || workspace.path}</option>
                  ))}
                </select>
                {changedFilesMap.size > 0 && (
                  <span className="text-[10px] font-mono text-amber-400 font-medium">
                    ({changedFilesMap.size})
                  </span>
                )}
              </div>

              <div className="flex items-center gap-0.5">
                <button
                  type="button"
                  onClick={() => setOnlyChanged(!onlyChanged)}
                  className={`p-1 rounded text-xs transition ${
                    onlyChanged
                      ? 'bg-amber-500/15 text-amber-400'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
                  }`}
                  title={onlyChanged ? 'Show all files' : 'Show only changed files'}
                >
                  <Filter className="h-3 w-3" />
                </button>
                <button
                  type="button"
                  onClick={loadData}
                  className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
                  title="Refresh files"
                >
                  <RefreshCw className="h-3 w-3" />
                </button>
              </div>
            </div>

            {/* Compact Filter Box */}
            <div className="p-1.5 border-b border-[var(--color-border-subtle)]">
              <div className="relative">
                <Search className="absolute left-2 top-1.5 h-3 w-3 text-[var(--color-fg-subtle)]" />
                <input
                  type="text"
                  placeholder="Filter files..."
                  value={filterQuery}
                  onChange={(e) => setFilterQuery(e.target.value)}
                  className="w-full rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] pl-6 pr-2 py-0.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-emerald-500/50"
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

          {/* Right Column: Code & Diff Viewer */}
          <div className="flex-1 flex flex-col min-w-0 bg-[var(--color-canvas)]">
            {selectedPath ? (
              loadingContent ? (
                <div className="flex h-full items-center justify-center text-xs text-[var(--color-fg-subtle)]">
                  Loading...
                </div>
              ) : fileDiff ? (
                <>
                  {/* File Header */}
                  <div className="flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-3 text-xs bg-[var(--color-surface-1)]/30 font-mono">
                    <div className="flex items-center gap-2 truncate text-[11px] text-[var(--color-fg-muted)]">
                      {getFileIcon(selectedPath)}
                      <span className="text-[var(--color-editor-fg)] font-medium truncate">{selectedPath}</span>
                      {currentDiffEntry && (
                        <span className="text-[10px] text-amber-400 font-semibold">
                          {currentDiffEntry.is_staged ? '• Staged' : '• Modified'}
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-1.5">
                      {currentDiffEntry && (
                        <>
                          <button
                            type="button"
                            disabled={!!actionInProgress}
                            onClick={handlePrimaryAction}
                            className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium transition disabled:opacity-50 ${
                              currentDiffEntry.is_staged
                                ? 'bg-zinc-700 hover:bg-zinc-600 text-zinc-200'
                                : 'bg-emerald-600 hover:bg-emerald-500 text-white'
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
                            className="flex items-center gap-1 px-2 py-0.5 rounded text-[10px] text-rose-400 hover:bg-rose-500/10 transition"
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
                        title="Copy path"
                      >
                        {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
                      </button>
                    </div>
                  </div>

                  {/* Diff Viewer */}
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
                  sizeBytes={fileContent.size_bytes}
                  lineCount={fileContent.line_count}
                  isReadOnly={true}
                  onOpenInBrowser={(url) => {
                    const fileName = selectedPath.split('/').pop() || 'Web Preview';
                    _onOpenTab?.('browser', url, `${fileName} - Browser`);
                    onShowToast?.(`Opened ${fileName} in Browser Workbench`);
                  }}
                  extraHeaderActions={
                    currentDiffEntry ? (
                      <div className="flex items-center gap-1.5">
                        <button
                          type="button"
                          disabled={!!actionInProgress}
                          onClick={handlePrimaryAction}
                          className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium transition disabled:opacity-50 ${
                            currentDiffEntry.is_staged
                              ? 'bg-zinc-700 hover:bg-zinc-600 text-zinc-200'
                              : 'bg-emerald-600 hover:bg-emerald-500 text-white'
                          }`}
                        >
                          {currentDiffEntry.is_staged ? 'Unstage' : 'Stage'}
                        </button>
                      </div>
                    ) : undefined
                  }
                />
              ) : (
                <div className="flex h-full items-center justify-center text-xs text-[var(--color-fg-subtle)]">
                  File content is binary or unavailable.
                </div>
              )
            ) : (
              <div className="flex-1 flex flex-col items-center justify-center p-6 text-center text-xs text-[var(--color-fg-subtle)] space-y-2">
                <FileDiff className="w-6 h-6 text-[var(--color-fg-subtle)]/40" />
                <p>Select a file to view code or diff.</p>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
};
