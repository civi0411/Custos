import React, { useEffect, useState, useCallback, useRef } from 'react';
import {
  GitBranch,
  RefreshCw,
  GitCompare,
  Plus,
  Loader2,
  Trophy,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import type { ExecutionWorkspace, DirtyManifest, Task } from '@/types/domain';
import type { WorktreeManagerPaneProps, CandidateComparison } from './types';
import { WorktreeCard } from './WorktreeCard';
import { CompareCandidatesModal } from './CompareCandidatesModal';
import { SpawnCandidatesModal } from './SpawnCandidatesModal';
import { RaceVerdictBanner } from './RaceVerdictBanner';

export const WorktreeManagerView: React.FC<WorktreeManagerPaneProps> = ({
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
  const [spawnTaskId, setSpawnTaskId] = useState('');
  const [spawnCount, setSpawnCount] = useState(2);
  const [spawning, setSpawning] = useState(false);

  // Candidate Race state
  const [racing, setRacing] = useState(false);
  const [raceResult, setRaceResult] = useState<Awaited<ReturnType<typeof daemonClient.runCandidateRace>> | null>(null);

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

  const handleCompareAll = async () => {
    const selected = workspaces.find((w) => w.id === selectedWorkspaceId);
    const taskId = selected?.owner_task_id || spawnTaskId;
    if (!taskId) {
      onShowToast?.('Select a candidate worktree belonging to the Task you want to compare.');
      return;
    }
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

  const handleIntegrateWorkspace = async (winnerWsId: string) => {
    setShowSecondaryMenu(null);
    const winner = workspaces.find((w) => w.id === winnerWsId);
    if (!winner) {
      onShowToast?.('Cannot integrate: the selected worktree is no longer available. Refresh the list and compare again.');
      return;
    }
    setIntegratingId(winnerWsId);
    try {
      const preview = await daemonClient.integrateCandidate({
        winner_workspace_id: winnerWsId,
        target_branch: winner.lineage?.target_branch || undefined,
        preview: true,
      });
      if (preview.preview) setIntegrationPreview(preview);
      else onShowToast?.(preview.merge_detail);
    } catch (error) {
      onShowToast?.(`Could not preview integration: ${String(error)}`);
    } finally {
      setIntegratingId(null);
    }
  };

  const applyIntegration = async () => {
    if (!integrationPreview) return;
    const winnerWsId = integrationPreview.winner_workspace_id;
    const winner = workspaces.find((w) => w.id === winnerWsId);
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
        onShowToast?.(`Successfully integrated candidate branch ${res.winner_branch} into ${res.target_branch}!`);
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
      const signature = JSON.stringify([taskId, baseWs.id, spawnCount]);
      if (spawnAttempt.current?.signature !== signature) {
        spawnAttempt.current = { signature, commandId: crypto.randomUUID() };
      }
      const batch = await daemonClient.spawnCandidates({
        task_id: taskId,
        command_id: spawnAttempt.current.commandId,
        base_workspace_id: baseWs.id,
        count: spawnCount,
      });
      onShowToast?.(
        batch.partial
          ? `Created ${batch.candidates.length} candidates; ${batch.failed.length} failed: ${batch.failed.map((item) => item.error).join('; ')}`
          : `Created ${batch.candidates.length} worktrees for Task ${taskId}. No agent was started.`
      );
      setShowSpawnModal(false);
      setSpawnTaskId('');
      await loadWorkspaces();
    } catch (err: any) {
      onShowToast?.(`Failed to spawn candidates: ${err?.message || err}`);
    } finally {
      setSpawning(false);
    }
  };

  const handleRunCandidateRace = async () => {
    const candidateWs = workspaces.filter((w) => w.owner_task_id);
    if (candidateWs.length === 0) {
      onShowToast?.('No task-owned candidate worktrees found to run race.');
      return;
    }
    const taskId = candidateWs[0].owner_task_id!;
    setRacing(true);
    try {
      onShowToast?.(`Starting candidate race across ${candidateWs.length} models for Task ${taskId}...`);
      const res = await daemonClient.runCandidateRace({
        task_id: taskId,
        test_command: 'cargo test --workspace',
      });
      setRaceResult(res);
      if (res.winner_candidate_id) {
        onShowToast?.(`Candidate race finished! Winner: ${res.winner_candidate_id}`);
      } else {
        onShowToast?.('Candidate race finished without an automated winner.');
      }
      await loadWorkspaces();
    } catch (err: any) {
      onShowToast?.(`Failed to run candidate race: ${err?.message || err}`);
    } finally {
      setRacing(false);
    }
  };

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {integrationPreview && (
        <div
          role="dialog"
          aria-modal="true"
          aria-label="Review integration"
          className="absolute inset-0 z-50 bg-canvas/95 p-6 flex flex-col gap-4 overflow-auto"
        >
          <h3 className="text-sm font-semibold">Review integration</h3>
          <p className="text-xs text-fg-muted">
            Merge candidate into target checkout. Pre-checked before applying; conflicts stay in target.
          </p>
          <dl className="text-xs space-y-2 break-all">
            <dt>Candidate</dt>
            <dd>{integrationPreview.winner_branch}</dd>
            <dt>Candidate commit</dt>
            <dd className="font-mono">{integrationPreview.winner_head}</dd>
            <dt>Target</dt>
            <dd>{integrationPreview.target_branch}</dd>
            <dt>Target commit</dt>
            <dd className="font-mono">{integrationPreview.current_target_head}</dd>
          </dl>
          <div className="flex gap-2">
            <button
              type="button"
              autoFocus
              className="canvas-open-button"
              disabled={!!integratingId}
              onClick={() => setIntegrationPreview(null)}
            >
              Cancel
            </button>
            <button
              type="button"
              className="canvas-open-button"
              disabled={!!integratingId}
              onClick={() => void applyIntegration()}
            >
              {integratingId ? 'Integrating…' : 'Integrate reviewed commits'}
            </button>
          </div>
        </div>
      )}

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
          {workspaces.some((w) => w.owner_task_id) && (
            <button
              type="button"
              onClick={handleRunCandidateRace}
              disabled={racing}
              className="flex items-center gap-1.5 rounded-md border border-amber-500/30 bg-amber-500/10 px-2.5 py-1 text-xs text-amber-200 hover:bg-amber-500/20 transition font-medium disabled:opacity-50"
              title="Run multi-model candidate race with Test Oracle"
            >
              {racing ? <Loader2 className="h-3 w-3 animate-spin" /> : <Trophy className="h-3 w-3 text-amber-400" />}
              <span>{racing ? 'Racing…' : 'Run Race'}</span>
            </button>
          )}

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
        <RaceVerdictBanner raceResult={raceResult} onDismiss={() => setRaceResult(null)} />

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
          workspaces.map((ws) => (
            <WorktreeCard
              key={ws.id}
              workspace={ws}
              isSelected={ws.id === selectedWorkspaceId}
              manifest={dirtyManifests[ws.id]}
              showSecondaryMenu={showSecondaryMenu === ws.id}
              integratingId={integratingId}
              onSelect={() => {
                setSelectedWorkspaceId(ws.id);
                onSelectWorkspace?.(ws.id);
              }}
              onToggleSecondaryMenu={() => setShowSecondaryMenu(showSecondaryMenu === ws.id ? null : ws.id)}
              onCloseSecondaryMenu={() => setShowSecondaryMenu(null)}
              onViewDiff={() => handleViewDiff(ws)}
              onOpenTerminal={() => handleOpenTerminal(ws)}
              onRecover={() => handleRecoverWorkspace(ws)}
              onIntegrate={() => handleIntegrateWorkspace(ws.id)}
              onStartAgent={() => onOpenWorkspaceResource?.('agents', ws.id)}
            />
          ))
        )}
      </div>

      <CompareCandidatesModal
        isOpen={showCompareModal}
        comparing={comparing}
        candidateComparisons={candidateComparisons}
        integratingId={integratingId}
        onClose={() => setShowCompareModal(false)}
        onIntegrate={handleIntegrateWorkspace}
      />

      <SpawnCandidatesModal
        isOpen={showSpawnModal}
        spawning={spawning}
        tasks={tasks}
        spawnTaskId={spawnTaskId}
        spawnCount={spawnCount}
        onClose={() => setShowSpawnModal(false)}
        onTaskIdChange={setSpawnTaskId}
        onSpawnCountChange={setSpawnCount}
        onSubmit={handleSpawnSubmit}
      />
    </div>
  );
};
