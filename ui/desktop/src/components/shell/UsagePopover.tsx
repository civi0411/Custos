import React, { useState } from 'react';
import { RotateCw, ChevronRight, AlertTriangle, Clock } from 'lucide-react';
import { OpenAIIcon, AntigravityIcon, ClaudeIcon, GeminiIcon } from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';
import type { ProviderQuotaSnapshot } from '@/types/provider_types';

export interface UsagePopoverProps {
  isOpen: boolean;
  onClose: () => void;
  onRefresh?: () => void;
  quotas?: ProviderQuotaSnapshot[];
  position?: 'top' | 'bottom';
  align?: 'left' | 'right' | 'center';
}

function formatResetTime(reset: string | number | null | undefined): string | null {
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
  position = 'top',
  align = 'right',
}) => {
  const { openAiSetup, openSettings } = useAppContext();
  const [viewMode, setViewMode] = useState<'detailed' | 'compact'>('detailed');
  const [displayMode, setDisplayMode] = useState<'remaining' | 'used'>('remaining');
  const [isRefreshing, setIsRefreshing] = useState(false);

  if (!isOpen) return null;

  const handleRefresh = (e: React.MouseEvent) => {
    e.stopPropagation();
    setIsRefreshing(true);
    onRefresh?.();
    setTimeout(() => setIsRefreshing(false), 600);
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

  // Filter only available provider quotas
  const activeQuotas = quotas.filter((q) => q.status === 'available' && q.windows.length > 0);

  // Group quotas by provider type to avoid duplicating same provider with multiple tokens
  const providerGroups: {
    key: string;
    name: string;
    icon: React.ElementType;
    quota: ProviderQuotaSnapshot;
  }[] = [];

  const seenProviders = new Set<string>();
  for (const q of activeQuotas) {
    const id = q.account_id.toLowerCase();
    let key = 'other';
    let name = 'Provider';
    let icon = OpenAIIcon;

    if (id.includes('antigravity')) {
      key = 'antigravity';
      name = 'Antigravity';
      icon = AntigravityIcon;
    } else if (id.includes('openai') || id.includes('codex')) {
      key = 'openai';
      name = 'OpenAI';
      icon = OpenAIIcon;
    } else if (id.includes('claude') || id.includes('anthropic')) {
      key = 'claude';
      name = 'Anthropic Claude';
      icon = ClaudeIcon;
    } else if (id.includes('gemini') || id.includes('google')) {
      key = 'gemini';
      name = 'Google Gemini';
      icon = GeminiIcon;
    }

    if (!seenProviders.has(key)) {
      seenProviders.add(key);
      providerGroups.push({ key, name, icon, quota: q });
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
        className={`${positionClasses} w-[360px] sm:w-[410px] bg-[#141518] border border-white/12 rounded-2xl shadow-2xl z-[100] p-4 text-white/90 select-none animate-in fade-in zoom-in-95 duration-150`}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Provider Quotas & Rate Limits"
      >
        {/* Header */}
        <div className="flex items-center justify-between pb-3 border-b border-white/8">
          <div>
            <h2 className="text-[14px] font-semibold tracking-tight text-white">Rate Limits & Quota</h2>
            <p className="text-[10.5px] text-white/40">Real vendor allowance tracking</p>
          </div>
          <div className="flex items-center gap-1.5">
            {/* Display Mode Toggle: Remaining vs Used (Orca pattern) */}
            <div className="flex items-center p-0.5 rounded-lg bg-black/40 border border-white/8 text-[10px] font-mono">
              <button
                type="button"
                onClick={() => setDisplayMode('remaining')}
                className={`px-2 py-0.5 rounded transition cursor-pointer ${
                  displayMode === 'remaining'
                    ? 'bg-emerald-500/20 text-emerald-300 font-semibold'
                    : 'text-white/40 hover:text-white/70'
                }`}
                title="Show quota remaining"
              >
                % Left
              </button>
              <button
                type="button"
                onClick={() => setDisplayMode('used')}
                className={`px-2 py-0.5 rounded transition cursor-pointer ${
                  displayMode === 'used'
                    ? 'bg-white/15 text-white font-semibold'
                    : 'text-white/40 hover:text-white/70'
                }`}
                title="Show quota consumed"
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

        {/* View Mode Segmented Switch: Detailed vs Compact */}
        <div className="flex items-center p-0.5 rounded-xl bg-black/40 border border-white/8 my-3">
          <button
            type="button"
            onClick={() => setViewMode('detailed')}
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
            onClick={() => setViewMode('compact')}
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
          <div className="space-y-2.5 max-h-[340px] overflow-y-auto scrollbar-none pr-0.5">
            {providerGroups.map(({ key, name, icon: Icon, quota }) => {
              const resetWindow = quota.windows.find((w) => w.resets_at);
              const resetStr = resetWindow ? formatResetTime(resetWindow.resets_at) : null;

              // Check max used across all windows to determine provider health status
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
                      <span className="text-xs font-semibold text-white">{name}</span>
                    </div>

                    <div className="flex items-center gap-1.5">
                      {isExhausted ? (
                        <span className="inline-flex items-center gap-1 text-[9.5px] font-semibold text-rose-400 bg-rose-500/15 border border-rose-500/30 px-1.5 py-0.5 rounded">
                          <AlertTriangle className="size-2.5 shrink-0" />
                          Exhausted
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
                                className={`font-mono text-[11px] w-12 text-right tabular-nums ${
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

                        let shortName = w.name.includes('5h') || w.name === 'Session' ? 'Session' : 'Weekly';
                        if (w.name.includes('gemini')) shortName = 'Gemini';
                        if (w.name.includes('3p')) shortName = 'Claude/GPT';

                        return (
                          <div key={idx} className="flex items-center justify-between px-2 py-1 rounded-lg bg-black/25 border border-white/5">
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
            <p className="text-xs text-white/60 mb-1">No active vendor quotas reported</p>
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
