import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Activity, RefreshCw, Square } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { LiveRunSnapshot } from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

type SessionRun = { run_id: string; model?: string; status: string; journal_occurred_at?: string };

export interface ActivityPaneProps {
  sessionId?: string;
  initialRunId?: string;
  canCancel?: boolean;
  onShowToast?: (msg: string) => void;
  onAskAgent?: (draft: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

const activeStatus = (status: string) => ['queued', 'pending', 'active', 'running'].includes(status.toLowerCase());
const eventText = (value: unknown) => {
  if (typeof value === 'string') return value;
  try { return JSON.stringify(value, null, 2); } catch { return String(value); }
};

export const ActivityPane: React.FC<ActivityPaneProps> = ({ sessionId, initialRunId, canCancel = false, onShowToast, onOpenTab: _onOpenTab }) => {
  const generation = useRef(0);
  const [runs, setRuns] = useState<SessionRun[]>([]);
  const [selectedRunId, setSelectedRunId] = useState('');
  const [snapshot, setSnapshot] = useState<LiveRunSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [stopping, setStopping] = useState(false);
  const [cancelRequested, setCancelRequested] = useState(false);
  useEffect(() => { setCancelRequested(false); }, [sessionId, selectedRunId]);

  const refresh = useCallback(async () => {
    const request = ++generation.current;
    setLoading(true);
    setError('');
    try {
      const result = sessionId ? await daemonClient.listSessionRuns(sessionId) : [];
      if (request !== generation.current) return;
      const effective = result;
      setRuns(effective);
      setSelectedRunId((previous) =>
        (initialRunId && effective.some(run => run.run_id === initialRunId)) ? initialRunId : previous && effective.some((run) => run.run_id === previous)
          ? previous
          : effective.find((run) => activeStatus(run.status))?.run_id ?? effective[0]?.run_id ?? ''
      );
    } catch (cause) {
      if (request !== generation.current) return;
      setError(cause instanceof Error ? cause.message : String(cause));
      setRuns([]);
      setSelectedRunId('');
    } finally {
      if (request === generation.current) setLoading(false);
    }
  }, [sessionId, initialRunId]);

  useEffect(() => { setRuns([]); setSelectedRunId(''); setSnapshot(null); void refresh(); return () => { generation.current++; }; }, [refresh]);

  useEffect(() => {
    if (!selectedRunId) { setSnapshot(null); return; }
    if (!sessionId) { setSnapshot(null); return; }
    const controller = new AbortController();
    setSnapshot(null);
    daemonClient.watchRun(sessionId, selectedRunId, value => { if (!controller.signal.aborted && value.session_id === sessionId && value.run_id === selectedRunId) setSnapshot(value); }, controller.signal)
      .catch((cause) => {
        if (!controller.signal.aborted) {
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      });
    return () => controller.abort();
  }, [sessionId, selectedRunId]);

  const selectedRun = runs.find((run) => run.run_id === selectedRunId);
  const isActive = selectedRun ? activeStatus(snapshot?.status || selectedRun.status) : false;
  const activities = useMemo(() => snapshot?.activities ?? [], [snapshot]);

  const stop = async () => {
    if (!selectedRunId || !canCancel) return;
    setStopping(true);
    try {
      await daemonClient.cancelRun({ run_id: selectedRunId, reason: 'Stopped by user from Activity' });
      setCancelRequested(true);
      onShowToast?.('Cancellation requested; waiting for runtime confirmation.');
      await refresh();
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
    } finally { setStopping(false); }
  };

  return <div className="flex h-full min-h-0 flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
    <header className="wb-pane-toolbar flex h-11 shrink-0 items-center justify-between gap-3 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]/50 px-3">
      <div className="flex min-w-0 items-center gap-2"><Activity className="h-4 w-4 shrink-0 text-[var(--color-fg-muted)]" /><span className="text-xs font-semibold">Run activity</span>
        {selectedRun && <span className={`truncate text-[10px] ${isActive ? 'text-white/80 font-medium' : 'text-[var(--color-fg-subtle)]'}`}>{snapshot?.status ?? selectedRun.status}</span>}
      </div>
      <div className="flex items-center gap-1.5">
        <button type="button" onClick={() => void refresh()} disabled={loading} aria-label="Refresh run activity" title="Refresh run activity" className="rounded border border-[var(--color-border-subtle)] p-1.5 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] disabled:opacity-50"><RefreshCw className={`h-3.5 w-3.5 ${loading ? 'animate-spin' : ''}`} /></button>
        {isActive && canCancel && <button type="button" onClick={() => void stop()} disabled={stopping || cancelRequested} className="flex items-center gap-1 rounded bg-white/10 hover:bg-white/15 px-2.5 py-1 text-[11px] font-medium text-white border border-white/20 disabled:opacity-50"><Square className="h-3 w-3 fill-current" />{stopping ? 'Requesting…' : cancelRequested ? 'Cancel requested' : 'Cancel run'}</button>}
      </div>
    </header>

    {snapshot?.reconnecting && <p role="status" className="border-b border-border-muted px-4 py-2 text-xs text-white/70">Reconnecting. Last observed state may be stale.</p>}
    {error && <p role="alert" className="border-b border-rose-500/20 px-4 py-2 text-xs text-rose-400">Could not load live activity: {error}</p>}
    <div className="wb-split flex min-h-0 flex-1">
      <nav aria-label="Session runs" className="wb-master w-64 shrink-0 space-y-1 overflow-y-auto border-r border-[var(--color-border-subtle)] p-2">
        {runs.length === 0 && !loading && !error && <p className="p-3 text-xs text-[var(--color-fg-subtle)]">{sessionId ? 'No recorded runs for this session.' : 'Choose a session to inspect its runs.'}</p>}
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
    </div>
  </div>;
};
