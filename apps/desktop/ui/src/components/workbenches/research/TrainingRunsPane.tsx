import { useEffect, useRef, useState } from 'react';
import { RefreshCw, Search } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { CheckpointResumeAssessment, ResearchExecutionRecord } from '@/types/research';

type Attempt = ResearchExecutionRecord & { recipeName: string };
const readable = (value: string) => value.replace(/_/g, ' ');
const step = (value?: number | null) => value == null ? 'Unknown' : value.toLocaleString();

/** Read-only durable metadata. This view never starts a runner or allocates compute. */
export function TrainingRunsPane({ sessionId }: { sessionId?: string }) {
  const [attempts, setAttempts] = useState<Attempt[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(false);
  const [loadedSession, setLoadedSession] = useState<string | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const [assessment, setAssessment] = useState<{ id: string; result: CheckpointResumeAssessment } | null>(null);
  const [assessmentError, setAssessmentError] = useState<{ id: string; message: string } | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    let active = true;
    setAttempts([]); setSelectedId(null); setError(null); setAssessment(null);
    setSearch(''); setLoading(Boolean(sessionId));
    if (!sessionId) return () => { active = false; };
    void (async () => {
      try {
        const recipes = await daemonClient.listResearchRecipes();
        const records: Attempt[] = [];
        // Bound fan-out without silently omitting older recipes.
        for (let index = 0; active && index < recipes.length; index += 4) {
          const batch = await Promise.all(recipes.slice(index, index + 4).map(async recipe =>
            (await daemonClient.listResearchExecutions(recipe.id))
              .filter(record => record.sessionId === sessionId)
              .filter(record => record.backend != null || record.checkpoint != null)
              .map(record => ({ ...record, recipeName: recipe.name })),
          ));
          records.push(...batch.flat());
        }
        if (!active) return;
        records.sort((a, b) => b.startedAt.localeCompare(a.startedAt));
        setAttempts(records); setSelectedId(records[0]?.id ?? null);
      } catch (cause) {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        if (active) { setLoadedSession(sessionId); setLoading(false); }
      }
    })();
    return () => { active = false; };
  }, [sessionId, revision]);

  useEffect(() => {
    let active = true;
    setAssessment(null); setAssessmentError(null);
    if (!selectedId) return () => { active = false; };
    void daemonClient.assessResearchResume(selectedId)
      .then(result => { if (active) setAssessment({ id: selectedId, result }); })
      .catch(cause => { if (active) setAssessmentError({ id: selectedId, message: cause instanceof Error ? cause.message : String(cause) }); });
    return () => { active = false; };
  }, [selectedId, sessionId, revision]);

  const isLoading = loading || Boolean(sessionId && loadedSession !== sessionId);
  const filtered = (loadedSession === sessionId ? attempts : []).filter(record => `${record.recipeName} ${record.id} ${record.backend ?? ''}`.toLowerCase().includes(search.toLowerCase()));
  const selected = filtered.find(record => record.id === selectedId) ?? filtered[0];
  // Filtering may reveal a different record; only show an assessment for its exact ID.
  useEffect(() => { setSelectedId(selected?.id ?? null); }, [selected?.id]);

  return <section aria-label="Training attempts" aria-busy={isLoading} className="h-full min-h-0 flex flex-col bg-canvas text-editor-fg"
    onKeyDown={event => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || event.nativeEvent.isComposing) return;
      const target = event.target as HTMLElement;
      if (event.key === '/' && !target.closest('input, textarea, select, [contenteditable="true"], [role="textbox"]')) {
        event.preventDefault(); searchRef.current?.focus();
      }
    }}>
    <header className="wb-pane-toolbar px-4 py-3 border-b border-border-default flex items-center justify-between gap-3">
      <div><h2 className="text-sm font-medium">Training</h2><p className="text-xs text-fg-muted mt-1">Recorded attempts · refresh reads metadata, not GPU state</p></div>
      <button type="button" className="workbench-tool" disabled={isLoading || !sessionId} onClick={() => setRevision(value => value + 1)} aria-label="Refresh training metadata"><RefreshCw size={14} aria-hidden="true" /> Refresh</button>
    </header>
    {!sessionId ? <p className="p-4 text-sm text-fg-muted">Open a Research conversation to inspect its training attempts.</p>
      : isLoading ? <p role="status" className="p-4 text-sm text-fg-muted">Loading recorded attempts…</p>
      : error ? <div role="alert" className="p-4 text-sm"><p>Unable to read training records: {error}</p><button type="button" className="workbench-tool mt-3" onClick={() => setRevision(value => value + 1)}>Retry</button></div>
      : !attempts.length ? <div className="p-4 text-sm text-fg-muted space-y-3"><p>No training attempts recorded in this conversation.</p><p>This inspector reads saved execution records. Cloud launch, Unsloth runner and automatic GPU handoff are not connected yet. Ordinary notebook commands remain in Runs.</p></div>
      : <>
        <label className="m-3 flex items-center gap-2 border border-border-default rounded-md px-2 py-1.5 text-fg-muted">
          <Search size={14} aria-hidden="true" /><span className="sr-only">Search training attempts</span>
          <input ref={searchRef} className="w-full bg-transparent text-sm text-editor-fg" placeholder="Search attempts (/)" value={search} onChange={event => setSearch(event.target.value)} onKeyDown={event => { if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); setSearch(''); } }} />
        </label>
        {!filtered.length ? <p role="status" className="px-4 text-sm text-fg-muted">No matching attempts. Clear the search with Escape.</p> :
          <div className="flex-1 min-h-0 overflow-auto px-3 pb-4 space-y-3">
            <div aria-label="Recorded training attempts" className="flex flex-col gap-1" onKeyDown={event => {
              if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || !['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
              const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('button'));
              const index = buttons.indexOf(event.target as HTMLButtonElement);
              if (index < 0) return;
              event.preventDefault();
              const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length;
              buttons[next]?.focus();
            }}>
              {filtered.map(record => <button key={record.id} type="button" aria-pressed={selected?.id === record.id} className={`text-left rounded-md border px-3 py-2 ${selected?.id === record.id ? 'border-border-default bg-surface-hover' : 'border-transparent hover:bg-surface-hover'}`} onClick={() => setSelectedId(record.id)}>
                <span className="block text-sm font-medium">{record.recipeName}</span>
                <span className="block text-xs text-fg-muted break-all">{record.id} · {readable(record.backend ?? 'unknown backend')} · {record.status}</span>
              </button>)}
            </div>
            {selected && <article aria-label="Selected training attempt" className="border-t border-border-default pt-3 space-y-4 text-sm">
              <dl className="grid grid-cols-2 gap-x-4 gap-y-2">
                <dt className="text-fg-muted">Observed step</dt><dd>{step(selected.latestObservedStep)}</dd>
                <dt className="text-fg-muted">Local checkpoint</dt><dd>{step(selected.latestLocalCheckpointStep)}</dd>
                <dt className="text-fg-muted">Protected step (recorded)</dt><dd>{step(selected.latestProtectedCheckpointStep)}</dd>
              </dl>
              <p className="text-xs text-fg-muted">Observed progress can exceed recoverable progress. Recorded protection is a declaration, not a fresh remote integrity check.</p>
              {selected.resumeFromAttemptId && <p className="break-all">Continues {selected.resumeFromAttemptId} from checkpoint {selected.resumedFromCheckpointId ?? 'unknown'}</p>}
              {selected.outputNamespace && <p className="break-all text-xs text-fg-muted">Output namespace: {selected.outputNamespace}</p>}
              {selected.checkpoint && <div><h3 className="font-medium">Checkpoint inventory</h3><p className="text-xs text-fg-muted break-all mt-1">{selected.checkpoint.checkpointId} · {readable(selected.checkpoint.commitStatus)} · {readable(selected.checkpoint.resumabilityLevel)} · {selected.checkpoint.precision} · {selected.checkpoint.files.length} files</p></div>}
              <div aria-live="polite"><h3 className="font-medium">Continuation checks</h3>
                {assessmentError?.id === selected.id ? <p role="alert" className="mt-2">Assessment unavailable: {assessmentError.message}. Refresh to retry.</p>
                  : !assessment || selected.id !== assessment.id ? <p className="mt-2 text-fg-muted">Checking saved metadata…</p>
                  : <><p className="mt-2">{assessment.result.stateInventoryDeclared ? 'State inventory declared; restore is not verified.' : 'State-resume inventory is not eligible.'}</p>
                    <ul className="list-disc pl-5 mt-2 space-y-1 text-fg-muted">{[...assessment.result.blockers, ...assessment.result.unverifiedChecks].map((reason, index) => <li key={index}>{reason}</li>)}</ul>
                    <p className="mt-3 text-xs text-fg-muted">Read-only inspection. No compute was allocated and no training was started.</p></>}
              </div>
            </article>}
          </div>}
      </>}
    <footer className="px-3 py-2 border-t border-border-default text-xs text-fg-muted">/ Search · ↑↓ Focus attempt · Enter Select · Esc Clear search</footer>
  </section>;
}
