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
  CheckCircle2,
  AlertTriangle,
  ExternalLink,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';

export interface ProcessMetric {
  id: string;
  name: string;
  command?: string;
  kind: string;
  pid: number;
  memory_mb: number;
  cpu_percent: number;
}

export interface ListeningPortInfo {
  port: number;
  pid?: number;
  command?: string;
  public?: boolean;
}

export interface ResourceUsagePopoverProps {
  isOpen: boolean;
  onClose: () => void;
  onRefresh?: () => void;
  memoryResidentMb: number;
  systemTotalMemoryMb?: number | null;
  processGroups?: ProcessMetric[];
  activePorts?: number[];
  listeningPorts?: ListeningPortInfo[];
  activeRuns?: number;
  caffeinateActive?: boolean;
  caffeinateMode?: 'off' | 'agent' | 'always';
  position?: 'top' | 'bottom';
  align?: 'left' | 'right' | 'center';
}

function formatBytesMb(mb: number): string {
  if (mb < 1024) return `${Math.round(mb)} MB`;
  return `${(mb / 1024).toFixed(2)} GB`;
}

export const ResourceUsagePopover: React.FC<ResourceUsagePopoverProps> = ({
  isOpen,
  onClose,
  onRefresh,
  memoryResidentMb,
  systemTotalMemoryMb,
  processGroups = [],
  activePorts = [],
  listeningPorts = [],
  activeRuns = 0,
  caffeinateActive = false,
  caffeinateMode = 'off',
  position = 'top',
  align = 'right',
}) => {
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [currentAwakeMode, setCurrentAwakeMode] = useState<'off' | 'agent' | 'always'>(caffeinateMode);
  const [isSwitchingAwake, setIsSwitchingAwake] = useState(false);

  if (!isOpen) return null;

  const handleRefresh = (e: React.MouseEvent) => {
    e.stopPropagation();
    setIsRefreshing(true);
    onRefresh?.();
    setTimeout(() => setIsRefreshing(false), 500);
  };

  const handleAwakeChange = async (mode: 'off' | 'agent' | 'always') => {
    try {
      setIsSwitchingAwake(true);
      setCurrentAwakeMode(mode);
      await daemonClient.setCaffeinate(mode);
      onRefresh?.();
    } catch {
      // Keep optimistic
    } finally {
      setIsSwitchingAwake(false);
    }
  };

  const totalRamGb = systemTotalMemoryMb ? (systemTotalMemoryMb / 1024).toFixed(0) : null;
  const memoryPct = systemTotalMemoryMb && systemTotalMemoryMb > 0
    ? ((memoryResidentMb / systemTotalMemoryMb) * 100).toFixed(1)
    : null;

  // Memory Pressure classification (Orca/OpenScience rule)
  // Green (< 500 MB): Healthy
  // Amber (500 MB - 1500 MB): Elevated
  // Red (> 1500 MB): High / Memory Leak risk
  const isHighMemory = memoryResidentMb >= 1500;
  const isModerateMemory = memoryResidentMb >= 500 && memoryResidentMb < 1500;

  // Combine activePorts and listeningPorts cleanly
  const resolvedPorts: ListeningPortInfo[] = listeningPorts.length > 0
    ? listeningPorts
    : activePorts.map((p) => ({ port: p, command: p === 3000 ? 'custos-daemon' : p === 1420 ? 'vite' : 'service', public: false }));

  const positionClasses =
    position === 'bottom'
      ? 'fixed bottom-10 right-4'
      : align === 'right'
        ? 'fixed bottom-8 right-6'
        : 'fixed bottom-8 left-12';

  const totalCpu = processGroups.reduce((acc, p) => acc + (p.cpu_percent || 0), 0);

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-[90] bg-black/40 backdrop-blur-[1px]"
        onClick={onClose}
      />

      {/* Floating Resource Usage Card */}
      <div
        className={`${positionClasses} w-[370px] sm:w-[420px] bg-[#141518] border border-white/12 rounded-2xl shadow-2xl z-[100] p-4 text-white/90 select-none animate-in fade-in zoom-in-95 duration-150`}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Custos Resource & RAM Monitor"
      >
        {/* Header */}
        <div className="flex items-center justify-between pb-3 border-b border-white/8">
          <div className="flex items-center gap-2">
            <div className="size-7 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center p-1.5 shrink-0 text-emerald-400">
              <Activity className="size-4" />
            </div>
            <div>
              <h2 className="text-[14px] font-semibold text-white tracking-tight">Custos RAM & Process Tree</h2>
              <p className="text-[10.5px] text-white/40">Scoped process accounting & host monitor</p>
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
                <HardDrive className={`size-3 ${isHighMemory ? 'text-rose-400' : isModerateMemory ? 'text-amber-400' : 'text-emerald-400'}`} />
                Custos Memory
              </span>
              {memoryPct && <span className="font-mono text-[10px] text-white/40">{memoryPct}% RAM</span>}
            </div>
            <div className="flex items-baseline gap-1.5">
              <span className="font-mono text-lg font-semibold text-white">
                {formatBytesMb(memoryResidentMb)}
              </span>
              {totalRamGb && (
                <span className="text-[10.5px] text-white/40 font-mono">
                  / {totalRamGb} GB
                </span>
              )}
            </div>
            {/* Health status pill */}
            <div className="mt-1.5 flex items-center gap-1 text-[10px]">
              {isHighMemory ? (
                <span className="flex items-center gap-1 text-rose-400 font-medium">
                  <AlertTriangle className="size-2.5" /> High footprint (check leaks)
                </span>
              ) : isModerateMemory ? (
                <span className="flex items-center gap-1 text-amber-300 font-medium">
                  Moderate (active jobs)
                </span>
              ) : (
                <span className="flex items-center gap-1 text-emerald-400 font-medium">
                  <CheckCircle2 className="size-2.5" /> Healthy & lean footprint
                </span>
              )}
            </div>
          </div>

          {/* Workload Card */}
          <div className="p-2.5 rounded-xl bg-white/[0.04] border border-white/8">
            <div className="flex items-center justify-between text-[11px] text-white/50 mb-1">
              <span className="flex items-center gap-1">
                <Cpu className="size-3 text-cyan-400" />
                Workload
              </span>
              <span className="font-mono text-[10px] text-white/40">
                {totalCpu >= 1 ? `${totalCpu.toFixed(1)}% CPU` : '<1% CPU'}
              </span>
            </div>
            <div className="flex items-baseline gap-1.5">
              <span className="font-mono text-lg font-semibold text-white">
                {activeRuns} <span className="text-xs font-normal text-white/60">run{activeRuns !== 1 ? 's' : ''}</span>
              </span>
              <span className="text-[10.5px] text-white/40 font-mono">
                {processGroups.length} proc
              </span>
            </div>
            <div className="mt-1.5 text-[10px] text-white/40 font-mono truncate">
              {caffeinateActive ? 'Machine awake active' : 'Standard power sleep'}
            </div>
          </div>
        </div>

        {/* Machine Awake (Caffeinate) Mode Switcher (Orca pattern) */}
        <div className="p-2.5 rounded-xl bg-white/[0.03] border border-white/6 my-2.5">
          <div className="flex items-center justify-between text-[11px] text-white/70 mb-2">
            <span className="flex items-center gap-1.5 font-medium">
              <Coffee className={`size-3.5 ${caffeinateActive ? 'text-amber-300' : 'text-white/40'}`} />
              Keep Machine Awake
            </span>
            <span className="text-[10px] text-white/40">
              {caffeinateActive ? 'Active (-i -s)' : 'Idle'}
            </span>
          </div>

          <div className="grid grid-cols-3 gap-1 p-0.5 rounded-lg bg-black/40 border border-white/8 text-[10.5px]">
            <button
              type="button"
              disabled={isSwitchingAwake}
              onClick={() => handleAwakeChange('off')}
              className={`py-1 rounded transition text-center cursor-pointer ${
                currentAwakeMode === 'off'
                  ? 'bg-white/15 text-white font-semibold shadow-xs'
                  : 'text-white/40 hover:text-white/70'
              }`}
            >
              Off
            </button>
            <button
              type="button"
              disabled={isSwitchingAwake}
              onClick={() => handleAwakeChange('agent')}
              className={`py-1 rounded transition text-center cursor-pointer ${
                currentAwakeMode === 'agent'
                  ? 'bg-amber-500/20 text-amber-300 font-semibold shadow-xs'
                  : 'text-white/40 hover:text-white/70'
              }`}
              title="Prevent sleep only while agent is executing"
            >
              Auto (Agent)
            </button>
            <button
              type="button"
              disabled={isSwitchingAwake}
              onClick={() => handleAwakeChange('always')}
              className={`py-1 rounded transition text-center cursor-pointer ${
                currentAwakeMode === 'always'
                  ? 'bg-amber-500/25 text-amber-300 font-semibold shadow-xs'
                  : 'text-white/40 hover:text-white/70'
              }`}
              title="Keep machine awake always while Custos is running"
            >
              Always
            </button>
          </div>
        </div>

        {/* Process Breakdown List (Heaviest first, matching Orca/OpenScience tree_of) */}
        <div className="space-y-1.5">
          <div className="flex items-center justify-between text-[11px] font-medium text-white/50 px-1 pt-1">
            <span>Process Tree (Heaviest First)</span>
            <span>RSS / CPU</span>
          </div>

          <div className="space-y-1.5 max-h-[160px] overflow-y-auto pr-0.5 scrollbar-none">
            {processGroups.length > 0 ? (
              processGroups.map((proc) => {
                const isDaemon = proc.kind === 'daemon';
                const isApp = proc.kind === 'app';
                const isTerminal = proc.kind === 'terminal';
                const Icon = isDaemon ? Server : isApp ? AppWindow : isTerminal ? Terminal : Cpu;

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
                        <div className="text-[12px] font-medium text-white truncate max-w-[180px]">
                          {proc.name}
                        </div>
                        <div className="text-[10px] font-mono text-white/35 flex items-center gap-1.5">
                          <span>PID {proc.pid}</span>
                          {proc.command && (
                            <span className="truncate max-w-[120px] text-white/25">· {proc.command.split('/').pop()}</span>
                          )}
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

        {/* Listening Ports (Orca/OpenScience PortsSegment) */}
        {resolvedPorts.length > 0 && (
          <div className="mt-3 pt-2.5 border-t border-white/8">
            <div className="flex items-center justify-between text-[11px] mb-1.5">
              <span className="flex items-center gap-1.5 text-white/50">
                <Plug className="size-3 text-cyan-400" />
                Custos Listening Ports
              </span>
              <span className="text-[10px] text-white/40 font-mono">
                {resolvedPorts.length} active
              </span>
            </div>

            <div className="flex items-center gap-1.5 flex-wrap">
              {resolvedPorts.map((p) => (
                <a
                  key={p.port}
                  href={`http://localhost:${p.port}`}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-white/6 hover:bg-white/12 border border-white/8 hover:border-white/20 font-mono text-[10.5px] text-white/80 hover:text-white transition cursor-pointer"
                  title={`Open http://localhost:${p.port} (${p.command || 'service'}) in browser`}
                >
                  <span>:{p.port}</span>
                  <span className="text-[9px] text-white/40">{p.command?.split('/').pop() || 'local'}</span>
                  {p.public && (
                    <span className="text-[8.5px] bg-amber-500/20 text-amber-300 px-1 rounded">public</span>
                  )}
                  <ExternalLink className="size-2.5 text-white/30" />
                </a>
              ))}
            </div>
          </div>
        )}

        {/* Footer Note */}
        <p className="mt-3 pt-2 border-t border-white/6 text-[10px] text-white/35 text-center font-mono">
          Scoped to Custos process tree (PID & child processes). Excludes unrelated apps.
        </p>
      </div>
    </>
  );
};
