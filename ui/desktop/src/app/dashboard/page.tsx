import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Activity,
  ArrowLeft,
  RefreshCw,
  Search,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useScreenContext } from '@/context/ScreenContext';
import { useAppContext } from '@/context/AppContext';
import { RunWaterfallInspector } from '@/components/workbenches/shared/run-inspector/RunWaterfallInspector';
import { DashboardKpiStrip } from './DashboardKpiStrip';
import { DashboardTaskDetail } from './DashboardTaskDetail';
import { WorkQueueCard } from './WorkQueueCard';
import type {
  SupervisionOverview,
  SupervisionRunRecord,
  SupervisionFinding,
} from '@/types/workflow_telemetry';
import type { Task } from '@/types/domain';
import type { RecordedRun, WorkItem, Bucket } from './types';

export type { RecordedRun, WorkItem, Bucket };

export function supervisionBucket(status: string): Bucket {
  const value = status.toLowerCase();
  if (value === 'blocked' || value === 'suspended') return 'attention';
  if (['pending', 'queued', 'active', 'running'].includes(value)) return 'running';
  if (value === 'completed') return 'review';
  return 'history';
}

/** Read-only aggregation, bounded independently of the number of mounted Screens. */
export async function loadSupervisionRuns(
  sessionIds: string[],
  read: (id: string) => Promise<RecordedRun[]>
) {
  const ids = [...new Set(sessionIds)];
  const rows: Array<RecordedRun & { sessionId: string }> = [];
  const failures: string[] = [];
  let cursor = 0;
  await Promise.all(
    Array.from({ length: Math.min(4, ids.length) }, async () => {
      while (cursor < ids.length) {
        const id = ids[cursor++];
        try {
          const runRows = await read(id);
          rows.push(...runRows.map((run) => ({ ...run, sessionId: id })));
        } catch {
          failures.push(id);
        }
      }
    })
  );
  return { rows, failures };
}

