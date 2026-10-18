import React, { useEffect, useState, useCallback } from 'react';
import {
  GitBranch,
  HardDrive,
  Coffee,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import { useJobRegistry } from '@/hooks/useJobRegistry';
import { OpenAIIcon, AntigravityIcon } from '@/components/common/AgentIcons';
import { UsagePopover } from './UsagePopover';
import type { ProviderQuotaSnapshot } from '@/types/provider_types';

interface SystemMetrics {
  memory_resident_mb: number;
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

  // Derive quota percentages
  const openaiQuota = quotas.find(
    (q) => q.status === 'available' && (q.account_id.includes('openai') || q.account_id.includes('codex'))
  );
  const antigravityQuota = quotas.find(
    (q) => q.status === 'available' && q.account_id.includes('antigravity')
  );

  let openaiPercent: number | null = null;
  if (openaiQuota && openaiQuota.windows.length > 0) {
    const activeWindow = openaiQuota.windows.find((w) => w.window_kind === 'weekly') || openaiQuota.windows[0];
    openaiPercent = Math.round(activeWindow.used_percent);
  }

  let antigravityPercent: number | null = null;
  if (antigravityQuota && antigravityQuota.windows.length > 0) {
    const activeWindow = antigravityQuota.windows.find((w) => w.name === '3p-weekly') || antigravityQuota.windows[0];
    antigravityPercent = Math.round(activeWindow.used_percent);
  }

  const memoryMb = systemMetrics?.memory_resident_mb ?? 0;
  const activeRuns = (systemMetrics?.active_runs ?? 0) + runningJobsCount;
  const totalTokens = sessionMetrics?.total_tokens ?? 0;
  const costUsd = sessionMetrics?.estimated_cost_usd ?? 0;
  const isCaffeinated = systemMetrics?.caffeinate?.active;

  return (
    <footer
      className="h-6 w-full shrink-0 flex items-center justify-between px-3 bg-[#18181b] text-white/60 text-[11px] font-mono select-none z-40"
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
            <span className="size-1.5 rounded-full bg-white/70" />
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
            <span className="size-1.5 rounded-full bg-white/70 animate-pulse" />
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
      <div className="flex items-center gap-3">
        {/* Memory Resident Usage (like Orca) */}
        <div
          className="flex items-center gap-1.5 text-white/70 hover:text-white transition-colors cursor-default"
          title={`Resident Set Size (RAM): ${memoryMb > 0 ? `${memoryMb} MB` : 'Monitoring...'}`}
        >
          <HardDrive className="size-3 text-white/50" />
          <span className="text-[10.5px]">
            {memoryMb > 0 ? `RAM: ${Math.round(memoryMb)} MB` : 'RAM: active'}
          </span>
        </div>

        {/* Provider Quota Popover Toggle */}
        {(openaiPercent !== null || antigravityPercent !== null) && (
          <div className="relative">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setIsUsageOpen(!isUsageOpen);
              }}
              className="flex items-center gap-2 px-1.5 py-0.5 rounded hover:bg-white/10 transition cursor-pointer text-white/70 hover:text-white"
              title="Provider Rate Limits & Quotas"
              aria-label="View rate limits"
            >
              {openaiPercent !== null && (
                <span className="flex items-center gap-1 text-[10px]">
                  <OpenAIIcon className="size-2.5 text-white/80" />
                  <span>{openaiPercent}%</span>
                </span>
              )}
              {antigravityPercent !== null && (
                <span className="flex items-center gap-1 text-[10px]">
                  <AntigravityIcon className="size-2.5" />
                  <span className={antigravityPercent >= 90 ? 'text-rose-400 font-semibold' : ''}>
                    {antigravityPercent}%
                  </span>
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
        )}

        {/* Caffeinate Host Sleep Prevention */}
        {isCaffeinated && (
          <div
            className="hidden lg:flex items-center gap-1 text-white/60"
            title="Host sleep prevention active during agent run"
          >
            <Coffee className="size-3 text-white/50" />
            <span className="text-[10px]">Awake</span>
          </div>
        )}
      </div>
    </footer>
  );
};
