import React from 'react';
import {
  AlertTriangle,
  CheckCircle2,
  Clock,
  Layers,
  ShieldAlert,
  Zap,
} from 'lucide-react';
import type { SupervisionOverview } from '@/types/workflow_telemetry';

export interface DashboardKpiStripProps {
  overview: SupervisionOverview | null;
  counts: { attention: number; running: number; review: number; history: number };
  totalRuns: number;
  successRate: number;
  openFindingsCount: number;
  criticalFindingsCount: number;
  updatedAt: number | null;
}

export const DashboardKpiStrip: React.FC<DashboardKpiStripProps> = ({
  overview,
  counts,
  totalRuns,
  successRate,
  openFindingsCount,
  criticalFindingsCount,
  updatedAt,
}) => {
  return (
    <section
      aria-label="Execution metrics overview"
      className="grid shrink-0 grid-cols-2 gap-2 border-b border-white/10 bg-[#101113] px-4 py-2.5 sm:grid-cols-3 md:grid-cols-6"
    >
      <div className="rounded-lg border border-white/8 bg-[#141518]/60 p-2">
        <div className="flex items-center justify-between text-[10px] text-white/50">
          <span>Active Runs</span>
          <Zap size={12} className={overview?.active_runs ? 'text-amber-400 animate-pulse' : 'text-white/30'} />
        </div>
        <div className="mt-1 flex items-baseline gap-1.5">
          <span className="font-mono text-base font-semibold text-white">
            {overview?.active_runs ?? counts.running}
          </span>
          {overview?.active_runs ? (
            <span className="h-1.5 w-1.5 rounded-full bg-amber-400 animate-ping" />
          ) : null}
        </div>
      </div>

      <div className="rounded-lg border border-white/8 bg-[#141518]/60 p-2">
        <div className="flex items-center justify-between text-[10px] text-white/50">
          <span>Needs Attention</span>
          <AlertTriangle
            size={12}
            className={counts.attention > 0 ? 'text-rose-400' : 'text-white/30'}
          />
        </div>
        <div className="mt-1 font-mono text-base font-semibold text-white">
          <span className={counts.attention > 0 ? 'text-rose-400' : 'text-white'}>
            {counts.attention}
          </span>
        </div>
      </div>

      <div className="rounded-lg border border-white/8 bg-[#141518]/60 p-2">
        <div className="flex items-center justify-between text-[10px] text-white/50">
          <span>Success Rate</span>
          <CheckCircle2 size={12} className="text-emerald-400" />
        </div>
        <div className="mt-1 font-mono text-base font-semibold text-emerald-400">
          {successRate}%
        </div>
      </div>

      <div className="rounded-lg border border-white/8 bg-[#141518]/60 p-2">
        <div className="flex items-center justify-between text-[10px] text-white/50">
          <span>Total Executions</span>
          <Layers size={12} className="text-white/30" />
        </div>
        <div className="mt-1 font-mono text-base font-semibold text-white">{totalRuns}</div>
      </div>

      <div className="rounded-lg border border-white/8 bg-[#141518]/60 p-2">
        <div className="flex items-center justify-between text-[10px] text-white/50">
          <span>Watchdog Findings</span>
          <ShieldAlert
            size={12}
            className={criticalFindingsCount > 0 ? 'text-rose-400' : 'text-white/30'}
          />
        </div>
        <div className="mt-1 font-mono text-base font-semibold">
          <span className={criticalFindingsCount > 0 ? 'text-rose-400' : 'text-white'}>
            {openFindingsCount}
          </span>
          {criticalFindingsCount > 0 && (
            <span className="ml-1 text-[10px] text-rose-400">({criticalFindingsCount} crit)</span>
          )}
        </div>
      </div>

      <div className="rounded-lg border border-white/8 bg-[#141518]/60 p-2">
        <div className="flex items-center justify-between text-[10px] text-white/50">
          <span>Supervision Sync</span>
          <Clock size={12} className="text-white/30" />
        </div>
        <div className="mt-1 font-mono text-[11px] text-white/80">
          {updatedAt ? new Date(updatedAt).toLocaleTimeString() : 'Connecting…'}
        </div>
      </div>
    </section>
  );
};

