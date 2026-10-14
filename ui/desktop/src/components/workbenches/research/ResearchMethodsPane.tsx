import React, { useEffect, useMemo, useState } from 'react';
import { Box, Clock3, FileOutput, FlaskConical, TerminalSquare } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { ResearchExecutionRecord, ResearchRecipe } from '@/types/research';

const statusTone: Record<ResearchExecutionRecord['status'], string> = {
  pending: 'text-amber-500',
  running: 'text-blue-500',
  completed: 'text-emerald-500',
  failed: 'text-rose-500',
  cancelled: 'text-[var(--color-fg-subtle)]',
};

export const ResearchMethodsPane: React.FC = () => {
  const [recipes, setRecipes] = useState<ResearchRecipe[]>([]);
  const [selectedId, setSelectedId] = useState('');
  const [executions, setExecutions] = useState<ResearchExecutionRecord[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    daemonClient.listResearchRecipes()
      .then((records) => {
        setRecipes(records);
        setSelectedId(records[0]?.id ?? '');
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    if (!selectedId) {
      setExecutions([]);
      return;
    }
    daemonClient.listResearchExecutions(selectedId)
      .then(setExecutions)
      .catch((reason) => setError(String(reason)));
  }, [selectedId]);

  const selected = useMemo(
    () => recipes.find((recipe) => recipe.id === selectedId) ?? null,
    [recipes, selectedId]
  );

  if (loading) {
    return <div className="p-6 text-xs text-[var(--color-fg-muted)]">Loading method ledger…</div>;
  }

  if (error && recipes.length === 0) {
    return <div className="p-6 text-xs text-[var(--color-fg-muted)]">Method ledger unavailable: {error}</div>;
  }

  if (!selected) {
    return (
      <div className="flex h-full items-center justify-center bg-[var(--color-canvas)] p-8 text-center">
        <div className="max-w-sm">
          <FlaskConical className="mx-auto h-5 w-5 text-[var(--color-fg-subtle)]" />
          <h2 className="mt-3 text-sm font-semibold">No research methods yet</h2>
          <p className="mt-2 text-xs leading-5 text-[var(--color-fg-muted)]">
            A method recipe describes intended procedure and environment. It is not evidence that
            the command ran. Recipes are created through an authorized Research workflow.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="grid h-full min-h-0 grid-cols-[260px_minmax(0,1fr)] bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      <aside className="min-h-0 overflow-y-auto border-r border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3">
        <div className="px-2 pb-3 text-[10px] font-semibold uppercase tracking-[0.12em] text-[var(--color-fg-subtle)]">Methods</div>
        <div className="space-y-1">
          {recipes.map((recipe) => (
            <button
              key={recipe.id}
              type="button"
              onClick={() => setSelectedId(recipe.id)}
              className={`w-full rounded-lg px-3 py-2.5 text-left ${recipe.id === selectedId ? 'bg-[var(--color-surface-3)]' : 'hover:bg-[var(--color-surface-2)]'}`}
            >
              <div className="truncate text-xs font-medium">{recipe.name}</div>
              <div className="mt-1 truncate font-mono text-[10px] text-[var(--color-fg-subtle)]">{recipe.id}</div>
            </button>
          ))}
        </div>
      </aside>

      <main className="min-h-0 overflow-y-auto p-6">
        <div className="mx-auto max-w-4xl">
          <div className="workbench-kicker">reproduction method</div>
          <h1 className="mt-1 text-lg font-semibold tracking-[-0.02em]">{selected.name}</h1>
          {selected.description && <p className="mt-2 text-xs leading-5 text-[var(--color-fg-muted)]">{selected.description}</p>}

          <section className="mt-5 rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4">
            <div className="flex items-center gap-2 text-[10px] font-semibold uppercase tracking-[0.1em] text-[var(--color-fg-subtle)]">
              <TerminalSquare className="h-3.5 w-3.5" /> Intended command
            </div>
            <pre className="mt-3 overflow-x-auto whitespace-pre-wrap font-mono text-xs leading-5">{selected.command}</pre>
            <p className="mt-3 text-[10px] leading-4 text-[var(--color-fg-subtle)]">
              Display only. Opening this pane never dispatches the command.
            </p>
          </section>

          <div className="mt-4 grid gap-4 md:grid-cols-2">
            <section className="rounded-xl border border-[var(--color-border-muted)] p-4">
              <div className="flex items-center gap-2 text-xs font-semibold"><Box className="h-3.5 w-3.5" /> Environment</div>
              <dl className="mt-3 space-y-2 text-[11px] text-[var(--color-fg-muted)]">
                <div className="flex justify-between gap-4"><dt>Python</dt><dd>{selected.environmentSpec.pythonVersion ?? 'unspecified'}</dd></div>
                <div className="flex justify-between gap-4"><dt>Hardware</dt><dd>{selected.environmentSpec.hardware ?? 'unspecified'}</dd></div>
                <div className="flex justify-between gap-4"><dt>Container</dt><dd className="truncate">{selected.environmentSpec.containerImage ?? 'none'}</dd></div>
                <div className="flex justify-between gap-4"><dt>Requirements</dt><dd>{selected.environmentSpec.requirements.length}</dd></div>
              </dl>
            </section>
            <section className="rounded-xl border border-[var(--color-border-muted)] p-4">
              <div className="flex items-center gap-2 text-xs font-semibold"><FileOutput className="h-3.5 w-3.5" /> Contract</div>
              <div className="mt-3 text-[11px] leading-5 text-[var(--color-fg-muted)]">
                <div>{selected.inputs.length} versioned input reference(s)</div>
                <div>{selected.outputs.length} expected output path(s)</div>
              </div>
            </section>
          </div>

          <section className="mt-6">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="text-xs font-semibold">Observed executions</h2>
              <span className="text-[10px] text-[var(--color-fg-subtle)]">{executions.length} records</span>
            </div>
            <div className="space-y-2">
              {executions.map((record) => (
                <article key={record.id} className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4">
                  <div className="flex items-center justify-between gap-4">
                    <div className="min-w-0">
                      <div className="truncate font-mono text-[11px]">{record.id}</div>
                      <div className="mt-1 flex items-center gap-2 text-[10px] text-[var(--color-fg-subtle)]">
                        <Clock3 className="h-3 w-3" /> {record.startedAt}
                        {record.exitCode != null && <span>exit {record.exitCode}</span>}
                      </div>
                    </div>
                    <span className={`text-[10px] font-semibold uppercase tracking-[0.08em] ${statusTone[record.status]}`}>{record.status}</span>
                  </div>
                  <div className="mt-3 text-[10px] text-[var(--color-fg-muted)]">
                    {record.artifacts.length} artifact reference(s)
                    {record.stdoutCasUri ? ` · stdout ${record.stdoutCasUri}` : ''}
                  </div>
                </article>
              ))}
              {executions.length === 0 && (
                <div className="rounded-xl border border-dashed border-[var(--color-border-muted)] p-5 text-center text-xs text-[var(--color-fg-muted)]">
                  This method has no observed execution records.
                </div>
              )}
            </div>
          </section>
        </div>
      </main>
    </div>
  );
};
