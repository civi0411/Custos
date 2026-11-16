import React from 'react';
import { AlertCircle, AlertTriangle, Info, CheckCircle2 } from 'lucide-react';
import type { SupervisionFinding } from '@/types/workflow_telemetry';
import { resolveWorkbenchStrategy } from './workbenchWatchdogStrategy';

interface FindingsFeedProps {
  findings: SupervisionFinding[];
  workbenchId?: string | null;
}

export const FindingsFeed: React.FC<FindingsFeedProps> = ({ findings, workbenchId }) => {
  const strategy = resolveWorkbenchStrategy(workbenchId);

  if (findings.length === 0) {
    return (
      <div className="flex items-center gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-3 text-xs text-emerald-400">
        <CheckCircle2 size={16} className="shrink-0 text-emerald-400" />
        <span>Watchdog: No anomalies detected across this run.</span>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {findings.map((f) => {
        const isCrit = f.severity === 'critical';
        const isWarn = f.severity === 'warning';
        const borderClass = isCrit
          ? 'border-rose-500/40 bg-rose-500/10 text-rose-300'
          : isWarn
          ? 'border-amber-500/40 bg-amber-500/10 text-amber-300'
          : 'border-sky-500/40 bg-sky-500/10 text-sky-300';
        const hint = strategy.remedyHints[f.kind];

        return (
          <div
            key={f.id}
            className={`rounded-lg border p-3 text-xs transition duration-150 ${borderClass}`}
          >
            <div className="flex items-start gap-2.5">
              {isCrit ? (
                <AlertCircle size={16} className="mt-0.5 shrink-0 text-rose-400" />
              ) : isWarn ? (
                <AlertTriangle size={16} className="mt-0.5 shrink-0 text-amber-400" />
              ) : (
                <Info size={16} className="mt-0.5 shrink-0 text-sky-400" />
              )}
              <div className="min-w-0 flex-1 space-y-1">
                <div className="flex items-center justify-between gap-2">
                  <span className="font-semibold uppercase tracking-wider text-[10px]">
                    {f.kind.replace(/_/g, ' ')}
                  </span>
                  <span className="font-mono text-[9px] text-white/50">
                    {new Date(f.created_at_ms).toLocaleTimeString()}
                  </span>
                </div>
                <p className="leading-relaxed text-[11px] text-white/90">{f.message}</p>
                {hint && (
                  <p className="mt-1 text-[10px] text-white/60 italic">
                    Remedy hint ({strategy.name}): {hint}
                  </p>
                )}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
};

