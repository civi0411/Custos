import React from 'react';
import { resolveWorkbenchStrategy } from '@/components/workbenches/shared/run-inspector/workbenchWatchdogStrategy';
import type { WorkItem } from './types';

export interface WorkQueueCardProps {
  item: WorkItem;
  isSelected: boolean;
  currentProject: string;
  workspaceName?: string;
  onSelect: () => void;
}

export const WorkQueueCard: React.FC<WorkQueueCardProps> = ({
  item,
  isSelected,
  currentProject,
  workspaceName,
  onSelect,
}) => {
  const strategy = resolveWorkbenchStrategy(item.lens || item.workbench_id);
  const hasFindings = item.findings && item.findings.length > 0;

  return (
    <button
      type="button"
      aria-pressed={isSelected}
      onClick={onSelect}
      className={`block w-full rounded-lg border p-3 text-left transition ${
        isSelected
          ? 'border-sky-500 bg-[#16181d]'
          : 'border-white/8 bg-[#141518] hover:bg-[#18191d]'
      }`}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="truncate text-xs font-semibold text-white">{item.title}</span>
        <span
          className={`rounded px-1.5 py-0.5 font-mono text-[9px] uppercase font-medium border ${strategy.badgeClass}`}
        >
          {strategy.name}
        </span>
      </div>

      <div className="mt-1 flex items-center gap-2 font-mono text-[10px] text-white/60">
        <span
          className={`font-semibold uppercase ${
            item.status === 'running'
              ? 'text-amber-400'
              : item.status === 'success' || item.status === 'completed'
              ? 'text-emerald-400'
              : 'text-rose-400'
          }`}
        >
          {item.status}
        </span>
        <span>·</span>
        <span className="truncate">{item.model || 'Executor not reported'}</span>
        {item.duration_ms ? (
          <>
            <span>·</span>
            <span>{(item.duration_ms / 1000).toFixed(1)}s</span>
          </>
        ) : null}
      </div>

      {/* 3-Tier Chip & Run ID */}
      <div className="mt-1.5 flex items-center justify-between font-mono text-[10px] text-white/40">
        <span className="truncate">
          {item.project || currentProject} / {workspaceName || 'default'}
        </span>
        <span className="shrink-0 font-mono text-[9px] text-white/30">{item.run_id.slice(0, 8)}</span>
      </div>

      {/* Anomaly Watchdog Pills */}
      {hasFindings && (
        <div className="mt-2 flex flex-wrap gap-1">
          {item.findings!.map((f) => (
            <span
              key={f.id}
              className={`rounded px-1.5 py-0.5 font-mono text-[9px] uppercase font-semibold ${
                f.severity === 'critical'
                  ? 'bg-rose-500/20 text-rose-300 border border-rose-500/30'
                  : 'bg-amber-500/20 text-amber-300 border border-amber-500/30'
              }`}
            >
              {f.kind.replace('_', ' ')}
            </span>
          ))}
        </div>
      )}
    </button>
  );
};

