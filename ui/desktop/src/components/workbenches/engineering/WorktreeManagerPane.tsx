import React, { useEffect, useState, useCallback } from 'react';
import {
  GitBranch,
  Bot,
  Terminal,
  FileDiff,
  GitMerge,
  MoreVertical,
  Folder,
  RefreshCw,
  GitCompare,
  Plus,
  X,
  CheckCircle2,
  Loader2,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import type { ExecutionWorkspace, DirtyManifest, Task } from '@/types/domain';

export interface WorktreeManagerPaneProps {
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  onShowToast?: (msg: string) => void;
  onOpenWorkspaceResource?: (resource: 'files' | 'terminal' | 'agents', workspaceId: string) => void;
}

interface CandidateComparison {
  workspace_id: string;
  name: string;
  path: string;
  branch: string;
  base_commit_hash?: string;
  is_dirty: boolean;
  modified_count: number;
  untracked_count: number;
  deleted_count: number;
  modified_files: string[];
  untracked_files: string[];
  deleted_files: string[];
  diff: string;
  status: string;
}

export const WorktreeManagerPane: React.FC<WorktreeManagerPaneProps> = ({
  activeWorkspaceId: propActiveWorkspaceId,
  onSelectWorkspace: propOnSelectWorkspace,
  onShowToast,
  onOpenWorkspaceResource,
}) => {
  const { activeWorkspaceId: ctxActiveWorkspaceId, setActiveWorkspaceId: ctxSetActiveWorkspaceId } = useAppContext();
  const currentActiveWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const onSelectWorkspace = propOnSelectWorkspace ?? ctxSetActiveWorkspaceId;

  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(currentActiveWorkspaceId || null);
  const [dirtyManifests, setDirtyManifests] = useState<Record<string, DirtyManifest>>({});
  const [showSecondaryMenu, setShowSecondaryMenu] = useState<string | null>(null);

  // Compare Candidates state
  const [showCompareModal, setShowCompareModal] = useState(false);
  const [comparing, setComparing] = useState(false);
  const [candidateComparisons, setCandidateComparisons] = useState<CandidateComparison[]>([]);
  const [integratingId, setIntegratingId] = useState<string | null>(null);

  // Spawn Candidates state
  const [showSpawnModal, setShowSpawnModal] = useState(false);
  const [spawnTaskId, setSpawnTaskId] = useState('');
  const [spawnCount, setSpawnCount] = useState(2);
  const [spawning, setSpawning] = useState(false);

  useEffect(() => {
    daemonClient.listTasks().then(setTasks).catch((err) => {
      onShowToast?.(`Failed to load tasks: ${err?.message || err}`);
    });
  }, [onShowToast]);

  const loadWorkspaces = useCallback(async () => {
    try {
      setLoading(true);
      const list = await daemonClient.listWorkspaces();
      const active = list.filter((w) => w.status !== 'archived');
      setWorkspaces(active);
      if (active.length > 0 && !selectedWorkspaceId) {
        setSelectedWorkspaceId(active[0].id);
      }

      // Pre-fetch dirty manifests
      for (const ws of active) {
        daemonClient
          .inspectWorkspaceDirty(ws.id)
          .then((manifest) => {
            setDirtyManifests((prev) => ({ ...prev, [ws.id]: manifest }));
          })
          .catch(() => {});
      }
    } catch (err: any) {
      onShowToast?.(`Failed to load workspaces: ${err?.message || err}`);
    } finally {
      setLoading(false);
    }
  }, [selectedWorkspaceId, onShowToast]);

  useEffect(() => {
    loadWorkspaces();
  }, [loadWorkspaces]);

  useEffect(() => {
    if (currentActiveWorkspaceId && currentActiveWorkspaceId !== selectedWorkspaceId) {
      setSelectedWorkspaceId(currentActiveWorkspaceId);
    }
  }, [currentActiveWorkspaceId, selectedWorkspaceId]);

  const handleOpenTerminal = (workspace: ExecutionWorkspace) => {
    setShowSecondaryMenu(null);
    onOpenWorkspaceResource?.('terminal', workspace.id);
  };

  const handleViewDiff = (workspace: ExecutionWorkspace) => {
    setShowSecondaryMenu(null);
    onOpenWorkspaceResource?.('files', workspace.id);
  };

  const handleRecoverWorkspace = async (workspace: ExecutionWorkspace) => {
    setShowSecondaryMenu(null);
    try {
      onShowToast?.(`Reconciling and recovering worktree ${workspace.name || workspace.id}...`);
      await daemonClient.recoverWorkspace(workspace.id);
      onShowToast?.(`Worktree ${workspace.id.slice(0, 8)} status successfully reconciled.`);
      await loadWorkspaces();
    } catch (err: any) {
      onShowToast?.(`Recovery failed: ${err?.message || err}`);
    }
  };

  // Compare candidates across all active worktrees
  const handleCompareAll = async () => {
    try {
      setComparing(true);
      setShowCompareModal(true);
      const res = await daemonClient.compareCandidates({
        workspace_ids: workspaces.map((w) => w.id),
      });
      setCandidateComparisons(res.candidates || []);
    } catch (err: any) {
      onShowToast?.(`Failed to compare candidates: ${err?.message || err}`);
    } finally {
      setComparing(false);
    }
  };

  // Integrate winner into base branch (Packet T4)
  const handleIntegrateWorkspace = async (winnerWsId: string) => {
    setShowSecondaryMenu(null);
    const winner = workspaces.find((workspace) => workspace.id === winnerWsId);
    if (!winner) {
      onShowToast?.('Cannot integrate: the selected worktree is no longer available. Refresh the list and compare again.');
      return;
    }
    const targetBranch = winner.lineage?.target_branch || 'the configured base branch';
    const confirmed = window.confirm(
      `Integrate ${winner.active_branch || winner.name || winner.id} into ${targetBranch}?\n\nThis changes the base repository and may encounter merge conflicts. Review the candidate diff first.`
    );
    if (!confirmed) return;

    setIntegratingId(winnerWsId);
    try {
      const loserIds = workspaces.filter((w) => w.id !== winnerWsId).map((w) => w.id);

      onShowToast?.(`Integrating winner ${winner?.name || winnerWsId.slice(0, 8)} via Gate 4...`);
      const res = await daemonClient.integrateCandidate({
        winner_workspace_id: winnerWsId,
        loser_workspace_ids: loserIds,
        force_loser_cleanup: false,
      });

      if (res.is_stale_base) {
        onShowToast?.(
          `Stale-Base Notice: Target HEAD moved beyond base commit (${res.winner_base_commit?.slice(0, 7) || 'unknown'}). Merging...`
        );
      }

      if (res.merge_success) {
        onShowToast?.(
          `Successfully integrated candidate branch ${res.winner_branch} into ${res.target_branch}!`
        );
      } else {
        onShowToast?.(`Merge encountered conflict: ${res.merge_detail}`);
      }

      await loadWorkspaces();
      if (showCompareModal) {
        await handleCompareAll();
      }
    } catch (err: any) {
      onShowToast?.(`Integration failed: ${err?.message || err}`);
    } finally {
      setIntegratingId(null);
    }
  };

  // Spawn parallel candidates
  const handleSpawnSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const taskId = spawnTaskId.trim();
    if (!tasks.some((task) => task.id === taskId)) {
      onShowToast?.('Choose an existing Task before creating candidate worktrees.');
      return;
    }
    const baseWs = workspaces.find((w) => w.id === selectedWorkspaceId) || workspaces[0];
    if (!baseWs) {
      onShowToast?.('No base workspace available to spawn from.');
      return;
    }

    setSpawning(true);
    try {
      onShowToast?.(`Creating ${spawnCount} candidate worktrees for Task ${taskId}...`);
      await daemonClient.spawnCandidates({
        task_id: taskId,
        base_workspace_id: baseWs.id,
        count: spawnCount,
      });
      onShowToast?.(`Created ${spawnCount} worktrees for Task ${taskId}. No agent was started.`);
      setShowSpawnModal(false);
      setSpawnTaskId('');
      await loadWorkspaces();
    } catch (err: any) {
      onShowToast?.(`Failed to spawn candidates: ${err?.message || err}`);
    } finally {
      setSpawning(false);
    }
  };

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Header */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-3 bg-[var(--color-surface-1)]/50">
        <div className="flex items-center gap-2">
          <GitBranch className="h-4 w-4 text-[var(--color-fg-muted)] shrink-0" />
          <span className="text-xs font-semibold">Git worktrees</span>
          <span className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] font-mono text-[var(--color-fg-subtle)]">
            {workspaces.length} active
          </span>
        </div>

        <div className="flex items-center gap-2">
          {workspaces.length >= 2 && (
            <button
              type="button"
              onClick={handleCompareAll}
              className="flex items-center gap-1.5 rounded border border-blue-500/30 bg-blue-500/10 px-2.5 py-1 text-xs text-blue-400 hover:bg-blue-500/20 transition"
              title="Compare candidates side-by-side"
            >
              <GitCompare className="h-3 w-3" />
              <span>Compare Candidates</span>
            </button>
          )}

          <button
            type="button"
            onClick={() => setShowSpawnModal(true)}
            className="flex items-center gap-1.5 rounded border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-1 text-xs text-emerald-400 hover:bg-emerald-500/20 transition"
            title="Spawn new parallel candidate worktrees"
          >
            <Plus className="h-3 w-3" />
            <span>Spawn Candidates</span>
          </button>

          <button
            type="button"
            onClick={loadWorkspaces}
            className="flex items-center gap-1.5 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] px-2.5 py-1 text-xs text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] transition"
          >
            <RefreshCw className="h-3 w-3" />
            <span>Refresh</span>
          </button>
        </div>
      </div>

      {/* Worktrees Card Grid */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {loading && workspaces.length === 0 ? (
          <div className="flex h-32 items-center justify-center text-xs text-[var(--color-fg-subtle)]">
            Loading active worktrees...
          </div>
        ) : workspaces.length === 0 ? (
          <div className="rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-6 text-center text-xs text-[var(--color-fg-muted)]">
            <p>No active git worktrees registered.</p>
              <p className="mt-1 text-[11px] text-[var(--color-fg-subtle)]">
              Worktrees are isolated checkouts linked to a Task. Starting and steering agents requires an attached runtime session.
            </p>
            <button
              type="button"
              onClick={() => setShowSpawnModal(true)}
              className="mt-3 inline-flex items-center gap-1.5 rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-500 transition"
            >
              <Plus className="h-3.5 w-3.5" />
              <span>Spawn Parallel Candidates</span>
            </button>
          </div>
        ) : (
          workspaces.map((ws) => {
            const manifest = dirtyManifests[ws.id];
            const isDirty = manifest?.is_dirty || manifest?.isDirty;
            const dirtyCount =
              (manifest?.modified_files?.length || 0) +
              (manifest?.untracked_files?.length || 0) +
              (manifest?.deleted_files?.length || 0);

            const agentName = ws.running_agent_id || 'no agent session';
            const agentStatus = ws.agent_status || 'idle';
            const branchName = ws.active_branch || (ws.kind.type === 'git' ? ws.kind.branch || 'main' : 'default');

            const isSelected = ws.id === selectedWorkspaceId;
            return (
              <div
                key={ws.id}
                onClick={() => {
                  setSelectedWorkspaceId(ws.id);
                  onSelectWorkspace?.(ws.id);
                }}
                className={`rounded-xl border p-3.5 shadow-xs transition cursor-pointer ${
                  isSelected
                    ? 'border-[var(--workbench-accent)] bg-[var(--color-surface-2)]/60 ring-1 ring-[var(--workbench-accent)]/30'
                    : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] hover:border-[var(--color-border-default)]'
                }`}
              >
                {/* Top Row: Path, Branch, and Agent Badge */}
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <Folder className="h-4 w-4 text-amber-400 shrink-0" />
                      <span className="font-semibold text-xs truncate">
                        {ws.name || ws.path.split('/').pop() || ws.id}
                      </span>
                      <span className="flex items-center gap-1 rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] font-mono text-blue-400">
                        <GitBranch className="h-3 w-3" />
                        {branchName}
                      </span>
                    </div>
                    <p className="mt-1 font-mono text-[10px] text-[var(--color-fg-subtle)] truncate">
                      {ws.path}
                    </p>
                  </div>

                  {/* Agent Status Badge & Quick Integrate */}
                  <div className="flex items-center gap-1.5 shrink-0">
                    <span
                      className={`flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide border ${
                        agentStatus === 'running'
                          ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                          : agentStatus === 'review_needed'
                          ? 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                          : 'bg-zinc-500/10 text-[var(--color-fg-subtle)] border-zinc-500/20'
                      }`}
                    >
                      <Bot className="h-3 w-3" />
                      <span>{agentName}</span>
                      <span className="opacity-60">({agentStatus})</span>
                    </span>

                    {/* Secondary ··· Menu */}
                    <div className="relative">
                      <button
                        type="button"
                        onClick={() => setShowSecondaryMenu(showSecondaryMenu === ws.id ? null : ws.id)}
                        className="flex h-6 w-6 items-center justify-center rounded hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]"
                      >
                        <MoreVertical className="h-3.5 w-3.5" />
                      </button>

                      {showSecondaryMenu === ws.id && (
                        <div
                          className="absolute right-0 top-7 z-50 w-48 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] py-1 shadow-lg text-xs"
                          onMouseLeave={() => setShowSecondaryMenu(null)}
                        >
                          <button
                            type="button"
                            onClick={() => handleViewDiff(ws)}
                            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]"
                          >
                            <FileDiff className="h-3.5 w-3.5 text-blue-400" />
                            <span>View worktree files</span>
                          </button>
                          <button
                            type="button"
                            onClick={() => handleOpenTerminal(ws)}
                            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]"
                          >
                            <Terminal className="h-3.5 w-3.5" />
                            <span>Open worktree terminal</span>
                          </button>
                          <button
                            type="button"
                            onClick={() => handleRecoverWorkspace(ws)}
                            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sky-400 hover:bg-[var(--color-surface-2)]"
                          >
                            <RefreshCw className="h-3.5 w-3.5" />
                            <span>Recover Workspace State</span>
                          </button>
                          <button
                            type="button"
                            disabled={integratingId === ws.id}
                            onClick={() => handleIntegrateWorkspace(ws.id)}
                            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-emerald-400 hover:bg-[var(--color-surface-2)] disabled:opacity-40"
                          >
                            <GitMerge className="h-3.5 w-3.5" />
                            <span>Merge into Base (Gated)</span>
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                </div>

                {/* Status & Dirty Files Banner */}
                <div className="mt-2.5 flex items-center justify-between text-[11px] text-[var(--color-fg-muted)]">
                  <span className="flex items-center gap-1.5 font-mono">
                    <span
                      className={`h-1.5 w-1.5 rounded-full ${
                        isDirty ? 'bg-amber-400' : 'bg-emerald-400'
                      }`}
                    />
                    {isDirty ? `${dirtyCount} uncommitted changes` : 'Clean tree'}
                  </span>

                  <button
                    type="button"
                    disabled={integratingId === ws.id}
                    onClick={() => handleIntegrateWorkspace(ws.id)}
                    className="flex items-center gap-1 text-[11px] font-medium text-emerald-400 hover:underline disabled:opacity-50"
                  >
                    {integratingId === ws.id ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : (
                      <GitMerge className="h-3 w-3" />
                    )}
                    <span>Integrate Winner</span>
                  </button>
                </div>

                {!ws.running_agent_id && <p className="mt-3 border-t border-[var(--color-border-subtle)] pt-2.5 text-[10px] text-[var(--color-fg-subtle)]">Worktree only · no agent run attached</p>}
                <button
                  type="button"
                  onClick={() => onOpenWorkspaceResource?.('agents', ws.id)}
                  className="mt-2 inline-flex items-center gap-1.5 rounded-md border border-[var(--color-border-default)] px-2.5 py-1.5 text-[11px] font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-surface-2)]"
                >
                  <Bot className="h-3.5 w-3.5" />
                  Start agent in this worktree
                </button>
              </div>
            );
          })
        )}
      </div>

      {/* Compare Candidates Modal */}
      {showCompareModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
          <div className="flex h-[80vh] w-[90vw] max-w-5xl flex-col rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] shadow-2xl">
            {/* Modal Header */}
            <div className="flex items-center justify-between border-b border-[var(--color-border-subtle)] px-4 py-3 bg-[var(--color-surface-1)]">
              <div className="flex items-center gap-2">
                <GitCompare className="h-4 w-4 text-blue-400" />
                <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">
                  Candidate Race Compare View (OrCa)
                </h3>
                <span className="rounded bg-blue-500/10 px-2 py-0.5 text-[10px] font-mono text-blue-400">
                  {candidateComparisons.length} candidates
                </span>
              </div>
              <button
                type="button"
                onClick={() => setShowCompareModal(false)}
                className="rounded p-1 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {/* Modal Body */}
            <div className="flex-1 overflow-y-auto p-4 space-y-4">
              {comparing ? (
                <div className="flex h-48 items-center justify-center gap-2 text-xs text-[var(--color-fg-subtle)]">
                  <Loader2 className="h-4 w-4 animate-spin text-blue-400" />
                  <span>Computing candidate unified diffs and dirty manifests...</span>
                </div>
              ) : candidateComparisons.length === 0 ? (
                <div className="text-center py-12 text-xs text-[var(--color-fg-muted)]">
                  No candidates available for comparison.
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  {candidateComparisons.map((c) => (
                    <div
                      key={c.workspace_id}
                      className="flex flex-col rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4 shadow-xs"
                    >
                      <div className="flex items-start justify-between">
                        <div>
                          <div className="flex items-center gap-2">
                            <span className="font-semibold text-xs">{c.name}</span>
                            <span className="rounded bg-blue-500/15 px-1.5 py-0.5 text-[10px] font-mono text-blue-400">
                              {c.branch}
                            </span>
                          </div>
                          <p className="mt-0.5 text-[10px] font-mono text-[var(--color-fg-subtle)]">
                            Base: {c.base_commit_hash?.slice(0, 8) || 'unknown'}
                          </p>
                        </div>

                        <button
                          type="button"
                          disabled={integratingId === c.workspace_id}
                          onClick={() => handleIntegrateWorkspace(c.workspace_id)}
                          className="flex items-center gap-1 rounded bg-emerald-600 px-2.5 py-1 text-xs font-medium text-white hover:bg-emerald-500 disabled:opacity-40 transition"
                        >
                          {integratingId === c.workspace_id ? (
                            <Loader2 className="h-3 w-3 animate-spin" />
                          ) : (
                            <CheckCircle2 className="h-3 w-3" />
                          )}
                          <span>Integrate into base…</span>
                        </button>
                      </div>

                      {/* Diff Stats Banner */}
                      <div className="mt-3 flex items-center gap-3 text-[11px] font-mono">
                        <span className="text-amber-400">+{c.modified_count} modified</span>
                        <span className="text-blue-400">?{c.untracked_count} new</span>
                        <span className="text-red-400">-{c.deleted_count} deleted</span>
                      </div>

                      {/* Diff snippet */}
                      <div className="mt-3 flex-1 overflow-hidden rounded-lg border border-[var(--color-border-subtle)] bg-black/40 p-2.5 font-mono text-[11px] text-[var(--color-editor-fg)]">
                        {c.diff ? (
                          <pre className="max-h-56 overflow-auto whitespace-pre-wrap font-mono text-[10px] leading-relaxed text-[var(--color-fg-muted)]">
                            {c.diff}
                          </pre>
                        ) : (
                          <div className="py-8 text-center text-[10px] text-[var(--color-fg-subtle)] italic">
                            No uncommitted unified diff changes.
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Spawn Candidates Modal */}
      {showSpawnModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
          <div className="w-full max-w-md rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] p-5 shadow-2xl">
            <div className="flex items-center justify-between border-b border-[var(--color-border-subtle)] pb-3">
              <div className="flex items-center gap-2">
                <Plus className="h-4 w-4 text-emerald-400" />
                <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">
                  Spawn Parallel Candidate Worktrees
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setShowSpawnModal(false)}
                className="rounded p-1 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <form onSubmit={handleSpawnSubmit} className="mt-4 space-y-4 text-xs">
              <div>
                <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                  Existing Task
                </label>
                <select
                  required
                  value={spawnTaskId}
                  onChange={(e) => setSpawnTaskId(e.target.value)}
                  className="w-full rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-emerald-500"
                >
                  <option value="">Select a Task…</option>
                  {tasks.map((task) => <option key={task.id} value={task.id}>{task.title} · {task.id}</option>)}
                </select>
              </div>

              <div>
                <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                  Number of Candidates (Race)
                </label>
                <div className="flex gap-2">
                  {[2, 3, 4].map((n) => (
                    <button
                      key={n}
                      type="button"
                      onClick={() => setSpawnCount(n)}
                      className={`flex-1 rounded border py-1.5 font-medium transition ${
                        spawnCount === n
                          ? 'border-emerald-500 bg-emerald-500/15 text-emerald-400'
                          : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]'
                      }`}
                    >
                      {n} Candidates
                    </button>
                  ))}
                </div>
              </div>

              <div className="rounded-lg bg-[var(--color-surface-2)]/60 p-3 text-[11px] text-[var(--color-fg-subtle)] leading-relaxed">
                Creates isolated Git checkouts for the selected Task. This action does not launch coding agents.
              </div>

              <div className="flex justify-end gap-2 pt-2 border-t border-[var(--color-border-subtle)]">
                <button
                  type="button"
                  onClick={() => setShowSpawnModal(false)}
                  className="rounded px-3 py-1.5 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={spawning || !spawnTaskId || !tasks.some((task) => task.id === spawnTaskId)}
                  className="flex items-center gap-1.5 rounded bg-emerald-600 px-4 py-1.5 font-medium text-white hover:bg-emerald-500 disabled:opacity-40 transition"
                >
                  {spawning && <Loader2 className="h-3 w-3 animate-spin" />}
                  <span>Spawn Worktrees</span>
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
