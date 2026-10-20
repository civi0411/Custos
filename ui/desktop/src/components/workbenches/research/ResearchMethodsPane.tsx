import { useEffect, useState } from 'react';
import { RefreshCw, Search } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { WorkbenchPane, WorkbenchEmptyState, WorkbenchError } from '@/components/common/WorkbenchPane';
import type { ResearchExecutionRecord, ResearchRecipe } from '@/types/research';

export function ResearchMethodsPane() {
  const [recipes, setRecipes] = useState<ResearchRecipe[]>([]);
  const [selectedId, setSelectedId] = useState('');
  const [executions, setExecutions] = useState<ResearchExecutionRecord[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [executionsLoading, setExecutionsLoading] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [query, setQuery] = useState('');
  useEffect(() => {
    let active = true;
    setLoading(true);
    daemonClient.listResearchRecipes().then(records => {
      if (!active) return;
      setRecipes(records);
      setSelectedId(previous => records.some(record => record.id === previous) ? previous : records[0]?.id || '');
      setError(null);
    }).catch(reason => { if (active) setError(String(reason)); }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [attempt]);
  useEffect(() => {
    let active = true;
    setExecutions([]);
    if (!selectedId) { setExecutionsLoading(false); return; }
    setExecutionsLoading(true);
    daemonClient.listResearchExecutions(selectedId).then(records => { if (active) setExecutions(records); })
      .catch(reason => { if (active) setError(String(reason)); }).finally(() => { if (active) setExecutionsLoading(false); });
    return () => { active = false; };
  }, [selectedId, attempt]);
  const selected = recipes.find(recipe => recipe.id === selectedId);
  const filtered = recipes.filter(recipe => (recipe.name + ' ' + (recipe.description || '')).toLowerCase().includes(query.toLowerCase()));

  return <WorkbenchPane toolbar={<>
    <span className="wb-meta">{recipes.length} recorded methods</span>
    <span className="wb-toolbar-spacer" />
    <button type="button" className="wb-icon-button" aria-label="Refresh research methods" title="Refresh" disabled={loading} onClick={() => setAttempt(value => value + 1)}><RefreshCw size={14} /></button>
  </>}>
    <WorkbenchError message={error} onRetry={() => setAttempt(value => value + 1)} />
    <div className="wb-split flex flex-1 min-h-0">
      <aside className="wb-master flex flex-col min-h-0 border-r border-border-subtle">
        <label className="wb-search"><Search size={13} aria-hidden="true" /><input aria-label="Find a research method" placeholder="Find a method…" value={query} onChange={event => setQuery(event.target.value)} /></label>
        <div className="flex-1 overflow-auto">
          {loading && !recipes.length ? <WorkbenchEmptyState title="Loading methods…" busy /> : filtered.map(recipe => <div key={recipe.id} className={'wb-source-row ' + (recipe.id === selectedId ? 'is-selected' : '')}>
            <button type="button" aria-current={recipe.id === selectedId ? 'true' : undefined} onClick={() => setSelectedId(recipe.id)}><span>{recipe.name}</span>{recipe.description && <small className="line-clamp-2">{recipe.description}</small>}</button>
          </div>)}
          {!loading && !filtered.length && <WorkbenchEmptyState title={query ? 'No matching methods' : 'No methods recorded'} />}
        </div>
      </aside>
      <main className="min-h-0 min-w-0 flex-1 overflow-auto">
        {selected ? <article className="wb-document select-text">
          <h1>{selected.name}</h1>
          {selected.description && <p className="wb-document-byline">{selected.description}</p>}
          <section><h2>Command</h2><pre className="wb-code-block">{selected.command}</pre><p className="wb-meta">Recorded recipe · opening this view does not execute it.</p></section>
          <section><h2>Environment</h2><dl className="wb-property-list">
            <dt>Python</dt><dd>{selected.environmentSpec.pythonVersion || 'Not recorded'}</dd>
            <dt>Hardware</dt><dd>{selected.environmentSpec.hardware || 'Not recorded'}</dd>
            <dt>Container</dt><dd>{selected.environmentSpec.containerImage || 'Not specified'}</dd>
            <dt>Requirements</dt><dd>{selected.environmentSpec.requirements.length ? <pre className="whitespace-pre-wrap">{selected.environmentSpec.requirements.join('\n')}</pre> : 'Not recorded'}</dd>
            <dt>Inputs</dt><dd>{selected.inputs.length} versioned references</dd>
            <dt>Outputs</dt><dd>{selected.outputs.length} expected paths</dd>
          </dl></section>
          <section><h2>Observed executions</h2>
            {executionsLoading ? <p role="status" className="wb-meta">Loading executions…</p> : !executions.length ? <p className="wb-meta">No execution records for this method.</p> : executions.map(record => <details className="wb-disclosure" key={record.id}>
              <summary><span>{record.startedAt}</span><span className="ml-3">{record.status}</span>{record.exitCode != null && <span className="ml-3">exit {record.exitCode}</span>}</summary>
              <dl className="wb-property-list"><dt>Execution</dt><dd className="font-mono">{record.id}</dd><dt>Artifacts</dt><dd>{record.artifacts.length} recorded references</dd><dt>Output log</dt><dd>{record.stdoutCasUri || 'Not recorded'}</dd></dl>
            </details>)}
          </section>
        </article> : <WorkbenchEmptyState title="Reproduction methods" description="Recorded recipes keep commands, environment requirements and observed runs together. Choose a method to inspect its history." />}
      </main>
    </div>
  </WorkbenchPane>;
}
