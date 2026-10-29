import React, { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import {
  FolderOpen,
  FileCode,
  RefreshCw,
  FolderTree,
  FolderGit2,
  PanelLeft,
  PanelLeftClose,
  Terminal,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import { CompanionFileViewer } from '@/components/viewer';
import { getFileIcon } from '@/utils/fileIcons';
import { FileTreeSidebar, checkIsDir } from './FileTreeSidebar';
import { DiffViewer } from './DiffViewer';
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
  const [isExplorerOpen, setIsExplorerOpen] = useState<boolean>(true);

  // Files & Diff state
  const [fileEntries, setFileEntries] = useState<WorkspaceFileEntry[]>([]);
  const [diffSummary, setDiffSummary] = useState<WorkspaceDiffSummary | null>(null);

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

  useEffect(() => {
    if (initialViewMode) {
      setFileViewMode(initialViewMode);
    }
  }, [initialViewMode]);

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
            <FileTreeSidebar
              fileEntries={fileEntries}
              changedFilesMap={changedFilesMap}
              folderChangeCounts={folderChangeCounts}
              selectedPath={selectedPath}
              onSelectPath={setSelectedPath}
              onRefresh={loadData}
              onClose={() => setIsExplorerOpen(false)}
              loading={loading}
            />
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
                <DiffViewer
                  selectedPath={selectedPath}
                  fileDiff={fileDiff}
                  fileContent={fileContent}
                  currentDiffEntry={currentDiffEntry}
                  diffStats={diffStats}
                  isExplorerOpen={isExplorerOpen}
                  onToggleExplorer={() => setIsExplorerOpen((prev) => !prev)}
                  onViewModeChange={setFileViewMode}
                  changedFileList={changedFileList}
                  currentDiffIndex={currentDiffIndex}
                  onPrevDiff={handlePrevDiff}
                  onNextDiff={handleNextDiff}
                  onPrimaryAction={handlePrimaryAction}
                  onDiscardCurrent={handleDiscardCurrent}
                  onCopyPath={handleCopyPath}
                  copied={copied}
                  actionInProgress={actionInProgress}
                />
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
