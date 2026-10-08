import React, { useState, useEffect } from 'react';
import {
  FlaskConical,
  Search,
  Check,
  RotateCcw,
  ChevronDown,
  ChevronRight,
  Cpu,
  Package,
  FileCode2,
  FileOutput,
  Copy,
  Terminal,
} from 'lucide-react';
import { ResearchExperimentRun } from '@/types/research';
import { daemonClient } from '@/api/daemon_client';

interface RunsLedgerPaneProps {
  onReproduce?: (run: ResearchExperimentRun) => void;
  onShowToast?: (msg: string) => void;
}

export const RunsLedgerPane: React.FC<RunsLedgerPaneProps> = ({
  onReproduce,
  onShowToast,
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
        setRuns(loaded);
        setExpandedId(loaded[0]?.runId ?? null);
      })
      .catch((err) => setLoadError(String(err)));
  }, []);

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

  const handleReproduce = (run: ResearchExperimentRun) => {
    onReproduce?.(run);
    onShowToast?.(`Drafted reproducible prompt for ${run.runId} into Research Chat!`);
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
      <header className="flex h-12 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-5 z-10">
        <div className="flex items-center gap-2.5">
          <FlaskConical className="w-4 h-4 workbench-accent" />
          <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
            Experiment Ledger
          </span>
          <span className="text-[10px] text-[var(--color-fg-muted)] font-mono font-medium bg-[var(--color-surface-2)] px-2 py-0.5 rounded border border-[var(--color-border-muted)] ml-1">
            {runs.length} recorded run(s)
          </span>
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
                ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-500'
                : 'bg-[var(--color-surface-0)] border-[var(--color-border-muted)] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]'
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${statusFilter === 'ok' ? 'bg-emerald-500' : 'bg-[var(--color-fg-subtle)]'}`} />
            <span>Success</span>
          </button>

          <button
            onClick={() => setStatusFilter(statusFilter === 'failed' ? 'all' : 'failed')}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg border text-xs font-medium transition ${
              statusFilter === 'failed'
                ? 'bg-rose-500/10 border-rose-500/30 text-rose-500'
                : 'bg-[var(--color-surface-0)] border-[var(--color-border-muted)] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]'
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${statusFilter === 'failed' ? 'bg-rose-500' : 'bg-[var(--color-fg-subtle)]'}`} />
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
                      isOk ? 'bg-emerald-500' : 'bg-rose-500'
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
                <div className="border-t border-[var(--color-border-muted)] p-5 bg-[var(--color-surface-1)] space-y-5">
                  {/* Action Bar */}
                  <div className="flex items-center justify-between text-xs">
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => handleReproduce(run)}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] font-medium transition"
                        title="Generate verification prompt with exact environment & lockfile"
                      >
                        <RotateCcw size={13} />
                        <span>Reproduce Run</span>
                      </button>

                      <button
                        onClick={() => handleCopyCommand(run)}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition font-medium"
                      >
                        {copiedId === run.runId ? (
                          <Check size={13} className="text-emerald-500" />
                        ) : (
                          <Copy size={13} />
                        )}
                        <span>Copy Command</span>
                      </button>
                    </div>

                    <div className="font-mono text-[10px] text-[var(--color-fg-subtle)] bg-[var(--color-surface-2)] px-2.5 py-1 rounded border border-[var(--color-border-muted)]">
                      Merkle Hash: <span className="text-[var(--color-editor-fg)] ml-1 font-semibold">{run.outputMerkleRoot}</span>
                    </div>
                  </div>

                  {/* Hardware & Env Chips */}
                  <div className="flex flex-wrap items-center gap-2 font-mono text-[11px]">
                    <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] border border-[var(--color-border-muted)]">
                      <Cpu size={12} className="text-[var(--color-fg-muted)]" />
                      <span className="font-medium">{run.envSnapshot.hardware}</span>
                    </span>

                    <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] border border-[var(--color-border-muted)]">
                      <Package size={12} className="text-[var(--color-fg-muted)]" />
                      <span className="font-medium">Python {run.envSnapshot.pythonVersion}</span>
                      <span className="text-[var(--color-fg-subtle)]">({run.envSnapshot.packageCount} pkgs)</span>
                    </span>

                    <span className="px-2.5 py-1 rounded-lg bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border border-[var(--color-border-muted)] font-medium">
                      Lockfile: {run.envSnapshot.lockfileHash.slice(0, 16)}...
                    </span>
                  </div>

                  {/* Input Code & Outputs */}
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs">
                    {run.codeFiles && (
                      <div className="p-3.5 rounded-xl bg-[var(--color-surface-0)] border border-[var(--color-border-muted)] space-y-2.5">
                        <div className="text-[10px] text-[var(--color-fg-subtle)] font-semibold uppercase tracking-wider flex items-center gap-1.5">
                          <FileCode2 size={12} /> 
                          Code Inputs
                        </div>
                        <div className="space-y-1.5">
                          {run.codeFiles.map((f) => (
                            <div key={f.path} className="font-mono text-xs flex justify-between items-center bg-[var(--color-surface-1)] p-2 rounded-lg border border-[var(--color-border-muted)]">
                              <span className="truncate text-[var(--color-editor-fg)]">{f.path}</span>
                              <span className="text-[var(--color-fg-subtle)] text-[10px] ml-2 shrink-0">{f.hash}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    {run.outputFiles && (
                      <div className="p-3.5 rounded-xl bg-[var(--color-surface-0)] border border-[var(--color-border-muted)] space-y-2.5">
                        <div className="text-[10px] text-[var(--color-fg-subtle)] font-semibold uppercase tracking-wider flex items-center gap-1.5">
                          <FileOutput size={12} /> 
                          Captured Outputs
                        </div>
                        <div className="space-y-1.5">
                          {run.outputFiles.map((f) => (
                            <div key={f.path} className="font-mono text-xs flex justify-between items-center bg-[var(--color-surface-1)] p-2 rounded-lg border border-[var(--color-border-muted)]">
                              <span className="truncate text-[var(--color-editor-fg)]">{f.path}</span>
                              <span className="text-emerald-500 font-medium bg-emerald-500/10 px-1.5 py-0.5 rounded text-[10px] ml-2 shrink-0">
                                {(f.size / 1024).toFixed(1)} KB
                              </span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>

                  {/* Stdout / Stderr Log */}
                  {run.logText && (
                    <div className="rounded-xl bg-[var(--color-canvas-inset)] border border-[var(--color-border-muted)] overflow-hidden">
                      <div className="bg-[var(--color-surface-2)] px-3.5 py-1.5 border-b border-[var(--color-border-muted)] text-[10px] text-[var(--color-fg-muted)] font-semibold uppercase tracking-wider flex items-center gap-1.5">
                        <Terminal size={12} /> 
                        Execution Trace
                      </div>
                      <div className="p-3.5 font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] overflow-x-auto">
                        <pre className="whitespace-pre-wrap">
                          {run.logText.split('\n').map((line, i) => {
                            let colorClass = 'text-[var(--color-fg-muted)]';
                            if (line.includes('[FATAL]') || line.includes('Violation')) colorClass = 'text-rose-500 font-semibold';
                            else if (line.includes('[Result]') || line.includes('verified')) colorClass = 'text-emerald-500';
                            else if (line.includes('[SADE]')) colorClass = 'text-[var(--workbench-accent)]';
                            
                            return (
                              <div key={i} className={colorClass}>{line}</div>
                            );
                          })}
                        </pre>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
