import React, { useState, useEffect } from 'react';
import { RotateCw, ChevronRight, AlertTriangle, Clock, Coins } from 'lucide-react';
import { OpenAIIcon, AntigravityIcon, ClaudeIcon, GeminiIcon } from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';
import type { ProviderQuotaSnapshot } from '@/types/provider_types';

export interface UsagePopoverProps {
  isOpen: boolean;
  onClose: () => void;
  onRefresh?: () => void;
  quotas?: ProviderQuotaSnapshot[];
  totalTokens?: number;
  costUsd?: number;
  position?: 'top' | 'bottom';
  align?: 'left' | 'right' | 'center';
}

const MODE_KEY = 'custos.usage.mode.v1';
const DISPLAY_KEY = 'custos.usage.display.v1';

function formatResetDuration(reset: string | number | null | undefined): string | null {
  if (!reset) return null;
  try {
    let resetDate: Date;
    if (typeof reset === 'number') {
      const ms = reset < 10000000000 ? reset * 1000 : reset;
      resetDate = new Date(ms);
    } else {
      resetDate = new Date(reset);
    }
    const diffMs = resetDate.getTime() - Date.now();
    if (diffMs <= 0) return 'Resetting soon';
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays > 0) {
      return `${diffDays}d ${diffHours % 24}h left`;
    }
    if (diffHours > 0) {
      return `${diffHours}h ${diffMins % 60}m left`;
    }
    return `${diffMins}m left`;
  } catch {
    return null;
  }
}

