import React, { useEffect, useState, useCallback, useRef } from 'react';
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
  Loader2,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import type { ExecutionWorkspace, DirtyManifest, Task } from '@/types/domain';
import { SpawnModal } from './SpawnModal';
import { CompareModal, type CandidateComparison } from './CompareModal';

export interface WorktreeManagerPaneProps {
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  onShowToast?: (msg: string) => void;
  onOpenWorkspaceResource?: (resource: 'files' | 'terminal' | 'agents', workspaceId: string) => void;
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
  const [integrationPreview, setIntegrationPreview] = useState<Awaited<ReturnType<typeof daemonClient.integrateCandidate>> | null>(null);
  const spawnAttempt = useRef<{ signature: string; commandId: string } | null>(null);


  // Spawn Candidates state
  const [showSpawnModal, setShowSpawnModal] = useState(false);

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
    const selected = workspaces.find(workspace => workspace.id === selectedWorkspaceId);
    const taskId = selected?.owner_task_id;
    if (!taskId) { onShowToast?.('Select a candidate worktree belonging to the Task you want to compare.'); return; }
    setCandidateComparisons([]);
    try {
      setComparing(true);
      setShowCompareModal(true);
      const res = await daemonClient.compareCandidates({
        task_id: taskId,
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
    setIntegratingId(winnerWsId);
    try {
      const preview = await daemonClient.integrateCandidate({ winner_workspace_id: winnerWsId, target_branch: winner.lineage?.target_branch || undefined, preview: true });
      if (preview.preview) setIntegrationPreview(preview);
      else onShowToast?.(preview.merge_detail);
    } catch (error) { onShowToast?.(`Could not preview integration: ${String(error)}`); }
    finally { setIntegratingId(null); }
  };
  const applyIntegration = async () => {
    if (!integrationPreview) return;
    const winnerWsId = integrationPreview.winner_workspace_id;
    const winner = workspaces.find(workspace => workspace.id === winnerWsId);
    if (!winner) return;
    setIntegratingId(winnerWsId);
    try {
      onShowToast?.(`Integrating ${winner.name || winnerWsId.slice(0, 8)}...`);
      const res = await daemonClient.integrateCandidate({
        winner_workspace_id: winnerWsId,
        target_branch: integrationPreview.target_branch,
        expected_target_head: integrationPreview.current_target_head,
        expected_winner_head: integrationPreview.winner_head,
      });
      setIntegrationPreview(null);

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
  const handleSpawnCandidates = async (taskId: string, count: number) => {
    const trimmedTaskId = taskId.trim();
    if (!tasks.some((task) => task.id === trimmedTaskId)) {
      onShowToast?.('Choose an existing Task before creating candidate worktrees.');
      return;
    }
    const baseWs = workspaces.find((w) => w.id === selectedWorkspaceId) || workspaces[0];
    if (!baseWs) {
      onShowToast?.('No base workspace available to spawn from.');
      return;
    }

    try {
      onShowToast?.(`Creating ${count} candidate worktrees for Task ${trimmedTaskId}...`);
      const signature = JSON.stringify([trimmedTaskId, baseWs.id, count]);
      if (spawnAttempt.current?.signature !== signature) spawnAttempt.current = { signature, commandId: crypto.randomUUID() };
      const batch = await daemonClient.spawnCandidates({
        task_id: trimmedTaskId,
        command_id: spawnAttempt.current.commandId,
        base_workspace_id: baseWs.id,
        count,
      });
      onShowToast?.(batch.partial ? `Created ${batch.candidates.length} candidates; ${batch.failed.length} failed: ${batch.failed.map(item => item.error).join('; ')}` : `Created ${batch.candidates.length} worktrees for Task ${trimmedTaskId}. No agent was started.`);
      setShowSpawnModal(false);
      await loadWorkspaces();
    } catch (err: any) {
      onShowToast?.(`Failed to spawn candidates: ${err?.message || err}`);
      throw err;
    }
  };

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {integrationPreview && <div role="dialog" aria-modal="true" aria-label="Review integration" className="absolute inset-0 z-50 bg-canvas/95 p-6 flex flex-col gap-4 overflow-auto">
        <h3 className="text-sm font-semibold">Review integration</h3>
        <p className="text-xs text-fg-muted">Merge candidate into target checkout. Pre-checked before applying; conflicts stay in target.</p>
        <dl className="text-xs space-y-2 break-all"><dt>Candidate</dt><dd>{integrationPreview.winner_branch}</dd><dt>Candidate commit</dt><dd className="font-mono">{integrationPreview.winner_head}</dd><dt>Target</dt><dd>{integrationPreview.target_branch}</dd><dt>Target commit</dt><dd className="font-mono">{integrationPreview.current_target_head}</dd></dl>
        <div className="flex gap-2"><button type="button" autoFocus className="canvas-open-button" disabled={!!integratingId} onClick={() => setIntegrationPreview(null)}>Cancel</button><button type="button" className="canvas-open-button" disabled={!!integratingId} onClick={() => void applyIntegration()}>{integratingId ? 'Integrating…' : 'Integrate reviewed commits'}</button></div>
      </div>}
      {/* Header */}
      <div className="flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-3 bg-[var(--color-surface-1)]/50">
        <div className="flex items-center gap-2">
          <GitBranch className="h-4 w-4 text-white/70 shrink-0" />
          <span className="text-xs font-semibold text-white">Git Worktrees</span>
          <span className="rounded bg-white/5 border border-white/10 px-1.5 py-0.5 text-[10px] font-mono text-white/60">
            {workspaces.length} active
          </span>
        </div>

        <div className="flex items-center gap-2">
          {workspaces.length >= 2 && (
            <button
              type="button"
              onClick={handleCompareAll}
              className="flex items-center gap-1.5 rounded-md border border-white/15 bg-white/5 px-2.5 py-1 text-xs text-white/80 hover:bg-white/10 hover:text-white transition font-medium"
              title="Compare candidates side-by-side"
            >
              <GitCompare className="h-3 w-3" />
              <span>Compare</span>
            </button>
          )}

          <button
            type="button"
            onClick={() => setShowSpawnModal(true)}
            className="flex items-center gap-1.5 rounded-md bg-white text-black px-2.5 py-1 text-xs font-semibold hover:bg-white/90 transition shadow-sm"
            title="Spawn new parallel candidate worktrees"
          >
            <Plus className="h-3 w-3 stroke-[2.5]" />
            <span>Spawn Candidates</span>
          </button>

          <button
            type="button"
            onClick={loadWorkspaces}
            className="flex items-center gap-1.5 rounded-md border border-white/10 bg-white/5 px-2 py-1 text-xs text-white/60 hover:bg-white/10 hover:text-white transition"
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
          <div className="rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-8 text-center text-xs space-y-3">
            <GitBranch className="h-6 w-6 text-white/20 mx-auto" />
            <p className="text-white/80 font-medium">No active worktrees</p>
            <button
              type="button"
              onClick={() => setShowSpawnModal(true)}
              className="inline-flex items-center gap-1.5 rounded-md bg-white text-black px-3 py-1.5 text-xs font-semibold hover:bg-white/90 shadow-sm transition"
            >
              <Plus className="h-3.5 w-3.5 stroke-[2.5]" />
              <span>Spawn Worktrees</span>
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
                    ? 'border-white/30 bg-[var(--color-surface-2)]/60 ring-1 ring-white/20'
                    : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] hover:border-[var(--color-border-default)]'
                }`}
              >
                {/* Top Row: Path, Branch, and Agent Badge */}
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <Folder className="h-4 w-4 text-white/60 shrink-0" />
                      <span className="font-semibold text-xs truncate">
                        {ws.name || ws.path.split('/').pop() || ws.id}
                      </span>
                      <span className="flex items-center gap-1 rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] font-mono text-white/70 border border-white/10">
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
                          ? 'bg-white/10 text-white border-white/20'
                          : agentStatus === 'review_needed'
                          ? 'bg-white/[0.06] text-white/80 border-white/15'
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
                            <FileDiff className="h-3.5 w-3.5 text-white/60" />
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
                            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-white/80 hover:bg-[var(--color-surface-2)]"
                          >
                            <RefreshCw className="h-3.5 w-3.5" />
                            <span>Recover Workspace State</span>
                          </button>
                          <button
                            type="button"
                            disabled={integratingId === ws.id}
                            onClick={() => handleIntegrateWorkspace(ws.id)}
                            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-white hover:bg-[var(--color-surface-2)] disabled:opacity-40"
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
                        isDirty ? 'bg-white/40' : 'bg-white/90'
                      }`}
                    />
                    {isDirty ? `${dirtyCount} uncommitted changes` : 'Clean tree'}
                  </span>

                  <button
                    type="button"
                    disabled={integratingId === ws.id}
                    onClick={() => handleIntegrateWorkspace(ws.id)}
                    className="flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] font-medium bg-white/10 hover:bg-white/20 text-white border border-white/20 disabled:opacity-40 transition"
                  >
                    {integratingId === ws.id ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : (
                      <GitMerge className="h-3 w-3" />
                    )}
                    <span>Integrate</span>
                  </button>
                </div>

                <div className="mt-2.5 pt-2 border-t border-white/5 flex items-center justify-between">
                  <button
                    type="button"
                    onClick={() => onOpenWorkspaceResource?.('agents', ws.id)}
                    className="inline-flex items-center gap-1.5 rounded-md border border-white/15 bg-white/5 px-2.5 py-1 text-xs font-medium text-white/90 hover:bg-white/10 hover:text-white transition"
                  >
                    <Bot className="h-3.5 w-3.5" />
                    <span>Start Agent</span>
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* Compare Candidates Modal */}
      {showCompareModal && (
        <CompareModal
          comparing={comparing}
          candidateComparisons={candidateComparisons}
          integratingId={integratingId}
          onIntegrateWorkspace={handleIntegrateWorkspace}
          onClose={() => setShowCompareModal(false)}
        />
      )}

      {/* Spawn Candidates Modal */}
      {showSpawnModal && (
        <SpawnModal
          tasks={tasks}
          onSpawn={handleSpawnCandidates}
          onClose={() => setShowSpawnModal(false)}
        />
      )}
    </div>
  );
};
