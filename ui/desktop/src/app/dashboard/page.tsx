import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Activity, ArrowLeft, ExternalLink, RefreshCw } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useScreenContext } from '@/context/ScreenContext';
import { useAppContext } from '@/context/AppContext';
import { ActivityPane } from '@/components/workbenches/engineering/ActivityPane';
import type { Task } from '@/types/domain';

type RecordedRun = { run_id: string; status: string; model?: string; journal_occurred_at?: string };
type WorkItem = RecordedRun & { sessionId: string; taskId?: string; title: string; project?: string; lens?: string; workspaceId?: string };
type Bucket = 'attention' | 'running' | 'review' | 'history';
export function supervisionBucket(status: string): Bucket {
  const value = status.toLowerCase();
  if (value === 'blocked' || value === 'suspended') return 'attention';
  if (['pending', 'queued', 'active', 'running'].includes(value)) return 'running';
  if (value === 'completed') return 'review';
  return 'history';
}

/** Read-only aggregation, bounded independently of the number of mounted Screens. */
export async function loadSupervisionRuns(sessionIds: string[], read: (id: string) => Promise<RecordedRun[]>) {
  const ids = [...new Set(sessionIds)];
  const rows: Array<RecordedRun & { sessionId: string }> = [];
  const failures: string[] = [];
  let cursor = 0;
  await Promise.all(Array.from({ length: Math.min(4, ids.length) }, async () => {
    while (cursor < ids.length) {
      const id = ids[cursor++];
      try { rows.push(...(await read(id)).map(run => ({ ...run, sessionId: id }))); }
      catch { failures.push(id); }
    }
  }));
  return { rows, failures };
}

