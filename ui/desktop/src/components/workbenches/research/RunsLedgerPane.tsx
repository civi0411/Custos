import React, { useState, useEffect } from 'react';
import {
  FlaskConical,
  Search,
  ChevronDown,
  ChevronRight,
  Database,
} from 'lucide-react';
import { ResearchExperimentRun } from '@/types/research';
import { daemonClient } from '@/api/daemon_client';
import { JobActionButton } from '@/components/common';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';
import { RunDetailRow } from './RunDetailRow';

interface RunsLedgerPaneProps {
  sessionId?: string;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (tabId: OrcaTabId) => void;
}

export const RunsLedgerPane: React.FC<RunsLedgerPaneProps> = ({
  sessionId,
  onShowToast,
  onOpenTab,
}) => {
  const [runs, setRuns] = useState<ResearchExperimentRun[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'ok' | 'failed'>('all');
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  useEffect(() => {
    daemonClient
      .listResearchRuns()
      .then((loaded) => {
        const scoped = sessionId ? loaded.filter(run => run.sessionId === sessionId) : [];
        setRuns(scoped);
        setExpandedId(scoped[0]?.runId ?? null);
      })
      .catch((err) => setLoadError(String(err)));
  }, [sessionId]);

  const filteredRuns = runs.filter((r) => {
    const matchesSearch =
      r.command.toLowerCase().includes(search.toLowerCase()) ||
      r.runId.toLowerCase().includes(search.toLowerCase());
    const matchesStatus =
      statusFilter === 'all' || (statusFilter === 'ok' ? r.status === 'ok' : r.status === 'failed');
    return matchesSearch && matchesStatus;
  });

  const handleCopyCommand = (run: ResearchExperimentRun) => {
    navigator.clipboard.writeText(run.command);
    setCopiedId(run.runId);
    setTimeout(() => setCopiedId(null), 1500);
  };


  if (loadError && runs.length === 0) {
    return (
      <div className="flex h-full items-center justify-center bg-[var(--color-canvas)] p-6 text-xs text-[var(--color-fg-muted)]">
        Experiment ledger unavailable: {loadError}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full w-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] overflow-hidden select-none font-sans relative">
      {/* 48px Standard Pane Header */}
      <header className="wb-pane-toolbar flex h-12 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-5 z-10">
        <div className="flex items-center gap-2.5">
          <FlaskConical className="w-4 h-4 workbench-accent" />
          <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
            Experiment Ledger
          </span>
          <span className="text-[10px] text-[var(--color-fg-muted)] font-mono font-medium bg-[var(--color-surface-2)] px-2 py-0.5 rounded border border-[var(--color-border-muted)] ml-1">
            {runs.length} recorded run(s)
          </span>
        </div>
        <div className="flex items-center gap-2">
          <JobActionButton
            jobId="research__dataset_audit"
            label="Audit Dataset"
            icon={Database}
            getInputs={() => {
              if (!runs[0]?.outputFiles?.[0]?.path) return null;
              return {
                dataset_path: runs[0].outputFiles[0].path,
                benchmark_split: 'eval',
              };
            }}
            sessionId={sessionId}
            notReadyTooltip="No experiment runs or dataset artifacts available to audit"
            readyTooltip="Audit training/eval dataset distributions and lineage invariants"
            onShowToast={onShowToast}
            variant="toolbar"
            size="sm"
          />
        </div>
      </header>

      {/* Filter Toolbar */}
      <div className="p-3.5 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)]/70 flex flex-wrap items-center gap-3 text-xs z-10">
        {/* Search */}
        <div className="relative min-w-[240px] flex-1">
          <Search className="w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-fg-subtle)]" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search commands, run IDs..."
            className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-lg py-1.5 pl-8 pr-3 text-xs text-[var(--color-editor-fg)] placeholder-[var(--color-fg-subtle)] outline-none focus:border-[var(--workbench-accent)] transition shadow-xs"
          />
        </div>

        {/* Facet Chips */}
        <div className="flex items-center gap-1.5">
          <button
            onClick={() => setStatusFilter(statusFilter === 'ok' ? 'all' : 'ok')}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg border text-xs font-medium transition ${
              statusFilter === 'ok'
                ? 'bg-white/10 border-white/20 text-white'
                : 'bg-[var(--color-surface-0)] border-[var(--color-border-muted)] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]'
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${statusFilter === 'ok' ? 'bg-white/90' : 'bg-[var(--color-fg-subtle)]'}`} />
            <span>Success</span>
          </button>

          <button
            onClick={() => setStatusFilter(statusFilter === 'failed' ? 'all' : 'failed')}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg border text-xs font-medium transition ${
              statusFilter === 'failed'
                ? 'bg-white/10 border-white/20 text-white'
                : 'bg-[var(--color-surface-0)] border-[var(--color-border-muted)] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]'
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${statusFilter === 'failed' ? 'bg-red-400' : 'bg-[var(--color-fg-subtle)]'}`} />
            <span>Failed</span>
          </button>
        </div>
      </div>

      {/* Runs List Scroll Area */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3 select-text max-w-5xl mx-auto w-full">
        {filteredRuns.map((run) => {
          const isExpanded = expandedId === run.runId;
          const isOk = run.status === 'ok';

          return (
            <div
              key={run.runId}
              className={`rounded-xl border transition overflow-hidden ${
                isExpanded
                  ? 'bg-[var(--color-surface-1)] border-[var(--color-border-default)] shadow-xs'
                  : 'bg-[var(--color-surface-0)] border-[var(--color-border-muted)] hover:border-[var(--color-border-default)] hover:bg-[var(--color-surface-1)]/60'
              }`}
            >
              {/* Row Header Button */}
              <div
                onClick={() => setExpandedId(isExpanded ? null : run.runId)}
                className="p-3.5 flex items-center justify-between gap-4 cursor-pointer transition select-none"
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div className="p-1 rounded text-[var(--color-fg-muted)]">
                    {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                  </div>
                  <span
                    className={`w-2 h-2 rounded-full shrink-0 ${
                      isOk ? 'bg-white/90' : 'bg-red-400'
                    }`}
                  />
                  <span className="font-mono text-xs text-[var(--color-editor-fg)] font-medium truncate">
                    {run.command}
                  </span>
                </div>

                <div className="flex items-center gap-3 shrink-0 text-[11px] text-[var(--color-fg-muted)] font-mono">
                  {run.surface && (
                    <span className="px-2 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] uppercase text-[10px] text-[var(--color-fg-subtle)]">
                      {run.surface}
                    </span>
                  )}
                  <span className="bg-[var(--color-surface-2)] px-2 py-0.5 rounded border border-[var(--color-border-muted)]">
                    {(run.wallMs / 1000).toFixed(1)}s
                  </span>
                  <span className="text-[var(--color-fg-subtle)]">
                    {new Date(run.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                  </span>
                </div>
              </div>

              {/* Expanded Details Pane */}
              {isExpanded && (
                <RunDetailRow
                  run={run}
                  sessionId={sessionId}
                  onShowToast={onShowToast}
                  onOpenTab={onOpenTab}
                  copiedId={copiedId}
                  onCopyCommand={handleCopyCommand}
                />
              )}
            </div>
          );
        })}
        {filteredRuns.length === 0 && (
          <div className="flex flex-col items-center justify-center p-8 text-center text-xs text-[var(--color-fg-muted)]">
            <FlaskConical className="h-6 w-6 text-[var(--color-fg-subtle)] mb-1.5" />
            <span>No experiment runs recorded</span>
          </div>
        )}
      </div>
    </div>
  );
};
