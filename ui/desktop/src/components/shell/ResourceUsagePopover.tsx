import React, { useState } from 'react';
import {
  RotateCw,
  Server,
  AppWindow,
  Terminal,
  Plug,
  HardDrive,
  Cpu,
  Coffee,
  Activity,
} from 'lucide-react';

export interface ProcessMetric {
  id: string;
  name: string;
  kind: string;
  pid: number;
  memory_mb: number;
  cpu_percent: number;
}

export interface ResourceUsagePopoverProps {
  isOpen: boolean;
  onClose: () => void;
  onRefresh?: () => void;
  memoryResidentMb: number;
  systemTotalMemoryMb?: number | null;
  processGroups?: ProcessMetric[];
  activePorts?: number[];
  activeRuns?: number;
  caffeinateActive?: boolean;
  position?: 'top' | 'bottom';
  align?: 'left' | 'right' | 'center';
}

export const ResourceUsagePopover: React.FC<ResourceUsagePopoverProps> = ({
  isOpen,
  onClose,
  onRefresh,
  memoryResidentMb,
  systemTotalMemoryMb,
  processGroups = [],
  activePorts = [],
  activeRuns = 0,
  caffeinateActive = false,
  position = 'top',
  align = 'right',
}) => {
  const [isRefreshing, setIsRefreshing] = useState(false);

  if (!isOpen) return null;

  const handleRefresh = (e: React.MouseEvent) => {
    e.stopPropagation();
    setIsRefreshing(true);
    onRefresh?.();
    setTimeout(() => setIsRefreshing(false), 500);
  };

  const totalRamGb = systemTotalMemoryMb ? (systemTotalMemoryMb / 1024).toFixed(0) : null;
  const memoryPct = systemTotalMemoryMb && systemTotalMemoryMb > 0
    ? ((memoryResidentMb / systemTotalMemoryMb) * 100).toFixed(1)
    : null;

  const positionClasses =
    position === 'bottom'
      ? 'fixed bottom-10 right-4'
      : align === 'right'
        ? 'fixed bottom-8 right-6'
        : 'fixed bottom-8 left-12';

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-[90] bg-black/40 backdrop-blur-[1px]"
        onClick={onClose}
      />

      {/* Floating Resource Usage Card */}
      <div
        className={`${positionClasses} w-[360px] sm:w-[400px] bg-[#141518] border border-white/12 rounded-2xl shadow-2xl z-[100] p-4 text-white/90 select-none animate-in fade-in zoom-in-95 duration-150`}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Custos Resource & RAM Monitor"
      >
        {/* Header */}
        <div className="flex items-center justify-between pb-3 border-b border-white/8">
          <div className="flex items-center gap-2">
            <div className="size-7 rounded-lg bg-white/10 flex items-center justify-center p-1.5 shrink-0 text-emerald-400">
              <Activity className="size-4" />
            </div>
            <div>
              <h2 className="text-[14px] font-semibold text-white tracking-tight">Custos RAM & Resources</h2>
              <p className="text-[10.5px] text-white/40">Real host process telemetry</p>
            </div>
          </div>
          <button
            type="button"
            onClick={handleRefresh}
            className="p-1.5 rounded-lg text-white/50 hover:text-white hover:bg-white/10 transition cursor-pointer"
            title="Refresh memory metrics"
            aria-label="Refresh memory metrics"
          >
            <RotateCw className={`size-3.5 ${isRefreshing ? 'animate-spin' : ''}`} />
          </button>
        </div>

        {/* Overview Stats */}
        <div className="grid grid-cols-2 gap-2 my-3">
          {/* Resident Memory Card */}
          <div className="p-2.5 rounded-xl bg-white/[0.04] border border-white/8">
            <div className="flex items-center justify-between text-[11px] text-white/50 mb-1">
              <span className="flex items-center gap-1">
                <HardDrive className="size-3 text-emerald-400" />
                Custos Memory
              </span>
              {memoryPct && <span className="font-mono text-[10px] text-white/40">{memoryPct}%</span>}
            </div>
            <div className="flex items-baseline gap-1.5">
              <span className="font-mono text-lg font-semibold text-white">
                {Math.round(memoryResidentMb)} <span className="text-xs font-normal text-white/60">MB</span>
              </span>
              {totalRamGb && (
                <span className="text-[10.5px] text-white/40 font-mono">
                  / {totalRamGb} GB
                </span>
              )}
            </div>
          </div>

          {/* Active Workload Card */}
          <div className="p-2.5 rounded-xl bg-white/[0.04] border border-white/8">
            <div className="flex items-center justify-between text-[11px] text-white/50 mb-1">
              <span className="flex items-center gap-1">
                <Cpu className="size-3 text-cyan-400" />
                Workload
              </span>
              {caffeinateActive && (
                <span className="flex items-center gap-0.5 text-[10px] text-amber-300">
                  <Coffee className="size-2.5" />
                  Awake
                </span>
              )}
            </div>
            <div className="flex items-baseline gap-1.5">
              <span className="font-mono text-lg font-semibold text-white">
                {activeRuns} <span className="text-xs font-normal text-white/60">run{activeRuns !== 1 ? 's' : ''}</span>
              </span>
              <span className="text-[10.5px] text-white/40 font-mono">
                {processGroups.length} proc
              </span>
            </div>
          </div>
        </div>

        {/* Process Breakdown List (Heaviest first, matching Orca/OpenScience) */}
        <div className="space-y-1.5">
          <div className="flex items-center justify-between text-[11px] font-medium text-white/50 px-1 pt-1">
            <span>Process Breakdown</span>
            <span>RSS / CPU</span>
          </div>

          <div className="space-y-1.5 max-h-[180px] overflow-y-auto pr-0.5 scrollbar-none">
            {processGroups.length > 0 ? (
              processGroups.map((proc) => {
                const isDaemon = proc.kind === 'daemon';
                const isApp = proc.kind === 'app';
                const Icon = isDaemon ? Server : isApp ? AppWindow : Terminal;

                return (
                  <div
                    key={`${proc.id}-${proc.pid}`}
                    className="flex items-center justify-between p-2 rounded-xl bg-white/[0.02] border border-white/5 hover:bg-white/[0.06] transition"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <div className="size-6 rounded-md bg-white/8 flex items-center justify-center p-1 shrink-0 text-white/70">
                        <Icon className="size-3.5" />
                      </div>
                      <div className="min-w-0">
                        <div className="text-[12px] font-medium text-white truncate max-w-[170px]">
                          {proc.name}
                        </div>
                        <div className="text-[10px] font-mono text-white/35">
                          PID {proc.pid}
                        </div>
                      </div>
                    </div>

                    <div className="text-right shrink-0">
                      <div className="font-mono text-[12px] font-medium text-white tabular-nums">
                        {proc.memory_mb} MB
                      </div>
                      <div className="font-mono text-[10px] text-white/40 tabular-nums">
                        {proc.cpu_percent > 0.5 ? `${proc.cpu_percent.toFixed(1)}% CPU` : '<1% CPU'}
                      </div>
                    </div>
                  </div>
                );
              })
            ) : (
              <div className="p-3 rounded-xl bg-white/[0.02] text-center text-xs text-white/40 font-mono">
                Single-process mode (PID active)
              </div>
            )}
          </div>
        </div>

        {/* Active Ports */}
        {activePorts.length > 0 && (
          <div className="mt-3 pt-2.5 border-t border-white/8 flex items-center justify-between text-[11px]">
            <span className="flex items-center gap-1.5 text-white/50">
              <Plug className="size-3 text-white/40" />
              Listening Ports
            </span>
            <div className="flex items-center gap-1 flex-wrap justify-end">
              {activePorts.slice(0, 4).map((port) => (
                <span
                  key={port}
                  className="px-1.5 py-0.5 rounded-md bg-white/6 border border-white/8 font-mono text-[10px] text-white/75"
                >
                  :{port}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Footer Note */}
        <p className="mt-3 pt-2 border-t border-white/6 text-[10px] text-white/35 text-center font-mono">
          Host Resident Set Size (RSS) sampled via OS process accounting.
        </p>
      </div>
    </>
  );
};