export const DashboardPage: React.FC = () => {
  const navigate = useNavigate();
  const app = useAppContext();
  const { updateActiveScreen } = useScreenContext();
  const [scope, setScope] = useState('project');
  const [bucket, setBucket] = useState<Bucket>('attention');
  const [search, setSearch] = useState('');
  const [runs, setRuns] = useState<Array<RecordedRun & { sessionId: string }>>([]);
  const [remoteSessions, setRemoteSessions] = useState<Awaited<ReturnType<typeof daemonClient.listSessions>>>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [failures, setFailures] = useState<string[]>([]);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const [updatedAt, setUpdatedAt] = useState<number | null>(null);
  const [selected, setSelected] = useState<{ sessionId: string; runId: string } | null>(null);
  const [selectedTask, setSelectedTask] = useState<string | null>(null);
  const inFlight = useRef(false);
  const mounted = useRef(false);
  const refresh = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    setLoading(true);
    try {
      const sessions = await daemonClient.listSessions();
      const [work, taskResult] = await Promise.all([
        loadSupervisionRuns(sessions.map(session => session.id), id => daemonClient.listSessionRuns(id)),
        daemonClient.listTasks().then(value => ({ value, error: '' }), () => ({ value: [] as Task[], error: 'Task attention could not be loaded.' })),
      ]);
      if (!mounted.current) return;
      setRemoteSessions(sessions); setRuns(work.rows); setTasks(taskResult.value);
      setFailures(work.failures); setError(taskResult.error); setUpdatedAt(Date.now());
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
    const timer = window.setInterval(() => { if (!document.hidden) void refresh(); }, 15000);
    return () => { mounted.current = false; window.clearInterval(timer); };
  }, [refresh]);
  useEffect(() => { setSelected(null); setSelectedTask(null); }, [scope, app.currentProject]);
  const indexed = useMemo(() => Object.entries(app.projectData).flatMap(([project, sessions]) => sessions.map(session => ({ project, session }))), [app.projectData]);
  const visibleSessionIds = new Set(indexed.filter(entry => entry.project === app.currentProject).map(entry => entry.session.id));
  const items: WorkItem[] = runs.filter(run => scope === 'all' || visibleSessionIds.has(run.sessionId)).map(run => {
    const local = indexed.find(entry => entry.session.id === run.sessionId);
    const remote = remoteSessions.find(session => session.id === run.sessionId);
    return { ...run, taskId: local?.session.taskId || remote?.attached_to || remote?.task_id || (typeof remote?.status === 'object' && remote.status && 'promoted' in remote.status ? remote.status.promoted.task_id : undefined),
      title: local?.session.title || remote?.title || run.sessionId, project: local?.project,
      lens: local?.session.pack, workspaceId: local?.session.workspaceId };
  }).sort((a, b) => (b.journal_occurred_at || '').localeCompare(a.journal_occurred_at || ''));
  const taskIds = new Set(items.map(item => item.taskId).filter(Boolean));
  // Include blocked tasks without runs when they have an actual loaded session binding.
  indexed.filter(entry => scope === 'all' || entry.project === app.currentProject).forEach(entry => { if (entry.session.taskId) taskIds.add(entry.session.taskId); });
  const blocked = tasks.filter(task => task.status.toLowerCase() === 'blocked' && (scope === 'all' || taskIds.has(task.id)) && !items.some(item => item.taskId === task.id && supervisionBucket(item.status) === 'attention'));
  const counts = { attention: items.filter(item => supervisionBucket(item.status) === 'attention').length + blocked.length,
    running: items.filter(item => supervisionBucket(item.status) === 'running').length,
    review: items.filter(item => supervisionBucket(item.status) === 'review').length,
    history: items.filter(item => supervisionBucket(item.status) === 'history').length };
  const visible = items.filter(item => supervisionBucket(item.status) === bucket && `${item.title} ${item.run_id} ${item.model || ''} ${item.project || ''}`.toLowerCase().includes(search.toLowerCase()));
  const inspected = selected && items.find(item => item.sessionId === selected.sessionId && item.run_id === selected.runId);
  const task = tasks.find(item => item.id === selectedTask);
  const reveal = (item: WorkItem) => {
    if (item.project) app.setCurrentProject(item.project);
    const homeWorkbench = ['engineering', 'coding', 'code'].includes(item.lens || '') ? 'engineering' : item.lens === 'research' ? 'research' : 'copilot';
    updateActiveScreen({ activeSessionId: item.sessionId, selectedTaskId: item.taskId || null, sidebarView: 'chats', homeWorkbench });
    app.setActiveWorkbench(homeWorkbench);
    app.setActiveSessionId(item.sessionId);
    navigate(`/studio/${encodeURIComponent(item.sessionId)}?view=chats`);
  };
  return <main className="flex h-full min-h-0 flex-col bg-canvas text-fg-editor">
    <header className="flex shrink-0 flex-wrap items-center gap-3 border-b border-border-muted px-4 py-3">
      <button type="button" onClick={() => navigate('/studio')} aria-label="Back to workspace" className="rounded p-2 hover:bg-surface-2"><ArrowLeft size={16} /></button>
      <Activity size={18} /><div className="mr-auto"><h1 className="text-sm font-semibold">Supervision</h1><p className="text-xs text-fg-muted">Work across Coding, Research and Copilot</p></div>
      <select aria-label="Supervision scope" value={scope} onChange={event => setScope(event.target.value)} className="rounded border border-border-default bg-surface-1 px-2 py-1 text-xs"><option value="project">{app.currentProject}</option><option value="all">All sessions / unfiled work</option></select>
      <button type="button" onClick={() => void refresh()} disabled={loading} className="flex items-center gap-2 rounded border border-border-default px-3 py-1.5 text-xs disabled:opacity-50"><RefreshCw size={13} className={loading ? 'animate-spin' : ''} />Refresh</button>
    </header>
    <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-border-muted p-3">
      {([['attention', 'Needs you'], ['running', 'Running'], ['review', 'Results to review'], ['history', 'History']] as const).map(([id, label]) => <button key={id} type="button" aria-pressed={bucket === id} onClick={() => setBucket(id)} className={`rounded-lg border px-3 py-2 text-xs ${bucket === id ? 'border-[var(--color-border-focus)] bg-surface-2' : 'border-border-muted hover:bg-surface-1'}`}>{label} <span className="ml-2 tabular-nums text-fg-muted">{counts[id]}</span></button>)}
      <input aria-label="Search supervised work" placeholder="Search work or run ID" value={search} onChange={event => setSearch(event.target.value)} className="ml-auto rounded border border-border-default bg-surface-1 px-3 py-2 text-xs" />
    </div>
    <div className="shrink-0 space-y-1 border-b border-border-muted px-4 py-2 text-[11px] text-fg-muted" aria-live="polite">
      <p>{loading ? 'Refreshing recorded work…' : updatedAt ? `Last successful refresh ${new Date(updatedAt).toLocaleTimeString()}.` : 'No successful refresh yet.'} Run completion does not establish Task acceptance.</p>
      {error && <p role="alert" className="text-rose-500">{error} {updatedAt ? 'Previously loaded data may be stale.' : 'Work status is unavailable.'}</p>}
      {failures.length > 0 && <p role="alert" className="text-amber-500">Partial coverage: could not read runs for {failures.length} session(s). Counts exclude those sessions.</p>}
      {bucket === 'attention' && <p>Blocked Tasks and suspended runs are shown here. A global approval/effect inbox is not available from this daemon view.</p>}
    </div>
    <div className="grid min-h-0 flex-1 grid-cols-1 overflow-auto lg:grid-cols-[minmax(260px,340px)_minmax(0,1fr)] lg:overflow-hidden">
      <section aria-label="Supervised work" className="space-y-2 overflow-y-auto border-r border-border-muted p-3">
        {bucket === 'attention' && blocked.filter(item => item.title.toLowerCase().includes(search.toLowerCase())).map(item => <button type="button" key={item.id} onClick={() => { setSelected(null); setSelectedTask(item.id); }} className="block w-full rounded-lg border border-amber-500/40 bg-surface-1 p-3 text-left"><span className="block text-xs font-medium">{item.title}</span><span className="text-[11px] text-fg-muted">Blocked Task · needs inspection</span></button>)}
        {visible.map(item => <button type="button" key={`${item.sessionId}:${item.run_id}`} aria-pressed={selected?.runId === item.run_id && selected.sessionId === item.sessionId} onClick={() => { setSelectedTask(null); setSelected({ sessionId: item.sessionId, runId: item.run_id }); }} className={`block w-full rounded-lg border bg-surface-1 p-3 text-left hover:bg-surface-2 ${inspected === item ? 'border-[var(--color-border-focus)]' : 'border-border-muted'}`}>
          <span className="block truncate text-xs font-medium">{item.title}</span><span className="mt-1 block text-[11px] text-fg-muted">{item.status} · {item.model || 'Executor not reported'}</span><span className="mt-1 block truncate text-[10px] text-fg-muted">{item.project || 'Unfiled'} · {item.lens || 'Workbench not reported'}</span><span className="mt-1 block truncate font-mono text-[10px] text-fg-muted">{item.run_id}</span>
        </button>)}
        {!loading && !error && visible.length === 0 && (bucket !== 'attention' || blocked.length === 0) && <p className="p-4 text-xs text-fg-muted">{failures.length ? 'No matching work in the sessions successfully read.' : 'No recorded work in this view.'}</p>}
      </section>
      <section aria-label="Work inspector" className="flex min-h-[360px] min-w-0 flex-col overflow-hidden">
        {inspected ? <><div className="flex flex-wrap items-center gap-2 border-b border-border-muted p-3 text-xs"><span className="mr-auto text-fg-muted">{inspected.taskId ? `Task ${inspected.taskId}` : 'Task binding not reported'} · {inspected.workspaceId ? app.workspaces.find(workspace => workspace.id === inspected.workspaceId)?.name || inspected.workspaceId : 'Workspace not reported'}</span><button type="button" onClick={() => reveal(inspected)} disabled={!inspected.project} title={!inspected.project ? 'Session Project membership is not loaded. Open it from Conversations after refreshing saved work.' : undefined} className="flex items-center gap-1 rounded border border-border-default px-2 py-1 disabled:opacity-40"><ExternalLink size={12} />Reveal conversation</button></div><div className="min-h-0 flex-1"><ActivityPane key={`${inspected.sessionId}:${inspected.run_id}`} sessionId={inspected.sessionId} initialRunId={inspected.run_id} onShowToast={app.showToast} /></div></> : task ? <div className="space-y-4 overflow-auto p-6"><h2 className="text-sm font-semibold">{task.title}</h2><p className="text-xs text-fg-muted">{task.contract?.description || 'No goal description recorded.'}</p><dl className="text-xs"><dt>Status</dt><dd className="text-fg-muted">{task.status}</dd><dt className="mt-3">Task ID</dt><dd className="break-all font-mono text-fg-muted">{task.id}</dd></dl><button type="button" className="rounded border border-border-default px-3 py-2 text-xs" onClick={() => { const owner = indexed.find(entry => entry.session.taskId === task.id); if (owner) app.setCurrentProject(owner.project); updateActiveScreen({ selectedTaskId: task.id, sidebarView: 'tasks' }); navigate(`/studio?view=tasks&task=${encodeURIComponent(task.id)}`); }}>Open Task</button><h3 className="text-xs font-semibold">Acceptance criteria</h3>{task.contract?.criteria?.map((criterion, index) => <p key={index} className="text-xs text-fg-muted">{criterion.name}: {criterion.description || 'No details recorded'}</p>)}<p className="text-xs text-fg-muted">This view reports a blocked Task; no specific approval request was supplied. Inspect its conversation before deciding the next step.</p></div> : <div className="m-auto max-w-md p-8 text-center"><Activity className="mx-auto mb-4 text-fg-muted" /><h2 className="text-sm font-medium">Select work to inspect</h2><p className="mt-2 text-xs leading-5 text-fg-muted">See the actual run output and observed events. Hidden Screens do not define execution state. Approvals, acceptance and native controls require their own supported operations.</p></div>}
      </section>
    </div>
  </main>;
};
export default DashboardPage;
