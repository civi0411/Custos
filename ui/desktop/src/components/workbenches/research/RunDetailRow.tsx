import React from 'react';
import {
  RotateCcw,
  Cpu,
  Package,
  FileCode2,
  FileOutput,
  Copy,
  Check,
  Terminal,
  BarChart3,
  ShieldCheck,
  GitBranch,
  BookmarkCheck,
} from 'lucide-react';
import type { ResearchExperimentRun } from '@/types/research';
import { JobActionButton } from '@/components/common';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export interface RunDetailRowProps {
  run: ResearchExperimentRun;
  sessionId?: string;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (tabId: OrcaTabId) => void;
  copiedId: string | null;
  onCopyCommand: (run: ResearchExperimentRun) => void;
}

export const RunDetailRow: React.FC<RunDetailRowProps> = ({
  run,
  sessionId,
  onShowToast,
  onOpenTab,
  copiedId,
  onCopyCommand,
}) => {
  return (
    <div className="border-t border-[var(--color-border-muted)] p-5 bg-[var(--color-surface-1)] space-y-5">
      {/* Action Bar */}
      <div className="flex items-center justify-between text-xs">
        <div className="flex items-center gap-2">
          <JobActionButton
            jobId="research__reproduce"
            label="Reproduce"
            icon={RotateCcw}
            getInputs={() => null}
            notReadyTooltip="This run has no recorded reproduction manifest with exact config and seeds. Open its method recipe before reproducing."
            sessionId={sessionId}
            sourceTabId="runs"
            onNavigate={onOpenTab}
            readyTooltip={`Execute autonomous reproduction for run ${run.runId}`}
            onShowToast={onShowToast}
            variant="toolbar"
            size="sm"
          />
          <JobActionButton
            jobId="research__result_analysis"
            label="Analyze Results"
            icon={BarChart3}
            getInputs={() => ({
              run_logs: `Run ID: ${run.runId}\nCommand: ${run.command}\nStatus: ${run.status}\nWallMs: ${run.wallMs}\nMerkle: ${run.outputMerkleRoot}`,
              baseline_run_id: run.runId,
            })}
            sessionId={sessionId}
            sourceTabId="runs"
            navigateOnSuccess="artifacts"
            onNavigate={onOpenTab}
            readyTooltip={`Perform deep analysis of telemetry and outputs for ${run.runId}`}
            onShowToast={onShowToast}
            variant="toolbar"
            size="sm"
          />

          <button
            onClick={() => onCopyCommand(run)}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition text-xs font-medium"
          >
            {copiedId === run.runId ? (
              <Check size={12} className="text-white/80" />
            ) : (
              <Copy size={12} />
            )}
            <span>Copy Command</span>
          </button>
        </div>

        <div className="font-mono text-[10px] text-[var(--color-fg-subtle)] bg-[var(--color-surface-2)] px-2.5 py-1 rounded border border-[var(--color-border-muted)]">
          Merkle Hash:{' '}
          <span className="text-[var(--color-editor-fg)] ml-1 font-semibold">
            {run.outputMerkleRoot}
          </span>
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
          <span className="text-[var(--color-fg-subtle)]">
            ({run.envSnapshot.packageCount} pkgs)
          </span>
        </span>

        <span className="px-2.5 py-1 rounded-lg bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border border-[var(--color-border-muted)] font-medium">
          Lockfile: {run.envSnapshot.lockfileHash.slice(0, 16)}...
        </span>
      </div>

      {/* Checkpoint Relay & Durability Milestones */}
      {run.checkpointRelay && (
        <div className="p-3.5 rounded-xl bg-[var(--color-surface-0)] border border-[var(--color-border-muted)] space-y-2.5">
          <div className="flex items-center justify-between text-[10px] text-[var(--color-fg-subtle)] font-semibold uppercase tracking-wider">
            <div className="flex items-center gap-1.5">
              <ShieldCheck size={12} className="text-emerald-400" />
              <span>Durable Checkpoint Relay</span>
            </div>
            {run.checkpointRelay.resumabilityLevel && (
              <span className="px-2 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] border border-[var(--color-border-muted)] font-mono text-[10px] normal-case">
                {run.checkpointRelay.resumabilityLevel === 'validated_resume'
                  ? 'Validated Parity'
                  : run.checkpointRelay.resumabilityLevel === 'training_state_resume'
                  ? 'Training-State Resume'
                  : run.checkpointRelay.resumabilityLevel === 'warm_start'
                  ? 'Warm Start'
                  : 'Export Only'}
              </span>
            )}
          </div>

          <div className="grid grid-cols-3 gap-2 font-mono text-xs">
            <div className="p-2 rounded-lg bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] flex flex-col">
              <span className="text-[10px] text-[var(--color-fg-muted)]">Observed Step</span>
              <span className="font-semibold text-[var(--color-editor-fg)]">
                {run.checkpointRelay.latestObservedStep ?? '—'}
              </span>
            </div>

            <div className="p-2 rounded-lg bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] flex flex-col">
              <span className="text-[10px] text-[var(--color-fg-muted)]">Local Staged</span>
              <span className="font-semibold text-blue-400">
                {run.checkpointRelay.latestLocalCheckpointStep ?? '—'}
              </span>
            </div>

            <div className="p-2 rounded-lg border border-emerald-500/30 bg-emerald-500/5 flex flex-col">
              <span className="text-[10px] text-emerald-400 font-medium flex items-center gap-1">
                <BookmarkCheck size={10} /> Protected Step
              </span>
              <span className="font-semibold text-emerald-400">
                {run.checkpointRelay.latestProtectedCheckpointStep ?? '—'}
              </span>
            </div>
          </div>

          {(run.checkpointRelay.resumedFromCheckpointId ||
            run.checkpointRelay.outputNamespace) && (
            <div className="flex items-center gap-3 text-[11px] font-mono text-[var(--color-fg-subtle)] pt-1 border-t border-[var(--color-border-muted)]">
              {run.checkpointRelay.resumedFromCheckpointId && (
                <span className="flex items-center gap-1 text-[var(--color-fg-muted)]">
                  <GitBranch size={11} /> Resumed from:{' '}
                  <span className="text-[var(--color-editor-fg)] font-medium">
                    {run.checkpointRelay.resumedFromCheckpointId}
                  </span>
                </span>
              )}
              {run.checkpointRelay.outputNamespace && (
                <span className="text-[var(--color-fg-subtle)]">
                  Namespace:{' '}
                  <span className="text-[var(--color-editor-fg)]">
                    {run.checkpointRelay.outputNamespace}
                  </span>
                </span>
              )}
            </div>
          )}
        </div>
      )}

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
                <div
                  key={f.path}
                  className="font-mono text-xs flex justify-between items-center bg-[var(--color-surface-1)] p-2 rounded-lg border border-[var(--color-border-muted)]"
                >
                  <span className="truncate text-[var(--color-editor-fg)]">{f.path}</span>
                  <span className="text-[var(--color-fg-subtle)] text-[10px] ml-2 shrink-0">
                    {f.hash}
                  </span>
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
                <div
                  key={f.path}
                  className="font-mono text-xs flex justify-between items-center bg-[var(--color-surface-1)] p-2 rounded-lg border border-[var(--color-border-muted)]"
                >
                  <span className="truncate text-[var(--color-editor-fg)]">{f.path}</span>
                  <span className="text-white/80 font-medium bg-white/5 border border-white/10 px-1.5 py-0.5 rounded text-[10px] ml-2 shrink-0">
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
                if (line.includes('[FATAL]') || line.includes('Violation'))
                  colorClass = 'text-red-400 font-semibold';
                else if (line.includes('[Result]') || line.includes('verified'))
                  colorClass = 'text-white/90 font-medium';
                else if (line.includes('[SADE]'))
                  colorClass = 'text-[var(--workbench-accent)]';

                return (
                  <div key={i} className={colorClass}>
                    {line}
                  </div>
                );
              })}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
};