export const DashboardPage: React.FC = () => {
  const navigate = useNavigate();
  const app = useAppContext();
  const { updateActiveScreen } = useScreenContext();

  const [scope, setScope] = useState<'project' | 'all'>('project');
  const [workbenchFilter, setWorkbenchFilter] = useState<string>('all');
  const [bucket, setBucket] = useState<Bucket>('attention');
  const [search, setSearch] = useState('');

  const [overview, setOverview] = useState<SupervisionOverview | null>(null);
  const [telemetryRuns, setTelemetryRuns] = useState<SupervisionRunRecord[]>([]);
  const [sessionRuns, setSessionRuns] = useState<Array<RecordedRun & { sessionId: string }>>([]);
  const [allFindings, setAllFindings] = useState<SupervisionFinding[]>([]);
  const [remoteSessions, setRemoteSessions] = useState<Awaited<ReturnType<typeof daemonClient.listSessions>>>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [failures, setFailures] = useState<string[]>([]);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const [updatedAt, setUpdatedAt] = useState<number | null>(null);

  const [selectedRun, setSelectedRun] = useState<WorkItem | null>(null);
  const [selectedTask, setSelectedTask] = useState<Task | null>(null);

  const inFlight = useRef(false);
  const mounted = useRef(false);

  const refresh = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    setLoading(true);
    try {
      const [sessions, overviewData, runsData, findingsData, tasksData] = await Promise.all([
        daemonClient.listSessions().catch(() => []),
        daemonClient.getSupervisionOverview().catch(() => null),
        daemonClient.listSupervisionRuns({ limit: 200 }).catch(() => ({ runs: [] })),
        daemonClient.listSupervisionFindings({ unresolved_only: true, limit: 200 }).catch(() => ({ findings: [] })),
        daemonClient.listTasks().catch(() => [] as Task[]),
      ]);

      const sessionIds = sessions.map((s) => s.id);
      const work = await loadSupervisionRuns(sessionIds, (id) => daemonClient.listSessionRuns(id));

      if (!mounted.current) return;
      setRemoteSessions(sessions);
      setOverview(overviewData);
      setTelemetryRuns(runsData?.runs ?? []);
      setAllFindings(findingsData?.findings ?? []);
      setSessionRuns(work.rows);
      setFailures(work.failures);
      setTasks(tasksData);
      setUpdatedAt(Date.now());
      setError('');
    } catch (cause) {
      if (mounted.current) setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      inFlight.current = false;
      if (mounted.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    const timer = window.setInterval(() => {
      if (!document.hidden) void refresh();
    }, 10000);
    return () => {
      mounted.current = false;
      window.clearInterval(timer);
    };
  }, [refresh]);

  useEffect(() => {
    setSelectedRun(null);
    setSelectedTask(null);
  }, [scope, app.currentProject]);

  const indexed = useMemo(
    () =>
      Object.entries(app.projectData).flatMap(([project, sessions]) =>
        sessions.map((session) => ({ project, session }))
      ),
    [app.projectData]
  );

  const visibleSessionIds = useMemo(
    () => new Set(indexed.filter((entry) => entry.project === app.currentProject).map((e) => e.session.id)),
    [indexed, app.currentProject]
  );

  const findingsByRun = useMemo(() => {
    const map = new Map<string, SupervisionFinding[]>();
    for (const f of allFindings) {
      const list = map.get(f.run_id) ?? [];
      list.push(f);
      map.set(f.run_id, list);
    }
    return map;
  }, [allFindings]);

  const items: WorkItem[] = useMemo(() => {
    const map = new Map<string, WorkItem>();
    for (const run of sessionRuns) {
      if (scope === 'project' && !visibleSessionIds.has(run.sessionId)) continue;
      const local = indexed.find((entry) => entry.session.id === run.sessionId);
      const remote = remoteSessions.find((s) => s.id === run.sessionId);
      const taskId =
        local?.session.taskId ||
        remote?.attached_to ||
        remote?.task_id ||
        (typeof remote?.status === 'object' && remote.status && 'promoted' in remote.status
          ? remote.status.promoted.task_id
          : undefined);

      map.set(run.run_id, {
        ...run,
        taskId,
        title: local?.session.title || remote?.title || run.sessionId,
        project: local?.project || app.currentProject,
        lens: local?.session.pack,
        workspaceId: local?.session.workspaceId,
        findings: findingsByRun.get(run.run_id) ?? [],
      });
    }

    for (const tr of telemetryRuns) {
      const existing = map.get(tr.run_id);
      if (existing) {
        existing.duration_ms = tr.duration_ms ?? existing.duration_ms;
        existing.tokens = tr.tokens ?? existing.tokens;
        existing.cost_usd = tr.cost_usd ?? existing.cost_usd;
        existing.workbench_id = tr.workbench_id ?? existing.workbench_id;
        if (!existing.findings || existing.findings.length === 0) {
          existing.findings = findingsByRun.get(tr.run_id) ?? [];
        }
      } else if (scope === 'all') {
        map.set(tr.run_id, {
          run_id: tr.run_id,
          status: tr.status,
          model: tr.agent,
          duration_ms: tr.duration_ms,
          tokens: tr.tokens,
          cost_usd: tr.cost_usd,
          workbench_id: tr.workbench_id,
          title: `Run ${tr.run_id.slice(0, 8)}`,
          project: app.currentProject,
          findings: findingsByRun.get(tr.run_id) ?? [],
        });
      }
    }

    return Array.from(map.values()).sort((a, b) =>
      (b.journal_occurred_at || '').localeCompare(a.journal_occurred_at || '')
    );
  }, [
    sessionRuns,
    telemetryRuns,
    findingsByRun,
    scope,
    visibleSessionIds,
    indexed,
    remoteSessions,
    app.currentProject,
  ]);

  const taskIds = useMemo(() => new Set(items.map((i) => i.taskId).filter(Boolean)), [items]);
  const blockedTasks = useMemo(() => {
    return tasks.filter(
      (task) =>
        task.status.toLowerCase() === 'blocked' &&
        (scope === 'all' || taskIds.has(task.id)) &&
        !items.some((item) => item.taskId === task.id && supervisionBucket(item.status) === 'attention')
    );
  }, [tasks, scope, taskIds, items]);

  const counts = useMemo(
    () => ({
      attention:
        items.filter((item) => supervisionBucket(item.status) === 'attention' || (item.findings && item.findings.length > 0)).length +
        blockedTasks.length,
      running: items.filter((item) => supervisionBucket(item.status) === 'running').length,
      review: items.filter((item) => supervisionBucket(item.status) === 'review').length,
      history: items.filter((item) => supervisionBucket(item.status) === 'history').length,
    }),
    [items, blockedTasks]
  );

  const filteredItems = useMemo(() => {
    return items.filter((item) => {
      if (workbenchFilter !== 'all') {
        const wb = (item.lens || item.workbench_id || '').toLowerCase();
        if (!wb.includes(workbenchFilter.toLowerCase())) return false;
      }
      const inBucket =
        bucket === 'attention'
          ? supervisionBucket(item.status) === 'attention' || (item.findings && item.findings.length > 0)
          : supervisionBucket(item.status) === bucket;
      if (!inBucket) return false;
      if (!search.trim()) return true;
      const q = search.toLowerCase();
      return (
        item.title.toLowerCase().includes(q) ||
        item.run_id.toLowerCase().includes(q) ||
        (item.model || '').toLowerCase().includes(q) ||
        (item.project || '').toLowerCase().includes(q)
      );
    });
  }, [items, workbenchFilter, bucket, search]);

  const revealSession = (sessionId: string, taskId?: string) => {
    const local = indexed.find((entry) => entry.session.id === sessionId);
    if (local?.project) app.setCurrentProject(local.project);
    const homeWorkbench = ['engineering', 'coding', 'code'].includes(local?.session.pack || '')
      ? 'engineering'
      : local?.session.pack === 'research'
      ? 'research'
      : 'copilot';

    updateActiveScreen({
      activeSessionId: sessionId,
      selectedTaskId: taskId || null,
      sidebarView: 'chats',
      homeWorkbench,
    });
    app.setActiveWorkbench(homeWorkbench);
    app.setActiveSessionId(sessionId);
    navigate(`/studio/${encodeURIComponent(sessionId)}?view=chats`);
  };

  const handleCancelRun = async (runId: string) => {
    await daemonClient.cancelRun({ run_id: runId, reason: 'Cancelled from Supervision Console' });
    await refresh();
  };

  const totalRuns = overview?.total_runs ?? items.length;
  const failedRuns = overview?.failed_runs ?? items.filter((i) => i.status === 'failed' || i.status === 'error').length;
  const successRate = totalRuns > 0 ? Math.round(((totalRuns - failedRuns) / totalRuns) * 100) : 100;
  const openFindingsCount = overview?.open_findings ?? allFindings.length;
  const criticalFindingsCount = overview?.critical_findings ?? allFindings.filter((f) => f.severity === 'critical').length;
  const currentWorkspace = app.workspaces.find((w) => w.id === selectedRun?.workspaceId);

  return (
    <main className="flex h-full min-h-0 flex-col bg-[#101113] text-[#f4f4f5]">
      {/* 1. Header Toolbar */}
      <header className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-white/10 bg-[#141518] px-4 py-2.5">
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => navigate('/studio')}
            aria-label="Back to workspace"
            className="rounded-lg border border-white/10 bg-white/[0.04] p-1.5 text-white/70 hover:bg-white/[0.08] hover:text-white transition"
          >
            <ArrowLeft size={16} />
          </button>
          <div className="flex items-center gap-2">
            <Activity size={18} className="text-sky-400" />
            <div>
              <h1 className="text-xs font-semibold tracking-wide text-white">Supervision Console</h1>
              <p className="text-[10px] text-white/50">
                Hierarchical waterfalls, anomaly watchdogs & multi-workbench governance
              </p>
            </div>
          </div>
        </div>

        {/* Global Controls */}
        <div className="flex items-center gap-2">
          <select
            aria-label="Supervision scope"
            value={scope}
            onChange={(e) => setScope(e.target.value as 'project' | 'all')}
            className="rounded-lg border border-white/10 bg-[#101113] px-2.5 py-1 text-xs text-white/80 focus:border-sky-500 focus:outline-hidden"
          >
            <option value="project">Project: {app.currentProject}</option>
            <option value="all">All Projects & Fleet</option>
          </select>

          <select
            aria-label="Filter workbench"
            value={workbenchFilter}
            onChange={(e) => setWorkbenchFilter(e.target.value)}
            className="rounded-lg border border-white/10 bg-[#101113] px-2.5 py-1 text-xs text-white/80 focus:border-sky-500 focus:outline-hidden"
          >
            <option value="all">All Workbenches</option>
            <option value="coding">Coding</option>
            <option value="research">Research</option>
            <option value="copilot">Copilot / Assistant</option>
            <option value="quant">Quant</option>
          </select>

          <button
            type="button"
            onClick={() => void refresh()}
            disabled={loading}
            className="inline-flex items-center gap-1.5 rounded-lg border border-white/10 bg-white/[0.04] px-2.5 py-1 text-xs text-white/80 hover:bg-white/[0.08] hover:text-white transition disabled:opacity-50"
          >
            <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
            <span>Refresh</span>
          </button>
        </div>
      </header>

      {/* 2. Top KPI Metric Strip */}
      <DashboardKpiStrip
        overview={overview}
        counts={counts}
        totalRuns={totalRuns}
        successRate={successRate}
        openFindingsCount={openFindingsCount}
        criticalFindingsCount={criticalFindingsCount}
        updatedAt={updatedAt}
      />

      {/* 3. Filter Buckets & Search Strip */}
      <div className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b border-white/10 bg-[#121316] px-4 py-2 text-xs">
        <div className="flex items-center gap-1.5">
          {(
            [
              ['attention', 'Needs you', counts.attention],
              ['running', 'Running', counts.running],
              ['review', 'Results to review', counts.review],
              ['history', 'History', counts.history],
            ] as const
          ).map(([id, label, count]) => (
            <button
              key={id}
              type="button"
              aria-pressed={bucket === id}
              onClick={() => setBucket(id)}
              className={`rounded-lg border px-3 py-1.5 text-xs font-medium transition ${
                bucket === id
                  ? 'border-sky-500/50 bg-sky-500/10 text-white shadow-xs'
                  : 'border-white/8 bg-white/[0.02] text-white/60 hover:bg-white/[0.06] hover:text-white'
              }`}
            >
              <span>{label}</span>
              <span className="ml-1.5 rounded-full bg-white/10 px-1.5 py-0.2 font-mono text-[10px] text-white/80">
                {count}
              </span>
            </button>
          ))}
        </div>

        <div className="relative min-w-[220px]">
          <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-white/40" />
          <input
            aria-label="Search supervised work"
            placeholder="Filter runs, tasks, models…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full rounded-lg border border-white/10 bg-[#101113] py-1.5 pl-8 pr-3 text-xs text-white placeholder-white/30 focus:border-sky-500 focus:outline-hidden"
          />
        </div>
      </div>

      {/* 4. Notification Alerts */}
      {error && (
        <div role="alert" className="border-b border-rose-500/30 bg-rose-500/10 px-4 py-1.5 text-xs text-rose-300">
          {error} · telemetry may be degraded.
        </div>
      )}
      {failures.length > 0 && (
        <div role="alert" className="border-b border-amber-500/30 bg-amber-500/10 px-4 py-1.5 text-xs text-amber-300">
          Partial coverage: could not read runs for {failures.length} session(s).
        </div>
      )}

      {/* 5. Master-Detail Split: Left Queue, Right Waterfall */}
      <div className="grid min-h-0 flex-1 grid-cols-1 overflow-hidden lg:grid-cols-[minmax(320px,380px)_minmax(0,1fr)]">
        {/* Left Master List */}
        <section
          aria-label="Supervised work and runs"
          className="flex min-h-0 flex-col overflow-y-auto border-r border-white/10 bg-[#101113] p-3 space-y-2"
        >
          {bucket === 'attention' &&
            blockedTasks
              .filter((t) => t.title.toLowerCase().includes(search.toLowerCase()))
              .map((t) => (
                <button
                  type="button"
                  key={`task-${t.id}`}
                  onClick={() => {
                    setSelectedRun(null);
                    setSelectedTask(t);
                  }}
                  className={`block w-full rounded-lg border p-3 text-left transition ${
                    selectedTask?.id === t.id && !selectedRun
                      ? 'border-amber-400 bg-amber-500/10'
                      : 'border-amber-500/30 bg-[#141518] hover:bg-[#18191d]'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="font-semibold text-xs text-white truncate">{t.title}</span>
                    <span className="rounded bg-amber-500/20 px-1.5 py-0.5 font-mono text-[9px] uppercase font-bold text-amber-300">
                      Blocked Task
                    </span>
                  </div>
                  <span className="mt-1 block text-[11px] text-white/50">
                    Task {t.id.slice(0, 8)} · needs inspection
                  </span>
                </button>
              ))}

          {filteredItems.map((item) => (
            <WorkQueueCard
              key={`${item.sessionId || 'nosess'}:${item.run_id}`}
              item={item}
              isSelected={selectedRun?.run_id === item.run_id}
              currentProject={app.currentProject}
              workspaceName={app.workspaces.find((w) => w.id === item.workspaceId)?.name}
              onSelect={() => {
                setSelectedTask(null);
                setSelectedRun(item);
              }}
            />
          ))}

          {!loading && !error && filteredItems.length === 0 && (bucket !== 'attention' || blockedTasks.length === 0) && (
            <div className="p-6 text-center text-xs text-white/40">
              {failures.length
                ? 'No matching work in the sessions successfully read.'
                : 'No recorded work in this view.'}
            </div>
          )}
        </section>

        {/* Right Detail View */}
        <section aria-label="Work inspector" className="flex min-h-[400px] min-w-0 flex-col overflow-hidden bg-[#101113]">
          {selectedRun ? (
            <RunWaterfallInspector
              key={selectedRun.run_id}
              runId={selectedRun.run_id}
              sessionId={selectedRun.sessionId}
              taskId={selectedRun.taskId}
              projectName={selectedRun.project}
              workspaceName={currentWorkspace?.name || selectedRun.workspaceId}
              worktreeBranch={currentWorkspace?.branch || 'main'}
              onRevealSession={revealSession}
              onCancelRun={handleCancelRun}
              onClose={() => setSelectedRun(null)}
            />
          ) : selectedTask ? (
            <DashboardTaskDetail
              task={selectedTask}
              onOpenTask={(taskId) => {
                const owner = indexed.find((entry) => entry.session.taskId === taskId);
                if (owner) app.setCurrentProject(owner.project);
                updateActiveScreen({ selectedTaskId: taskId, sidebarView: 'tasks' });
                navigate(`/studio?view=tasks&task=${encodeURIComponent(taskId)}`);
              }}
            />
          ) : (
            <div className="m-auto max-w-md p-8 text-center text-xs text-white/50">
              <Activity className="mx-auto mb-3 h-8 w-8 text-white/20" />
              <h2 className="text-sm font-semibold text-white">Select Work to Inspect</h2>
              <p className="mt-2 leading-relaxed">
                Select any run or task on the left queue to inspect its complete hierarchical execution
                waterfall, latency breakdown, deterministic watchdog anomalies, and correlated logs.
              </p>
            </div>
          )}
        </section>
      </div>
    </main>
  );
};

export default DashboardPage;