export const UsagePopover: React.FC<UsagePopoverProps> = ({
  isOpen,
  onClose,
  onRefresh,
  quotas = [],
  totalTokens = 0,
  costUsd = 0,
  position = 'top',
  align = 'right',
}) => {
  const { openAiSetup, openSettings } = useAppContext();
  const [viewMode, setViewMode] = useState<'detailed' | 'compact'>(() => {
    try {
      return window.localStorage.getItem(MODE_KEY) === 'compact' ? 'compact' : 'detailed';
    } catch {
      return 'detailed';
    }
  });

  const [displayMode, setDisplayMode] = useState<'remaining' | 'used'>(() => {
    try {
      return window.localStorage.getItem(DISPLAY_KEY) === 'used' ? 'used' : 'remaining';
    } catch {
      return 'remaining';
    }
  });

  const [isRefreshing, setIsRefreshing] = useState(false);
  const [, setTick] = useState(0);

  // Minute countdown tick (Orca useResetCountdownClock)
  useEffect(() => {
    if (!isOpen) return;
    const timer = setInterval(() => setTick((t) => t + 1), 60000);
    return () => clearInterval(timer);
  }, [isOpen]);

  if (!isOpen) return null;

  const handleRefresh = (e: React.MouseEvent) => {
    e.stopPropagation();
    setIsRefreshing(true);
    onRefresh?.();
    setTimeout(() => setIsRefreshing(false), 600);
  };

  const handleModeChange = (next: 'detailed' | 'compact') => {
    setViewMode(next);
    try {
      window.localStorage.setItem(MODE_KEY, next);
    } catch {
      // Ignore
    }
  };

  const handleDisplayChange = (next: 'remaining' | 'used') => {
    setDisplayMode(next);
    try {
      window.localStorage.setItem(DISPLAY_KEY, next);
    } catch {
      // Ignore
    }
  };

  const handleOpenQuotas = (e: React.MouseEvent) => {
    e.stopPropagation();
    onClose();
    openSettings('quotas');
  };

  const handleOpenAccounts = (e: React.MouseEvent) => {
    e.stopPropagation();
    onClose();
    openAiSetup('cloud');
  };

  // Filter available provider quotas
  const activeQuotas = quotas.filter((q) => q.status === 'available' && q.windows.length > 0);

  // Group quotas by provider type
  const providerGroups: {
    key: string;
    name: string;
    tier: string;
    icon: React.ElementType;
    quota: ProviderQuotaSnapshot;
  }[] = [];

  const seenProviders = new Set<string>();
  for (const q of activeQuotas) {
    const id = q.account_id.toLowerCase();
    let key = 'other';
    let name = 'Provider';
    let tier = 'API Plan';
    let icon = OpenAIIcon;

    if (id.includes('antigravity')) {
      key = 'antigravity';
      name = 'Antigravity';
      tier = 'Multi-Model Pool';
      icon = AntigravityIcon;
    } else if (id.includes('openai') || id.includes('codex')) {
      key = 'openai';
      name = 'OpenAI';
      tier = 'Tier Account';
      icon = OpenAIIcon;
    } else if (id.includes('claude') || id.includes('anthropic')) {
      key = 'claude';
      name = 'Anthropic Claude';
      tier = 'Build / Max Plan';
      icon = ClaudeIcon;
    } else if (id.includes('gemini') || id.includes('google')) {
      key = 'gemini';
      name = 'Google Gemini';
      tier = 'Vertex / Studio';
      icon = GeminiIcon;
    }

    if (!seenProviders.has(key)) {
      seenProviders.add(key);
      providerGroups.push({ key, name, tier, icon, quota: q });
    }
  }

  // Check overall exhaustion
  let hasExhaustedAny = false;
  let hasNearLimitAny = false;
  for (const q of activeQuotas) {
    for (const w of q.windows) {
      if (w.used_percent >= 100) hasExhaustedAny = true;
      else if (w.used_percent >= 90) hasNearLimitAny = true;
    }
  }

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

      {/* Floating Usage Card */}
      <div
        className={`${positionClasses} w-[370px] sm:w-[420px] bg-[#141518] border border-white/12 rounded-2xl shadow-2xl z-[100] p-4 text-white/90 select-none animate-in fade-in zoom-in-95 duration-150`}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Provider Quotas & Rate Limits"
      >
        {/* Header */}
        <div className="flex items-center justify-between pb-3 border-b border-white/8">
          <div>
            <h2 className="text-[14px] font-semibold tracking-tight text-white flex items-center gap-1.5">
              <span>Rate Limits & Vendor Quotas</span>
            </h2>
            <p className="text-[10.5px] text-white/40">Real allowance tracking & reset schedules</p>
          </div>
          <div className="flex items-center gap-1.5">
            {/* Display Mode Toggle: % Left vs % Used (Orca / OpenScience) */}
            <div className="flex items-center p-0.5 rounded-lg bg-black/40 border border-white/8 text-[10px] font-mono">
              <button
                type="button"
                onClick={() => handleDisplayChange('remaining')}
                className={`px-2 py-0.5 rounded transition cursor-pointer ${
                  displayMode === 'remaining'
                    ? 'bg-emerald-500/20 text-emerald-300 font-semibold shadow-xs'
                    : 'text-white/40 hover:text-white/70'
                }`}
                title="Show remaining allowance percentage"
              >
                % Left
              </button>
              <button
                type="button"
                onClick={() => handleDisplayChange('used')}
                className={`px-2 py-0.5 rounded transition cursor-pointer ${
                  displayMode === 'used'
                    ? 'bg-white/15 text-white font-semibold shadow-xs'
                    : 'text-white/40 hover:text-white/70'
                }`}
                title="Show consumed quota percentage"
              >
                % Used
              </button>
            </div>

            <button
              type="button"
              onClick={handleRefresh}
              className="p-1.5 rounded-lg text-white/50 hover:text-white hover:bg-white/10 transition cursor-pointer"
              title="Refresh quota & limits"
              aria-label="Refresh usage data"
            >
              <RotateCw className={`size-3.5 ${isRefreshing ? 'animate-spin' : ''}`} />
            </button>
          </div>
        </div>

        {/* Status Alert Banner if Quota is Depleted or Near Limit */}
        {hasExhaustedAny ? (
          <div className="my-2.5 p-2 rounded-xl bg-rose-500/15 border border-rose-500/30 flex items-start gap-2 text-rose-300 text-[11px]">
            <AlertTriangle className="size-3.5 shrink-0 text-rose-400 mt-0.5" />
            <div>
              <span className="font-semibold text-rose-200">Quota Depleted: </span>
              <span>1 or more vendor rate windows have reached 100%. Throttling or model fallbacks may apply.</span>
            </div>
          </div>
        ) : hasNearLimitAny ? (
          <div className="my-2.5 p-2 rounded-xl bg-amber-500/15 border border-amber-500/30 flex items-start gap-2 text-amber-200 text-[11px]">
            <AlertTriangle className="size-3.5 shrink-0 text-amber-400 mt-0.5" />
            <div>
              <span className="font-semibold">Approaching Limit: </span>
              <span>Active window has exceeded 90% consumption.</span>
            </div>
          </div>
        ) : null}

        {/* Session Spend & Token Summary (OpenScience / Orca SpendMetrics) */}
        {totalTokens > 0 && (
          <div className="my-2 p-2 rounded-xl bg-white/[0.03] border border-white/6 flex items-center justify-between text-[11px]">
            <span className="flex items-center gap-1.5 text-white/50">
              <Coins className="size-3.5 text-amber-300/80" />
              <span>Session Spend</span>
            </span>
            <div className="flex items-center gap-2 font-mono">
              <span className="text-white/80">
                {totalTokens >= 1000 ? `${(totalTokens / 1000).toFixed(1)}k` : totalTokens} tokens
              </span>
              {costUsd > 0 && (
                <span className="text-emerald-400 font-medium">
                  (${costUsd.toFixed(3)})
                </span>
              )}
            </div>
          </div>
        )}

        {/* View Mode Switch: Detailed Windows vs Compact Roster */}
        <div className="flex items-center p-0.5 rounded-xl bg-black/40 border border-white/8 my-2.5">
          <button
            type="button"
            onClick={() => handleModeChange('detailed')}
            className={`flex-1 py-1 text-xs font-medium rounded-lg transition cursor-pointer text-center ${
              viewMode === 'detailed'
                ? 'bg-white/12 text-white shadow-xs'
                : 'text-white/40 hover:text-white/70'
            }`}
          >
            Detailed Windows
          </button>
          <button
            type="button"
            onClick={() => handleModeChange('compact')}
            className={`flex-1 py-1 text-xs font-medium rounded-lg transition cursor-pointer text-center ${
              viewMode === 'compact'
                ? 'bg-white/12 text-white shadow-xs'
                : 'text-white/40 hover:text-white/70'
            }`}
          >
            Compact Roster
          </button>
        </div>

        {/* Dynamic Provider Cards from live quotas */}
        {providerGroups.length > 0 ? (
          <div className="space-y-2.5 max-h-[320px] overflow-y-auto scrollbar-none pr-0.5">
            {providerGroups.map(({ key, name, tier, icon: Icon, quota }) => {
              const resetWindow = quota.windows.find((w) => w.resets_at);
              const resetStr = resetWindow ? formatResetDuration(resetWindow.resets_at) : null;

              // Check max used across all windows
              const maxUsed = Math.max(...quota.windows.map((w) => w.used_percent || 0));
              const isExhausted = maxUsed >= 100;
              const isCritical = maxUsed >= 90 && maxUsed < 100;
              const isWarning = maxUsed >= 75 && maxUsed < 90;

              return (
                <div key={key} className="rounded-xl bg-white/[0.03] border border-white/6 p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <div className="size-6 rounded-md bg-white/10 flex items-center justify-center p-1 shrink-0">
                        <Icon className="size-3.5 text-white/90" />
                      </div>
                      <div>
                        <div className="text-xs font-semibold text-white leading-tight">{name}</div>
                        <div className="text-[9.5px] text-white/35">{tier}</div>
                      </div>
                    </div>

                    <div className="flex items-center gap-1.5">
                      {isExhausted ? (
                        <span className="inline-flex items-center gap-1 text-[9.5px] font-semibold text-rose-400 bg-rose-500/15 border border-rose-500/30 px-1.5 py-0.5 rounded">
                          <AlertTriangle className="size-2.5 shrink-0" />
                          Depleted
                        </span>
                      ) : isCritical ? (
                        <span className="inline-flex items-center gap-1 text-[9.5px] font-semibold text-rose-300 bg-rose-500/10 border border-rose-500/20 px-1.5 py-0.5 rounded">
                          Near Limit
                        </span>
                      ) : isWarning ? (
                        <span className="text-[9.5px] font-medium text-amber-300 bg-amber-500/10 border border-amber-500/20 px-1.5 py-0.5 rounded">
                          Warning
                        </span>
                      ) : (
                        <span className="text-[9.5px] font-medium text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-1.5 py-0.5 rounded">
                          Available
                        </span>
                      )}

                      {resetStr && (
                        <span className="flex items-center gap-1 text-[10px] text-white/40 font-mono pl-1">
                          <Clock className="size-2.5 text-white/30" />
                          {resetStr}
                        </span>
                      )}
                    </div>
                  </div>

                  {viewMode === 'detailed' ? (
                    <div className="space-y-2 pt-1 text-[11.5px]">
                      {quota.windows.map((w, idx) => {
                        const usedPct = Math.min(100, Math.max(0, Math.round(w.used_percent)));
                        const remainingPct = Math.max(0, 100 - usedPct);

                        const isWinExhausted = usedPct >= 100;
                        const isWinHigh = usedPct >= 90;
                        const isWinMed = usedPct >= 70 && usedPct < 90;

                        // Human-friendly title
                        let title = w.name;
                        if (w.name === '3p-weekly') title = 'Claude & GPT · Weekly';
                        else if (w.name === '3p-5h') title = 'Claude & GPT · 5h Session';
                        else if (w.name === 'gemini-weekly') title = 'Gemini · Weekly';
                        else if (w.name === 'gemini-5h') title = 'Gemini · 5h Session';
                        else if (w.name === 'Session') title = '5-Hour Window';
                        else if (w.name === 'Weekly') title = 'Weekly Window';

                        return (
                          <div key={idx} className="flex items-center justify-between gap-2">
                            <span className="text-white/65 truncate text-[11px]">{title}</span>
                            <div className="flex items-center gap-2 shrink-0">
                              <div className="w-16 h-1.5 rounded-full bg-white/10 overflow-hidden">
                                <div
                                  className={`h-full rounded-full transition-all duration-300 ${
                                    isWinExhausted || isWinHigh
                                      ? 'bg-rose-500'
                                      : isWinMed
                                        ? 'bg-amber-400'
                                        : 'bg-emerald-400'
                                  }`}
                                  style={{ width: `${displayMode === 'remaining' ? remainingPct : usedPct}%` }}
                                />
                              </div>
                              <span
                                className={`font-mono text-[11px] w-14 text-right tabular-nums ${
                                  isWinExhausted
                                    ? 'text-rose-400 font-bold'
                                    : isWinHigh
                                      ? 'text-rose-400 font-medium'
                                      : isWinMed
                                        ? 'text-amber-300 font-medium'
                                        : 'text-white/80'
                                }`}
                              >
                                {displayMode === 'remaining' ? `${remainingPct}% left` : `${usedPct}%`}
                              </span>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  ) : (
                    <div className="grid grid-cols-2 gap-2 pt-1 text-[11px]">
                      {quota.windows.slice(0, 2).map((w, idx) => {
                        const usedPct = Math.min(100, Math.max(0, Math.round(w.used_percent)));
                        const remainingPct = Math.max(0, 100 - usedPct);
                        const isWinExhausted = usedPct >= 100;
                        const isWinHigh = usedPct >= 90;

                        let shortName = w.name.includes('5h') || w.name === 'Session' ? '5h Window' : 'Weekly';
                        if (w.name.includes('gemini')) shortName = 'Gemini';
                        if (w.name.includes('3p')) shortName = 'Claude/GPT';

                        return (
                          <div key={idx} className="flex items-center justify-between px-2 py-1.5 rounded-lg bg-black/25 border border-white/5">
                            <span className="text-white/50 truncate pr-1">{shortName}</span>
                            <span
                              className={`font-mono text-[11px] tabular-nums ${
                                isWinExhausted
                                  ? 'text-rose-400 font-bold'
                                  : isWinHigh
                                    ? 'text-rose-400 font-medium'
                                    : 'text-white/80'
                              }`}
                            >
                              {displayMode === 'remaining' ? `${remainingPct}% left` : `${usedPct}%`}
                            </span>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        ) : (
          <div className="py-6 px-4 text-center rounded-xl bg-white/[0.02] border border-white/6 my-2">
            <p className="text-xs text-white/60 mb-1">No vendor quotas reported</p>
            <p className="text-[11px] text-white/35">
              Connect a Claude, OpenAI, Antigravity or Gemini provider in Settings to track live rate limits.
            </p>
          </div>
        )}

        {/* Footer links */}
        <div className="pt-3 mt-3 border-t border-white/8 space-y-1">
          <button
            type="button"
            onClick={handleOpenQuotas}
            className="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-white/6 text-xs text-white/80 hover:text-white transition cursor-pointer"
          >
            <span>Quota analytics & cost limits</span>
            <ChevronRight className="size-3.5 text-white/40" />
          </button>
          <button
            type="button"
            onClick={handleOpenAccounts}
            className="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-white/6 text-xs text-white/80 hover:text-white transition cursor-pointer"
          >
            <span>Manage connected provider keys</span>
            <ChevronRight className="size-3.5 text-white/40" />
          </button>
        </div>
      </div>
    </>
  );
};
