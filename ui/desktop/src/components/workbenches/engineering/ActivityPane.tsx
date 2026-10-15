import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Activity, FileDiff, RefreshCw, Square, Terminal } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { LiveRunSnapshot } from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

type SessionRun = { run_id: string; model?: string; status: string; journal_occurred_at?: string };

export interface ActivityPaneProps {
  sessionId?: string;
  onShowToast?: (msg: string) => void;
  onAskAgent?: (draft: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

const activeStatus = (status: string) => ['queued', 'pending', 'active', 'running'].includes(status.toLowerCase());
const eventText = (value: unknown) => {
  if (typeof value === 'string') return value;
  try { return JSON.stringify(value, null, 2); } catch { return String(value); }
};

export const ActivityPane: React.FC<ActivityPaneProps> = ({ sessionId, onShowToast, onOpenTab }) => {
  const [runs, setRuns] = useState<SessionRun[]>([]);
  const [selectedRunId, setSelectedRunId] = useState('');
  const [snapshot, setSnapshot] = useState<LiveRunSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [stopping, setStopping] = useState(false);

  const refresh = useCallback(async () => {
    if (!sessionId) { setRuns([]); setSelectedRunId(''); setSnapshot(null); return; }
    setLoading(true);
    setError('');
    try {
      const result = await daemonClient.listSessionRuns(sessionId);
      setRuns(result);
      setSelectedRunId((previous) => previous && result.some((run) => run.run_id === previous)
        ? previous
        : result.find((run) => activeStatus(run.status))?.run_id ?? result[0]?.run_id ?? '');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally { setLoading(false); }
  }, [sessionId]);

  useEffect(() => { void refresh(); }, [refresh]);

  useEffect(() => {
    if (!sessionId || !selectedRunId) { setSnapshot(null); return; }
    const controller = new AbortController();
    setSnapshot(null);
    daemonClient.watchRun(sessionId, selectedRunId, setSnapshot, controller.signal)
      .catch((cause) => { if (!controller.signal.aborted) setError(cause instanceof Error ? cause.message : String(cause)); });
    return () => controller.abort();
  }, [sessionId, selectedRunId]);

  const selectedRun = runs.find((run) => run.run_id === selectedRunId);
  const isActive = selectedRun ? activeStatus(snapshot?.status || selectedRun.status) : false;
  const activities = useMemo(() => snapshot?.activities ?? [], [snapshot]);

  const stop = async () => {
    if (!selectedRunId) return;
    setStopping(true);
    try {
      await daemonClient.cancelRun({ run_id: selectedRunId, reason: 'Stopped by user from Activity' });
      onShowToast?.('Stop requested for the selected run.');
      await refresh();
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
    } finally { setStopping(false); }
  };

  return <div className="flex h-full min-h-0 flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
    <header className="flex h-11 shrink-0 items-center justify-between gap-3 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]/50 px-3">
      <div className="flex min-w-0 items-center gap-2"><Activity className="h-4 w-4 shrink-0 text-[var(--color-fg-muted)]" /><span className="text-xs font-semibold">Run activity</span>
        {selectedRun && <span className={`truncate text-[10px] ${isActive ? 'text-blue-400' : 'text-[var(--color-fg-subtle)]'}`}>{snapshot?.status ?? selectedRun.status}</span>}
      </div>
      <div className="flex items-center gap-1.5">
        <button type="button" onClick={() => onOpenTab?.('files')} className="flex items-center gap-1 rounded border border-[var(--color-border-subtle)] px-2 py-1 text-[11px] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]"><FileDiff className="h-3 w-3" />Files</button>
        <button type="button" onClick={() => onOpenTab?.('terminal')} className="flex items-center gap-1 rounded border border-[var(--color-border-subtle)] px-2 py-1 text-[11px] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]"><Terminal className="h-3 w-3" />Terminal</button>
        <button type="button" onClick={() => void refresh()} disabled={loading} aria-label="Refresh run activity" className="rounded border border-[var(--color-border-subtle)] p-1.5 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] disabled:opacity-50"><RefreshCw className={`h-3.5 w-3.5 ${loading ? 'animate-spin' : ''}`} /></button>
        {isActive && <button type="button" onClick={() => void stop()} disabled={stopping} className="flex items-center gap-1 rounded bg-rose-600 px-2.5 py-1 text-[11px] font-medium text-white disabled:opacity-50"><Square className="h-3 w-3 fill-current" />{stopping ? 'Stopping…' : 'Stop'}</button>}
      </div>
    </header>

    {!sessionId && <p className="p-4 text-xs text-[var(--color-fg-muted)]">Open a session to see its runs.</p>}
    {error && <p role="alert" className="border-b border-rose-500/20 px-4 py-2 text-xs text-rose-400">Could not load live activity: {error}</p>}
    {sessionId && <div className="flex min-h-0 flex-1">
      <nav aria-label="Session runs" className="w-64 shrink-0 space-y-1 overflow-y-auto border-r border-[var(--color-border-subtle)] p-2">
        {runs.length === 0 && !loading && !error && <p className="p-3 text-xs text-[var(--color-fg-subtle)]">No recorded runs for this session.</p>}
        {runs.map((run) => <button key={run.run_id} type="button" onClick={() => setSelectedRunId(run.run_id)} aria-current={selectedRunId === run.run_id ? 'true' : undefined} className={`w-full rounded-lg border p-2.5 text-left ${selectedRunId === run.run_id ? 'border-[var(--color-border-focus)] bg-[var(--color-surface-2)]' : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)]'}`}>
          <span className="block truncate font-mono text-[10px]">{run.run_id}</span><span className="mt-1 block truncate text-[10px] text-[var(--color-fg-muted)]">{run.model || 'Runtime not reported'} · {run.status}</span>
        </button>)}
      </nav>
      <main className="min-w-0 flex-1 space-y-3 overflow-y-auto p-3">
        {snapshot?.text && <section className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-3"><h2 className="mb-2 text-[11px] font-semibold">Run output</h2><pre className="whitespace-pre-wrap break-words text-xs leading-relaxed">{snapshot.text}</pre></section>}
        {activities.map((event, index) => <section key={`${snapshot?.run_id}-${index}`} className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-3">
          <div className="mb-1 flex items-center justify-between gap-2"><span className="text-[11px] font-medium">{event.kind.replace(/_/g, ' ')}</span><span className="font-mono text-[9px] text-[var(--color-fg-subtle)]">#{Math.max(0, (snapshot?.cursor ?? 0) - activities.length + index + 1)}</span></div>
          <pre className="whitespace-pre-wrap break-words text-[10px] leading-relaxed text-[var(--color-fg-muted)]">{eventText(event.payload)}</pre>
        </section>)}
        {selectedRun && activities.length === 0 && !snapshot?.text && <p role="status" className="p-4 text-xs text-[var(--color-fg-subtle)]">{snapshot?.reconnecting ? 'Reconnecting to run event stream…' : snapshot ? `No activity events recorded. Run status: ${snapshot.status}.` : 'Loading persisted run events…'}</p>}
      </main>
    </div>}
  </div>;
};
