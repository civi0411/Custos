import React, { useEffect, useState, useCallback } from 'react';
import {
  GitBranch,
  HardDrive,
  Coffee,
  AlertTriangle,
  Zap,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import { useJobRegistry } from '@/hooks/useJobRegistry';
import { OpenAIIcon, AntigravityIcon, ClaudeIcon, GeminiIcon } from '@/components/common/AgentIcons';
import { UsagePopover } from './UsagePopover';
import { ResourceUsagePopover } from './ResourceUsagePopover';
import type { ProviderQuotaSnapshot } from '@/types/provider_types';

interface SystemProcessMetric {
  id: string;
  name: string;
  kind: string;
  pid: number;
  memory_mb: number;
  cpu_percent: number;
}

interface SystemMetrics {
  memory_resident_mb: number;
  system_total_memory_mb?: number | null;
  process_groups?: SystemProcessMetric[];
  active_runs: number;
  active_ports: number[];
  caffeinate?: {
    mode: 'off' | 'agent' | 'always';
    active: boolean;
  };
}

interface SessionMetrics {
  total_tokens: number;
  estimated_cost_usd: number;
  active_runs: number;
}

export const AppStatusBar: React.FC = () => {
  const {
    activeWorkspace,
    activeWorkbench,
    activeSession,
    activeSessionId,
  } = useAppContext();

  const { activeCount: runningJobsCount } = useJobRegistry();

  const [isDaemonOnline, setIsDaemonOnline] = useState(true);
  const [systemMetrics, setSystemMetrics] = useState<SystemMetrics | null>(null);
  const [sessionMetrics, setSessionMetrics] = useState<SessionMetrics | null>(null);
  const [quotas, setQuotas] = useState<ProviderQuotaSnapshot[]>([]);
  const [isUsageOpen, setIsUsageOpen] = useState(false);
  const [isResourceOpen, setIsResourceOpen] = useState(false);

  const fetchStatus = useCallback(async () => {
    try {
      const online = await daemonClient.checkHealth();
      setIsDaemonOnline(online);
      if (online) {
        const [sys, sess, usage] = await Promise.all([
          daemonClient.getSystemMetrics().catch(() => null),
          daemonClient.getSessionMetrics(activeSessionId || activeSession?.id).catch(() => null),
          daemonClient.getProviderUsage(true).catch(() => null),
        ]);
        if (sys) setSystemMetrics(sys);
        if (sess) setSessionMetrics(sess);
        if (usage && usage.quotas) setQuotas(usage.quotas);
      }
    } catch {
      setIsDaemonOnline(false);
    }
  }, [activeSessionId, activeSession?.id]);

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 10000);
    return () => clearInterval(interval);
  }, [fetchStatus]);

  // Derive active quotas and percentages
  const availableQuotas = quotas.filter((q) => q.status === 'available' && q.windows.length > 0);

  // Check if ANY quota is exhausted or near limit
  let hasExhaustedQuota = false;
  let hasNearLimitQuota = false;
  for (const q of availableQuotas) {
    for (const w of q.windows) {
      if (w.used_percent >= 100) hasExhaustedQuota = true;
      else if (w.used_percent >= 90) hasNearLimitQuota = true;
    }
  }

  // Find OpenAI quota
  const openaiQuota = availableQuotas.find(
    (q) => q.account_id.includes('openai') || q.account_id.includes('codex')
  );
  let openaiUsedPercent: number | null = null;
  if (openaiQuota && openaiQuota.windows.length > 0) {
    const activeWindow = openaiQuota.windows.find((w) => w.window_kind === 'weekly') || openaiQuota.windows[0];
    openaiUsedPercent = Math.round(activeWindow.used_percent);
  }

  // Find Antigravity quota
  const antigravityQuota = availableQuotas.find((q) => q.account_id.includes('antigravity'));
  let antigravityUsedPercent: number | null = null;
  if (antigravityQuota && antigravityQuota.windows.length > 0) {
    const activeWindow = antigravityQuota.windows.find((w) => w.name === '3p-weekly') || antigravityQuota.windows[0];
    antigravityUsedPercent = Math.round(activeWindow.used_percent);
  }

  // Find Claude quota
  const claudeQuota = availableQuotas.find((q) => q.account_id.includes('claude') || q.account_id.includes('anthropic'));
  let claudeUsedPercent: number | null = null;
  if (claudeQuota && claudeQuota.windows.length > 0) {
    claudeUsedPercent = Math.round(claudeQuota.windows[0].used_percent);
  }

  // Find Gemini quota
  const geminiQuota = availableQuotas.find((q) => q.account_id.includes('gemini') || q.account_id.includes('google'));
  let geminiUsedPercent: number | null = null;
  if (geminiQuota && geminiQuota.windows.length > 0) {
    geminiUsedPercent = Math.round(geminiQuota.windows[0].used_percent);
  }

  const memoryMb = systemMetrics?.memory_resident_mb ?? 0;
  const activeRuns = (systemMetrics?.active_runs ?? 0) + runningJobsCount;
  const totalTokens = sessionMetrics?.total_tokens ?? 0;
  const costUsd = sessionMetrics?.estimated_cost_usd ?? 0;
  const isCaffeinated = systemMetrics?.caffeinate?.active;

  return (
    <footer
      className="h-6 w-full shrink-0 flex items-center justify-between px-3 bg-[#141518] border-t border-white/6 text-white/60 text-[11px] font-mono select-none z-40"
      aria-label="Application Status Bar"
    >
      {/* ── LEFT: Daemon Connection + Workspace/Branch + Lens ── */}
      <div className="flex items-center gap-3 min-w-0">
        {/* Daemon Connection Indicator */}
        <div
          className="flex items-center gap-1.5 transition-colors cursor-default"
          title={isDaemonOnline ? 'Daemon: Connected (Local inference & workspace sidecar)' : 'Daemon: Offline'}
        >
          {isDaemonOnline ? (
            <span className="size-1.5 rounded-full bg-emerald-400" />
          ) : (
            <span className="size-1.5 rounded-full bg-rose-400 animate-pulse" />
          )}
          <span className={`text-[10.5px] ${isDaemonOnline ? 'text-white/80' : 'text-rose-400 font-medium'}`}>
            {isDaemonOnline ? 'Daemon Online' : 'Daemon Offline'}
          </span>
        </div>

        <span className="text-white/20">|</span>

        {/* Active Workspace / Git Branch */}
        <div
          className="flex items-center gap-1.5 text-white/70 truncate max-w-[240px]"
          title={activeWorkspace?.path || activeWorkspace?.name || 'Workspace'}
        >
          <GitBranch className="size-3 text-white/50 shrink-0" />
          <span className="truncate text-[10.5px]">
            {activeWorkspace ? (
              <>
                <span className="text-white/85 font-medium">{activeWorkspace.name}</span>
                {(activeWorkspace.active_branch || (activeWorkspace.kind?.type === 'git' ? activeWorkspace.kind.branch : undefined)) && (
                  <span className="text-white/50 ml-1">· {activeWorkspace.active_branch || (activeWorkspace.kind?.type === 'git' ? activeWorkspace.kind.branch : '')}</span>
                )}
              </>
            ) : (
              'AgentHub'
            )}
          </span>
        </div>

        <span className="text-white/20 hidden sm:inline">|</span>

        {/* Workbench Lens Indicator */}
        <div className="hidden sm:flex items-center gap-1 text-white/50">
          <span className="capitalize text-[10.5px] text-workbench-accent font-medium">
            {activeWorkbench}
          </span>
        </div>
      </div>

      {/* ── CENTER: Active Runs / Jobs + Tokens Used & Cost ── */}
      <div className="flex items-center gap-3">
        {activeRuns > 0 ? (
          <div className="flex items-center gap-1.5 text-white/85">
            <span className="size-1.5 rounded-full bg-cyan-400 animate-pulse" />
            <span className="text-[10.5px]">{activeRuns} active run{activeRuns > 1 ? 's' : ''}</span>
          </div>
        ) : (
          <div className="flex items-center gap-1 text-white/40">
            <span className="text-[10.5px]">Idle</span>
          </div>
        )}

        {totalTokens > 0 && (
          <>
            <span className="text-white/20">·</span>
            <div className="hidden md:flex items-center gap-1 text-white/60">
              <span className="text-[10.5px]">
                {totalTokens >= 1000 ? `${(totalTokens / 1000).toFixed(1)}k` : totalTokens} tokens
              </span>
              {costUsd > 0 && (
                <span className="text-white/40 text-[10px]">(${costUsd.toFixed(3)})</span>
              )}
            </div>
          </>
        )}
      </div>

      {/* ── RIGHT: RAM / Memory Resident + Quotas + Host Caffeinate ── */}
      <div className="flex items-center gap-2.5">
        {/* Interactive Custos Memory Resident Usage (like Orca / OpenScience) */}
        <div className="relative">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setIsResourceOpen(!isResourceOpen);
              setIsUsageOpen(false);
            }}
            className="flex items-center gap-1.5 px-1.5 py-0.5 rounded-md hover:bg-white/10 transition cursor-pointer text-white/70 hover:text-white"
            title="Custos RAM & Process Monitor (Click to inspect)"
            aria-label="Inspect RAM and system resources"
          >
            <HardDrive className={`size-3 ${memoryMb > 1000 ? 'text-amber-400' : 'text-emerald-400'}`} />
            <span className="text-[10.5px] tabular-nums font-mono">
              {memoryMb > 0 ? `RAM: ${Math.round(memoryMb)} MB` : 'RAM: active'}
            </span>
          </button>

          <ResourceUsagePopover
            isOpen={isResourceOpen}
            onClose={() => setIsResourceOpen(false)}
            onRefresh={fetchStatus}
            memoryResidentMb={memoryMb}
            systemTotalMemoryMb={systemMetrics?.system_total_memory_mb}
            processGroups={systemMetrics?.process_groups}
            activePorts={systemMetrics?.active_ports}
            activeRuns={activeRuns}
            caffeinateActive={Boolean(isCaffeinated)}
            position="top"
            align="right"
          />
        </div>

        <span className="text-white/15">·</span>

        {/* Provider Quota Popover Toggle (Shows Exhaustion, Remaining Quota & Limits) */}
        <div className="relative">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setIsUsageOpen(!isUsageOpen);
              setIsResourceOpen(false);
            }}
            className={`flex items-center gap-1.5 px-1.5 py-0.5 rounded-md transition cursor-pointer ${
              hasExhaustedQuota
                ? 'bg-rose-500/15 text-rose-300 hover:bg-rose-500/25 border border-rose-500/30'
                : hasNearLimitQuota
                  ? 'bg-amber-500/15 text-amber-300 hover:bg-amber-500/25 border border-amber-500/30'
                  : 'hover:bg-white/10 text-white/70 hover:text-white'
            }`}
            title="Provider Rate Limits & Remaining Quota (Click to view details)"
            aria-label="View rate limits and quota"
          >
            {hasExhaustedQuota ? (
              <span className="flex items-center gap-1 text-[10px] font-semibold text-rose-400">
                <AlertTriangle className="size-2.5 shrink-0" />
                <span>Quota Depleted</span>
              </span>
            ) : availableQuotas.length > 0 ? (
              <span className="flex items-center gap-1.5 text-[10px]">
                {antigravityUsedPercent !== null && (
                  <span className="flex items-center gap-0.5" title={`Antigravity: ${Math.max(0, 100 - antigravityUsedPercent)}% left`}>
                    <AntigravityIcon className="size-2.5" />
                    <span className={antigravityUsedPercent >= 90 ? 'text-rose-400 font-semibold' : ''}>
                      {Math.max(0, 100 - antigravityUsedPercent)}%
                    </span>
                  </span>
                )}
                {openaiUsedPercent !== null && (
                  <span className="flex items-center gap-0.5" title={`OpenAI: ${Math.max(0, 100 - openaiUsedPercent)}% left`}>
                    <OpenAIIcon className="size-2.5 text-white/80" />
                    <span className={openaiUsedPercent >= 90 ? 'text-rose-400 font-semibold' : ''}>
                      {Math.max(0, 100 - openaiUsedPercent)}%
                    </span>
                  </span>
                )}
                {claudeUsedPercent !== null && (
                  <span className="flex items-center gap-0.5" title={`Claude: ${Math.max(0, 100 - claudeUsedPercent)}% left`}>
                    <ClaudeIcon className="size-2.5 text-white/80" />
                    <span>{Math.max(0, 100 - claudeUsedPercent)}%</span>
                  </span>
                )}
                {geminiUsedPercent !== null && (
                  <span className="flex items-center gap-0.5" title={`Gemini: ${Math.max(0, 100 - geminiUsedPercent)}% left`}>
                    <GeminiIcon className="size-2.5 text-white/80" />
                    <span>{Math.max(0, 100 - geminiUsedPercent)}%</span>
                  </span>
                )}
                <span className="text-white/35 text-[9px]">left</span>
              </span>
            ) : (
              <span className="flex items-center gap-1 text-[10px] text-white/50">
                <Zap className="size-2.5 text-white/40" />
                <span>Quotas</span>
              </span>
            )}
          </button>

          <UsagePopover
            isOpen={isUsageOpen}
            onClose={() => setIsUsageOpen(false)}
            onRefresh={fetchStatus}
            quotas={quotas}
            position="top"
            align="right"
          />
        </div>

        {/* Caffeinate Host Sleep Prevention */}
        {isCaffeinated && (
          <>
            <span className="text-white/15 hidden lg:inline">·</span>
            <div
              className="hidden lg:flex items-center gap-1 text-white/60"
              title="Host sleep prevention active during agent run"
            >
              <Coffee className="size-3 text-amber-300" />
              <span className="text-[10px] text-amber-300/80">Awake</span>
            </div>
          </>
        )}
      </div>
    </footer>
  );
};
