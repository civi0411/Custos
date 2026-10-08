import React, { useEffect, useState } from 'react';
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  FileCode,
  FileMinus,
  FilePlus,
  Folder,
  GitBranch,
  GitCommit,
  HardDrive,
  RefreshCw,
  Server,
  ShieldAlert,
  Trash2,
  Wrench,
  X,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { DirtyManifest, ExecutionWorkspace } from '@/types/domain';

const kindLabel = (workspace: ExecutionWorkspace) => {
  if (workspace.kind.type === 'git') return 'Git worktree';
  if (workspace.kind.type === 'remote_ssh') return 'Remote SSH';
  return 'Folder';
};

const KindIcon = ({ workspace }: { workspace: ExecutionWorkspace }) => {
  if (workspace.kind.type === 'git') return <GitBranch className="h-4 w-4" />;
  if (workspace.kind.type === 'remote_ssh') return <Server className="h-4 w-4" />;
  return <Folder className="h-4 w-4" />;
};

const isDirty = (m?: DirtyManifest | null): boolean => {
  if (!m) return false;
  return Boolean(m.is_dirty ?? m.isDirty);
};

const getModified = (m?: DirtyManifest | null): string[] => m?.modified_files ?? m?.modifiedFiles ?? [];
const getUntracked = (m?: DirtyManifest | null): string[] => m?.untracked_files ?? m?.untrackedFiles ?? [];
const getDeleted = (m?: DirtyManifest | null): string[] => m?.deleted_files ?? m?.deletedFiles ?? [];
const getChangesCount = (m?: DirtyManifest | null): number =>
  getModified(m).length + getUntracked(m).length + getDeleted(m).length;

interface ArchiveDialogState {
  workspace: ExecutionWorkspace;
  deletePhysical: boolean;
  force: boolean;
}

