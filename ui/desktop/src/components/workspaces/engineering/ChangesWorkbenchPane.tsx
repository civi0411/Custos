import React, { useState, useEffect, useCallback, useMemo } from 'react';
import {
  FileDiff,
  GitBranch,
  RefreshCw,
  Plus,
  Minus,
  Trash2,
  Copy,
  CheckCircle2,
  AlertCircle,
  FileCode,
  Check,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import {
  ExecutionWorkspace,
  WorkspaceDiffEntry,
  WorkspaceDiffSummary,
  WorkspaceFileDiff,
} from '@/types/domain';

export const ChangesWorkbenchPane: React.FC = () => {
  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>('');
  const [loadingWorkspaces, setLoadingWorkspaces] = useState<boolean>(true);

  // Diff summary
  const [diffSummary, setDiffSummary] = useState<WorkspaceDiffSummary | null>(null);
  const [loadingDiff, setLoadingDiff] = useState<boolean>(false);
  const [selectedFile, setSelectedFile] = useState<WorkspaceDiffEntry | null>(null);

  // Single file diff
  const [fileDiff, setFileDiff] = useState<WorkspaceFileDiff | null>(null);
  const [loadingFileDiff, setLoadingFileDiff] = useState<boolean>(false);

  // UI state
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [successToast, setSuccessToast] = useState<string | null>(null);
  const [copied, setCopied] = useState<boolean>(false);

  const fetchWorkspaces = useCallback(async () => {
    try {
      setLoadingWorkspaces(true);
      const list = await daemonClient.listWorkspaces();
      const activeList = list.filter((w) => w.status !== 'archived');
      setWorkspaces(activeList);
      if (activeList.length > 0 && !selectedWorkspaceId) {
        setSelectedWorkspaceId(activeList[0].id);
      }
    } catch (e: any) {
      setErrorMessage(e?.message || 'Failed to list execution workspaces');
    } finally {
      setLoadingWorkspaces(false);
    }
  }, [selectedWorkspaceId]);

  useEffect(() => {
    fetchWorkspaces();
  }, [fetchWorkspaces]);

  const loadDiff = useCallback(async () => {
    if (!selectedWorkspaceId) return;
    try {
      setLoadingDiff(true);
      setErrorMessage(null);
      const summary = await daemonClient.getWorkspaceDiff(selectedWorkspaceId);
      setDiffSummary(summary);

      // Auto-select first file if current selection is invalid
      if (summary.files && summary.files.length > 0) {
        if (!selectedFile || !summary.files.some((f) => f.path === selectedFile.path && f.is_staged === selectedFile.is_staged)) {
          setSelectedFile(summary.files[0]);
        }
      } else {
        setSelectedFile(null);
        setFileDiff(null);
      }
    } catch (e: any) {
      setErrorMessage(e?.message || 'Failed to fetch Git diff summary');
      setDiffSummary(null);
    } finally {
      setLoadingDiff(false);
    }
  }, [selectedWorkspaceId, selectedFile]);

  useEffect(() => {
    if (selectedWorkspaceId) {
      loadDiff();
    }
  }, [selectedWorkspaceId, loadDiff]);

  // Load single file diff whenever selectedFile changes
  useEffect(() => {
    if (!selectedWorkspaceId || !selectedFile) {
      setFileDiff(null);
      return;
    }

    let isMounted = true;
    const fetchFileDiff = async () => {
      try {
        setLoadingFileDiff(true);
        const fd = await daemonClient.getWorkspaceFileDiff(
          selectedWorkspaceId,
          selectedFile.path,
          selectedFile.is_staged
        );
        if (isMounted) {
          setFileDiff(fd);
        }
      } catch (e: any) {
        if (isMounted) {
          setErrorMessage(e?.message || `Failed to read diff for ${selectedFile.path}`);
        }
      } finally {
        if (isMounted) {
          setLoadingFileDiff(false);
        }
      }
    };

    fetchFileDiff();
    return () => {
      isMounted = false;
    };
  }, [selectedWorkspaceId, selectedFile]);

  const handleStage = async (path: string) => {
    if (!selectedWorkspaceId) return;
    try {
      setActionInProgress(`stage-${path}`);
      await daemonClient.stageWorkspaceFile(selectedWorkspaceId, path);
      setSuccessToast(`Staged ${path}`);
      setTimeout(() => setSuccessToast(null), 3000);
      await loadDiff();
    } catch (e: any) {
      setErrorMessage(e?.message || `Failed to stage ${path}`);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleUnstage = async (path: string) => {
    if (!selectedWorkspaceId) return;
    try {
      setActionInProgress(`unstage-${path}`);
      await daemonClient.unstageWorkspaceFile(selectedWorkspaceId, path);
      setSuccessToast(`Unstaged ${path}`);
      setTimeout(() => setSuccessToast(null), 3000);
      await loadDiff();
    } catch (e: any) {
      setErrorMessage(e?.message || `Failed to unstage ${path}`);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleDiscard = async (path: string) => {
    if (!selectedWorkspaceId) return;
    const confirmed = window.confirm(
      `Are you sure you want to discard changes in '${path}'? This cannot be undone.`
    );
    if (!confirmed) return;

    try {
      setActionInProgress(`discard-${path}`);
      await daemonClient.discardWorkspaceFile(selectedWorkspaceId, path);
      setSuccessToast(`Discarded ${path}`);
      setTimeout(() => setSuccessToast(null), 3000);
      await loadDiff();
    } catch (e: any) {
      setErrorMessage(e?.message || `Failed to discard ${path}`);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleCopyDiff = () => {
    const textToCopy = fileDiff?.diff || diffSummary?.raw_diff || '';
    if (!textToCopy) return;
    navigator.clipboard.writeText(textToCopy);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const stagedFiles = useMemo(
    () => diffSummary?.files.filter((f) => f.is_staged) || [],
    [diffSummary]
  );
  const unstagedFiles = useMemo(
    () => diffSummary?.files.filter((f) => !f.is_staged && f.status !== '??') || [],
    [diffSummary]
  );
  const untrackedFiles = useMemo(
    () => diffSummary?.files.filter((f) => !f.is_staged && f.status === '??') || [],
    [diffSummary]
  );

  const activeWorkspace = workspaces.find((w) => w.id === selectedWorkspaceId);

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Top Bar */}
      <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-4 py-2.5">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <FileDiff className="h-4 w-4 text-[var(--color-fg-muted)]" />
            <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
              Changes
            </span>
          </div>

          <div className="h-4 w-[1px] bg-[var(--color-border-subtle)]" />

          {/* Workspace dropdown */}
          <div className="flex items-center gap-2">
            <label className="text-[11px] text-[var(--color-fg-subtle)]">Workspace:</label>
            <select
              value={selectedWorkspaceId}
              onChange={(e) => setSelectedWorkspaceId(e.target.value)}
              disabled={loadingWorkspaces}
              className="rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:ring-1 focus:ring-zinc-600"
            >
              {workspaces.map((ws) => (
                <option key={ws.id} value={ws.id}>
                  {ws.name} ({ws.kind.type === 'git' ? 'Git' : 'Folder'})
                </option>
              ))}
            </select>
            {activeWorkspace && (
              <span className="hidden xl:inline text-[11px] text-[var(--color-fg-subtle)] font-mono truncate max-w-[200px]">
                {activeWorkspace.path}
              </span>
            )}
          </div>

          {diffSummary && (
            <div className="hidden lg:flex items-center gap-2 text-xs">
              {diffSummary.branch && (
                <span className="inline-flex items-center gap-1 rounded bg-[var(--color-surface-2)] px-2 py-0.5 text-[11px] font-mono text-emerald-400">
                  <GitBranch className="h-3 w-3" />
                  {diffSummary.branch}
                </span>
              )}
              {diffSummary.head_hash && (
                <span className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[10px] text-[var(--color-fg-subtle)]">
                  {diffSummary.head_hash.substring(0, 7)}
                </span>
              )}
              <span className={`inline-flex items-center gap-1 text-[11px] font-medium ${
                diffSummary.is_clean ? 'text-emerald-500' : 'text-amber-400'
              }`}>
                {diffSummary.is_clean ? 'Clean' : `${diffSummary.files.length} changed`}
              </span>
              {(diffSummary.total_additions > 0 || diffSummary.total_deletions > 0) && (
                <div className="flex items-center gap-1 text-[10px] font-mono">
                  <span className="text-emerald-400">+{diffSummary.total_additions}</span>
                  <span className="text-red-400">-{diffSummary.total_deletions}</span>
                </div>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center gap-2">
          {successToast && (
            <span className="flex items-center gap-1 text-[11px] text-emerald-400">
              <CheckCircle2 className="h-3.5 w-3.5" />
              {successToast}
            </span>
          )}
          {errorMessage && (
            <span className="flex items-center gap-1 text-[11px] text-red-400">
              <AlertCircle className="h-3.5 w-3.5" />
              {errorMessage}
            </span>
          )}
          <button
            onClick={loadDiff}
            disabled={loadingDiff || !selectedWorkspaceId}
            title="Refresh Git Diff"
            className="flex items-center gap-1 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-1 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition disabled:opacity-50"
          >
            <RefreshCw className={`h-3 w-3 ${loadingDiff ? 'animate-spin' : ''}`} />
            <span>Refresh</span>
          </button>
        </div>
      </div>

      {/* Main Split Body */}
      <div className="flex flex-1 min-h-0">
        {/* Left Side: Changed Files List */}
        <div className="w-72 border-r border-[var(--color-border-muted)] bg-[var(--color-surface-1)]/50 flex flex-col">
          <div className="p-2.5 border-b border-[var(--color-border-subtle)] flex items-center justify-between text-xs font-semibold text-[var(--color-fg-muted)]">
            <span>Changed Files</span>
            <span className="text-[10px] font-mono text-[var(--color-fg-subtle)]">
              {diffSummary?.files.length || 0}
            </span>
          </div>

          <div className="flex-1 overflow-y-auto p-2 space-y-4 text-xs">
            {loadingDiff ? (
              <div className="flex items-center justify-center p-6 text-[11px] text-[var(--color-fg-muted)]">
                <RefreshCw className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                Reading Git changes...
              </div>
            ) : diffSummary?.is_clean ? (
              <div className="p-6 text-center text-[11px] text-[var(--color-fg-muted)]">
                <CheckCircle2 className="mx-auto mb-2 h-6 w-6 text-emerald-500 opacity-60" />
                Working tree clean.
              </div>
            ) : (
              <>
                {/* 1. Staged Changes */}
                {stagedFiles.length > 0 && (
                  <div>
                    <div className="mb-1.5 flex items-center justify-between text-[10px] font-semibold uppercase tracking-wider text-emerald-400">
                      <span>Staged Changes ({stagedFiles.length})</span>
                    </div>
                    <div className="space-y-1">
                      {stagedFiles.map((file) => {
                        const isSelected = selectedFile?.path === file.path && selectedFile?.is_staged;
                        return (
                          <div
                            key={`staged-${file.path}`}
                            onClick={() => setSelectedFile(file)}
                            className={`group flex items-center justify-between rounded px-2 py-1.5 cursor-pointer text-[11px] transition ${
                              isSelected
                                ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium border-l-2 border-emerald-500'
                                : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-1)] hover:text-[var(--color-editor-fg)]'
                            }`}
                          >
                            <div className="flex items-center gap-1.5 truncate">
                              <span className="font-mono text-[10px] text-emerald-400 font-bold">{file.status}</span>
                              <span className="truncate">{file.path}</span>
                            </div>
                            <div className="flex items-center gap-1">
                              {(file.additions > 0 || file.deletions > 0) && (
                                <span className="font-mono text-[9px] text-[var(--color-fg-subtle)]">
                                  +{file.additions} -{file.deletions}
                                </span>
                              )}
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleUnstage(file.path);
                                }}
                                title="Unstage file"
                                className="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-[var(--color-surface-3)] rounded text-[var(--color-fg-muted)] hover:text-amber-400 transition"
                              >
                                <Minus className="h-3 w-3" />
                              </button>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}

                {/* 2. Unstaged Changes */}
                {unstagedFiles.length > 0 && (
                  <div>
                    <div className="mb-1.5 flex items-center justify-between text-[10px] font-semibold uppercase tracking-wider text-amber-400">
                      <span>Unstaged Changes ({unstagedFiles.length})</span>
                    </div>
                    <div className="space-y-1">
                      {unstagedFiles.map((file) => {
                        const isSelected = selectedFile?.path === file.path && !selectedFile?.is_staged;
                        return (
                          <div
                            key={`unstaged-${file.path}`}
                            onClick={() => setSelectedFile(file)}
                            className={`group flex items-center justify-between rounded px-2 py-1.5 cursor-pointer text-[11px] transition ${
                              isSelected
                                ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium border-l-2 border-amber-500'
                                : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-1)] hover:text-[var(--color-editor-fg)]'
                            }`}
                          >
                            <div className="flex items-center gap-1.5 truncate">
                              <span className="font-mono text-[10px] text-amber-400 font-bold">{file.status}</span>
                              <span className="truncate">{file.path}</span>
                            </div>
                            <div className="flex items-center gap-1">
                              {(file.additions > 0 || file.deletions > 0) && (
                                <span className="font-mono text-[9px] text-[var(--color-fg-subtle)]">
                                  +{file.additions} -{file.deletions}
                                </span>
                              )}
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleStage(file.path);
                                }}
                                title="Stage file"
                                className="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-[var(--color-surface-3)] rounded text-[var(--color-fg-muted)] hover:text-emerald-400 transition"
                              >
                                <Plus className="h-3 w-3" />
                              </button>
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleDiscard(file.path);
                                }}
                                title="Discard changes"
                                className="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-[var(--color-surface-3)] rounded text-[var(--color-fg-muted)] hover:text-red-400 transition"
                              >
                                <Trash2 className="h-3 w-3" />
                              </button>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}

                {/* 3. Untracked Files */}
                {untrackedFiles.length > 0 && (
                  <div>
                    <div className="mb-1.5 flex items-center justify-between text-[10px] font-semibold uppercase tracking-wider text-zinc-400">
                      <span>Untracked Files ({untrackedFiles.length})</span>
                    </div>
                    <div className="space-y-1">
                      {untrackedFiles.map((file) => {
                        const isSelected = selectedFile?.path === file.path && !selectedFile?.is_staged;
                        return (
                          <div
                            key={`untracked-${file.path}`}
                            onClick={() => setSelectedFile(file)}
                            className={`group flex items-center justify-between rounded px-2 py-1.5 cursor-pointer text-[11px] transition ${
                              isSelected
                                ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium border-l-2 border-zinc-400'
                                : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-1)] hover:text-[var(--color-editor-fg)]'
                            }`}
                          >
                            <div className="flex items-center gap-1.5 truncate">
                              <span className="font-mono text-[10px] text-zinc-400 font-bold">??</span>
                              <span className="truncate">{file.path}</span>
                            </div>
                            <div className="flex items-center gap-1">
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleStage(file.path);
                                }}
                                title="Stage file"
                                className="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-[var(--color-surface-3)] rounded text-[var(--color-fg-muted)] hover:text-emerald-400 transition"
                              >
                                <Plus className="h-3 w-3" />
                              </button>
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleDiscard(file.path);
                                }}
                                title="Delete file"
                                className="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-[var(--color-surface-3)] rounded text-[var(--color-fg-muted)] hover:text-red-400 transition"
                              >
                                <Trash2 className="h-3 w-3" />
                              </button>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </>
            )}
          </div>
        </div>

        {/* Right Side: Diff Viewer */}
        <div className="flex-1 flex flex-col bg-[var(--color-canvas)]">
          {selectedFile ? (
            <>
              {/* Header */}
              <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-4 py-2">
                <div className="flex items-center gap-2">
                  <FileCode className="h-4 w-4 text-[var(--color-fg-muted)]" />
                  <span className="font-mono text-xs font-semibold text-[var(--color-editor-fg)]">
                    {selectedFile.path}
                  </span>
                  <span className={`rounded px-1.5 py-0.2 text-[10px] font-semibold uppercase ${
                    selectedFile.is_staged
                      ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
                      : 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
                  }`}>
                    {selectedFile.is_staged ? 'Staged' : 'Working Tree'}
                  </span>
                </div>

                <div className="flex items-center gap-2">
                  <button
                    onClick={handleCopyDiff}
                    className="flex items-center gap-1 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-1 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
                  >
                    {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
                    <span>{copied ? 'Copied' : 'Copy Diff'}</span>
                  </button>

                  {selectedFile.is_staged ? (
                    <button
                      onClick={() => handleUnstage(selectedFile.path)}
                      disabled={!!actionInProgress}
                      className="flex items-center gap-1 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-1 text-xs text-[var(--color-fg-muted)] hover:text-amber-400 transition disabled:opacity-50"
                    >
                      <Minus className="h-3 w-3" />
                      <span>{actionInProgress === `unstage-${selectedFile.path}` ? 'Unstaging...' : 'Unstage'}</span>
                    </button>
                  ) : (
                    <>
                      <button
                        onClick={() => handleStage(selectedFile.path)}
                        disabled={!!actionInProgress}
                        className="flex items-center gap-1 rounded bg-emerald-600 px-2.5 py-1 text-xs text-white hover:bg-emerald-500 transition shadow-sm disabled:opacity-50"
                      >
                        <Plus className="h-3 w-3" />
                        <span>{actionInProgress === `stage-${selectedFile.path}` ? 'Staging...' : 'Stage'}</span>
                      </button>
                      <button
                        onClick={() => handleDiscard(selectedFile.path)}
                        disabled={!!actionInProgress}
                        className="flex items-center gap-1 rounded border border-red-500/20 bg-red-500/10 px-2 py-1 text-xs text-red-400 hover:bg-red-500/20 transition disabled:opacity-50"
                      >
                        <Trash2 className="h-3 w-3" />
                        <span>{actionInProgress === `discard-${selectedFile.path}` ? 'Discarding...' : 'Discard'}</span>
                      </button>
                    </>
                  )}
                </div>
              </div>

              {/* Diff Content View */}
              <div className="flex-1 overflow-auto p-4 font-mono text-xs leading-relaxed selection:bg-zinc-800">
                {loadingFileDiff ? (
                  <div className="flex items-center justify-center p-8 text-xs text-[var(--color-fg-muted)]">
                    <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                    Loading diff...
                  </div>
                ) : fileDiff && fileDiff.diff ? (
                  <div className="space-y-0.5">
                    {fileDiff.diff.split('\n').map((line, idx) => {
                      let lineClass = 'text-[var(--color-fg-muted)]';
                      let bgClass = '';
                      if (line.startsWith('@@')) {
                        lineClass = 'text-cyan-400 font-semibold';
                        bgClass = 'bg-cyan-950/20';
                      } else if (line.startsWith('+') && !line.startsWith('+++')) {
                        lineClass = 'text-emerald-300';
                        bgClass = 'bg-emerald-950/30';
                      } else if (line.startsWith('-') && !line.startsWith('---')) {
                        lineClass = 'text-red-300';
                        bgClass = 'bg-red-950/30';
                      }

                      return (
                        <div key={idx} className={`flex px-2 py-0.5 rounded-sm ${bgClass}`}>
                          <span className="w-8 shrink-0 select-none text-[var(--color-fg-subtle)] text-right pr-3 opacity-50">
                            {idx + 1}
                          </span>
                          <span className={`whitespace-pre ${lineClass}`}>{line || ' '}</span>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <div className="p-8 text-center text-[11px] text-[var(--color-fg-muted)]">
                    {selectedFile.status === '??' ? (
                      <div>
                        <p className="font-semibold text-[var(--color-editor-fg)]">Untracked File</p>
                        <p className="mt-1">Stage this file to inspect line additions.</p>
                      </div>
                    ) : (
                      'No diff content available for this file.'
                    )}
                  </div>
                )}
              </div>
            </>
          ) : (
            <div className="flex flex-1 flex-col items-center justify-center p-8 text-center text-[var(--color-fg-muted)]">
              <FileDiff className="h-12 w-12 stroke-[1.2] opacity-40 mb-3" />
              <h3 className="text-xs font-semibold text-[var(--color-editor-fg)]">
                No File Selected
              </h3>
              <p className="mt-1 text-[11px] max-w-sm">
                Select a modified, staged, or untracked file on the left to review unified diffs.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