export const ExecutionWorkspacesPane: React.FC = () => {
  const [records, setRecords] = useState<ExecutionWorkspace[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [reconciling, setReconciling] = useState(false);
  const [expandedDirtyId, setExpandedDirtyId] = useState<string | null>(null);
  const [liveManifests, setLiveManifests] = useState<Record<string, DirtyManifest>>({});
  const [inspectingId, setInspectingId] = useState<string | null>(null);
  const [archiveDialog, setArchiveDialog] = useState<ArchiveDialogState | null>(null);
  const [archiving, setArchiving] = useState(false);

  const reload = () => {
    setLoading(true);
    setError(null);
    daemonClient.listWorkspaces()
      .then(setRecords)
      .catch((reason) => setError(String(reason)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, []);

  const handleInspectDirty = async (workspaceId: string) => {
    if (expandedDirtyId === workspaceId) {
      setExpandedDirtyId(null);
      return;
    }
    setInspectingId(workspaceId);
    try {
      const manifest = await daemonClient.inspectWorkspaceDirty(workspaceId);
      setLiveManifests((prev) => ({ ...prev, [workspaceId]: manifest }));
      setExpandedDirtyId(workspaceId);
    } catch (err: any) {
      setError(`Failed to inspect dirty manifest for ${workspaceId}: ${err?.message || err}`);
    } finally {
      setInspectingId(null);
    }
  };

  const handleRecover = async (workspaceId?: string) => {
    setReconciling(true);
    setError(null);
    setNotice(null);
    try {
      const res = await daemonClient.recoverWorkspace(workspaceId);
      const count = Array.isArray(res) ? res.length : 1;
      setNotice(
        workspaceId
          ? `Workspace ${workspaceId} verified and reconciled successfully.`
          : `Reconciled and restored ${count} workspace(s).`
      );
      reload();
    } catch (err: any) {
      setError(`Recovery error: ${err?.message || err}`);
    } finally {
      setReconciling(false);
    }
  };

  const confirmArchive = async () => {
    if (!archiveDialog) return;
    setArchiving(true);
    setError(null);
    try {
      await daemonClient.archiveWorkspace(
        archiveDialog.workspace.id,
        archiveDialog.deletePhysical,
        archiveDialog.force
      );
      setNotice(`Workspace ${archiveDialog.workspace.id} archived successfully.`);
      setArchiveDialog(null);
      reload();
    } catch (err: any) {
      setError(`Failed to archive workspace: ${err?.message || err}`);
    } finally {
      setArchiving(false);
    }
  };

  const dirtyCount = records.filter(
    (w) => isDirty(liveManifests[w.id] ?? w.dirty_manifest ?? w.dirtyManifest)
  ).length;

  return (
    <div className="h-full overflow-y-auto bg-[var(--color-canvas)] p-6 text-[var(--color-editor-fg)]">
      <div className="mx-auto max-w-5xl">
        <header className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className="workbench-kicker">execution plane</div>
            <h1 className="mt-1 text-lg font-semibold tracking-[-0.02em]">Workspaces</h1>
            <p className="mt-1 max-w-2xl text-xs leading-5 text-[var(--color-fg-muted)]">
              Daemon-owned folder and Git workspace records. Safe Git ownership, base commit
              hashes, porcelain dirty manifests, and host recovery.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => handleRecover()}
              disabled={loading || reconciling}
              className="flex items-center gap-1.5 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-2.5 py-1.5 text-xs font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-surface-2)] disabled:opacity-50"
              title="Verify and reconcile all registered workspaces with host"
            >
              <Wrench className={`h-3.5 w-3.5 ${reconciling ? 'animate-spin' : ''}`} />
              <span>{reconciling ? 'Reconciling...' : 'Reconcile All'}</span>
            </button>
            <button
              type="button"
              onClick={reload}
              disabled={loading}
              className="rounded-md border border-[var(--color-border-default)] p-2 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] disabled:opacity-50"
              aria-label="Refresh workspaces"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${loading ? 'animate-spin' : ''}`} />
            </button>
          </div>
        </header>

        {/* Status Metrics Bar */}
        <div className="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3">
            <div className="text-[10px] font-medium uppercase tracking-[0.06em] text-[var(--color-fg-subtle)]">
              Total Workspaces
            </div>
            <div className="mt-1 text-base font-semibold">{records.length}</div>
          </div>
          <div className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3">
            <div className="text-[10px] font-medium uppercase tracking-[0.06em] text-[var(--color-fg-subtle)]">
              Ready
            </div>
            <div className="mt-1 text-base font-semibold text-emerald-400">
              {records.filter((w) => w.status === 'ready').length}
            </div>
          </div>
          <div className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3">
            <div className="text-[10px] font-medium uppercase tracking-[0.06em] text-[var(--color-fg-subtle)]">
              Dirty Manifests
            </div>
            <div className={`mt-1 text-base font-semibold ${dirtyCount > 0 ? 'text-amber-400' : 'text-[var(--color-fg-muted)]'}`}>
              {dirtyCount}
            </div>
          </div>
          <div className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3">
            <div className="text-[10px] font-medium uppercase tracking-[0.06em] text-[var(--color-fg-subtle)]">
              Setup Issues
            </div>
            <div className="mt-1 text-base font-semibold text-rose-400">
              {records.filter((w) => w.status === 'setup_failed').length}
            </div>
          </div>
        </div>

        {notice && (
          <div className="mt-4 flex items-center justify-between rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-3 text-xs text-emerald-400">
            <span>{notice}</span>
            <button type="button" onClick={() => setNotice(null)} className="text-emerald-400 hover:text-emerald-300">
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        )}

        {error && (
          <div className="mt-4 flex items-center justify-between rounded-lg border border-rose-500/20 bg-rose-500/5 p-3 text-xs text-rose-400">
            <span>{error}</span>
            <button type="button" onClick={() => setError(null)} className="text-rose-400 hover:text-rose-300">
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        )}

        <div className="mt-6 space-y-3">
          {records.map((workspace) => {
            const manifest = liveManifests[workspace.id] ?? workspace.dirty_manifest ?? workspace.dirtyManifest;
            const workspaceIsDirty = isDirty(manifest);
            const changesCount = getChangesCount(manifest);
            const isGit = workspace.kind.type === 'git';
            const isExpanded = expandedDirtyId === workspace.id;
            const baseHash = workspace.base_commit_hash ?? workspace.baseCommitHash ?? workspace.lineage?.base_commit;
            const headHash = manifest?.head_commit ?? manifest?.headCommit ?? workspace.lineage?.head_commit;
            const ownerTask = workspace.owner_task_id ?? workspace.ownerTaskId;

            return (
              <article
                key={workspace.id}
                className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 transition-colors hover:border-[var(--color-border-default)]"
              >
                <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                  <div className="flex min-w-0 gap-3">
                    <div className="mt-0.5 text-[var(--color-fg-muted)]">
                      <KindIcon workspace={workspace} />
                    </div>
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <h2 className="truncate text-xs font-semibold">{workspace.name}</h2>
                        {isGit && (
                          <span
                            className={`inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[9px] font-medium tracking-[0.02em] ${
                              workspaceIsDirty
                                ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
                                : 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
                            }`}
                          >
                            {workspaceIsDirty ? (
                              <>
                                <AlertTriangle className="h-2.5 w-2.5" />
                                <span>Dirty ({changesCount})</span>
                              </>
                            ) : (
                              <>
                                <CheckCircle2 className="h-2.5 w-2.5" />
                                <span>Clean</span>
                              </>
                            )}
                          </span>
                        )}
                      </div>
                      <p className="mt-1 truncate font-mono text-[10px] text-[var(--color-fg-subtle)]">
                        {workspace.path}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 self-start sm:self-auto">
                    <span className="rounded bg-[var(--color-surface-2)] px-2 py-0.5 text-[9px] font-semibold uppercase tracking-[0.08em] text-[var(--color-fg-muted)]">
                      {workspace.status}
                    </span>
                    <button
                      type="button"
                      onClick={() => handleRecover(workspace.id)}
                      disabled={reconciling}
                      className="rounded border border-[var(--color-border-muted)] p-1 text-[var(--color-fg-subtle)] hover:border-[var(--color-border-default)] hover:text-[var(--color-fg-default)]"
                      title="Verify and recover worktree"
                    >
                      <Wrench className="h-3 w-3" />
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        setArchiveDialog({
                          workspace,
                          deletePhysical: false,
                          force: false,
                        })
                      }
                      className="rounded border border-[var(--color-border-muted)] p-1 text-[var(--color-fg-subtle)] hover:border-rose-500/30 hover:text-rose-400"
                      title="Archive workspace"
                    >
                      <Trash2 className="h-3 w-3" />
                    </button>
                  </div>
                </div>

                {/* Lineage & Ownership Details */}
                <div className="mt-4 flex flex-wrap items-center gap-x-4 gap-y-1.5 border-t border-[var(--color-border-muted)] pt-3 text-[10px] text-[var(--color-fg-subtle)]">
                  <span className="font-medium text-[var(--color-fg-muted)]">{kindLabel(workspace)}</span>
                  <span className="font-mono text-[var(--color-fg-muted)]">{workspace.id}</span>

                  {ownerTask ? (
                    <span className="inline-flex items-center gap-1 rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[9px] text-[var(--color-fg-default)]">
                      task:{ownerTask.slice(0, 8)}
                    </span>
                  ) : (
                    <span className="text-[9px] text-[var(--color-fg-subtle)]">unassigned task</span>
                  )}

                  {baseHash && (
                    <span className="inline-flex items-center gap-1 font-mono">
                      <GitCommit className="h-3 w-3 text-[var(--color-fg-subtle)]" />
                      base {baseHash.slice(0, 7)}
                    </span>
                  )}

                  {headHash && (
                    <span className="inline-flex items-center gap-1 font-mono">
                      <GitBranch className="h-3 w-3 text-[var(--color-fg-subtle)]" />
                      head {headHash.slice(0, 7)}
                    </span>
                  )}

                  {isGit && (
                    <button
                      type="button"
                      onClick={() => handleInspectDirty(workspace.id)}
                      disabled={inspectingId === workspace.id}
                      className="ml-auto inline-flex items-center gap-1 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-0.5 text-[9px] font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-surface-3)]"
                    >
                      {inspectingId === workspace.id ? (
                        <RefreshCw className="h-2.5 w-2.5 animate-spin" />
                      ) : isExpanded ? (
                        <ChevronDown className="h-2.5 w-2.5" />
                      ) : (
                        <ChevronRight className="h-2.5 w-2.5" />
                      )}
                      <span>Inspect Porcelain</span>
                    </button>
                  )}
                </div>

                {workspace.status_reason && (
                  <p className="mt-3 text-[10px] text-rose-400">{workspace.status_reason}</p>
                )}

                {/* Porcelain Dirty Inspection Drawer */}
                {isExpanded && (
                  <div className="mt-3 rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] p-3">
                    <div className="flex items-center justify-between text-[10px]">
                      <span className="font-semibold text-[var(--color-fg-default)]">
                        Porcelain Status Manifest
                      </span>
                      <span className="font-mono text-[9px] text-[var(--color-fg-subtle)]">
                        {manifest?.checked_at || manifest?.checkedAt
                          ? `Checked: ${new Date((manifest.checked_at ?? manifest.checkedAt ?? 0) * 1000).toLocaleTimeString()}`
                          : 'Not checked yet'}
                      </span>
                    </div>

                    {!workspaceIsDirty && changesCount === 0 ? (
                      <div className="mt-2 text-[10px] text-emerald-400">
                        Working directory is completely clean. No modified, untracked, or deleted files.
                      </div>
                    ) : (
                      <div className="mt-2 space-y-1.5 text-[10px] font-mono">
                        {getModified(manifest).map((file) => (
                          <div key={file} className="flex items-center gap-2 text-amber-300">
                            <FileCode className="h-3 w-3 shrink-0" />
                            <span className="rounded bg-amber-500/10 px-1 py-0.2 text-[8px] font-bold">M</span>
                            <span className="truncate">{file}</span>
                          </div>
                        ))}
                        {getUntracked(manifest).map((file) => (
                          <div key={file} className="flex items-center gap-2 text-sky-300">
                            <FilePlus className="h-3 w-3 shrink-0" />
                            <span className="rounded bg-sky-500/10 px-1 py-0.2 text-[8px] font-bold">?</span>
                            <span className="truncate">{file}</span>
                          </div>
                        ))}
                        {getDeleted(manifest).map((file) => (
                          <div key={file} className="flex items-center gap-2 text-rose-300">
                            <FileMinus className="h-3 w-3 shrink-0" />
                            <span className="rounded bg-rose-500/10 px-1 py-0.2 text-[8px] font-bold">D</span>
                            <span className="truncate">{file}</span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </article>
            );
          })}
        </div>

        {!loading && !error && records.length === 0 && (
          <div className="mt-12 text-center">
            <HardDrive className="mx-auto h-5 w-5 text-[var(--color-fg-subtle)]" />
            <p className="mt-3 text-xs text-[var(--color-fg-muted)]">
              No execution workspaces have been registered.
            </p>
          </div>
        )}
      </div>

      {/* Safe Archive Modal */}
      {archiveDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
          <div className="w-full max-w-md rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-5 text-[var(--color-editor-fg)] shadow-2xl">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-semibold">Archive Workspace</h2>
              <button
                type="button"
                onClick={() => setArchiveDialog(null)}
                className="text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <p className="mt-2 text-xs text-[var(--color-fg-muted)]">
              Archiving will retire workspace record{' '}
              <span className="font-mono text-[var(--color-fg-default)]">
                {archiveDialog.workspace.name}
              </span>{' '}
              ({archiveDialog.workspace.id}).
            </p>

            {isDirty(
              liveManifests[archiveDialog.workspace.id] ??
                archiveDialog.workspace.dirty_manifest ??
                archiveDialog.workspace.dirtyManifest
            ) && (
              <div className="mt-3 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-300">
                <div className="flex items-center gap-1.5 font-semibold">
                  <ShieldAlert className="h-4 w-4" />
                  <span>Uncommitted Edits Detected</span>
                </div>
                <p className="mt-1 text-[11px] leading-4 text-amber-200/90">
                  This worktree is dirty. In accordance with fail-closed workspace invariants,
                  physical paths cannot be destroyed without explicit force confirmation.
                </p>
              </div>
            )}

            <div className="mt-4 space-y-2.5 text-xs">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={archiveDialog.deletePhysical}
                  onChange={(e) =>
                    setArchiveDialog({
                      ...archiveDialog,
                      deletePhysical: e.target.checked,
                    })
                  }
                  className="rounded border-[var(--color-border-default)]"
                />
                <span>Delete physical directory from disk</span>
              </label>

              {archiveDialog.deletePhysical && (
                <label className="flex items-center gap-2 cursor-pointer text-amber-400">
                  <input
                    type="checkbox"
                    checked={archiveDialog.force}
                    onChange={(e) =>
                      setArchiveDialog({
                        ...archiveDialog,
                        force: e.target.checked,
                      })
                    }
                    className="rounded border-amber-500/50"
                  />
                  <span>Force delete even if uncommitted changes exist</span>
                </label>
              )}
            </div>

            <div className="mt-5 flex justify-end gap-2 text-xs">
              <button
                type="button"
                onClick={() => setArchiveDialog(null)}
                className="rounded-md border border-[var(--color-border-default)] px-3 py-1.5 hover:bg-[var(--color-surface-2)]"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={confirmArchive}
                disabled={archiving}
                className="rounded-md bg-rose-600 px-3 py-1.5 font-medium text-white hover:bg-rose-500 disabled:opacity-50"
              >
                {archiving ? 'Archiving...' : 'Confirm Archive'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
