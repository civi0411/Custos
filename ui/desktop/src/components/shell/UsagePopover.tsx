import React, { useState } from 'react';
import { RotateCw, ChevronRight } from 'lucide-react';
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
      return `Resets in ${diffDays}d ${diffHours % 24}h`;
    }
    if (diffHours > 0) {
      return `Resets in ${diffHours}h ${diffMins % 60}m`;
    }
    return `Resets in ${diffMins}m`;
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
  align = 'left',
}) => {
  const { openAiSetup, openSettings } = useAppContext();
  const [viewMode, setViewMode] = useState<'detailed' | 'compact'>('detailed');
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
      ? 'fixed bottom-10 left-12'
      : align === 'left'
        ? 'fixed top-11 left-48'
        : align === 'right'
          ? 'fixed top-11 right-36'
          : 'fixed top-11 left-1/2 -translate-x-1/2';

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-[90] bg-black/30 backdrop-blur-[1px]"
        onClick={onClose}
      />

      {/* Floating Usage Card (Matching Orca/Antigravity style) */}
      <div
        className={`${positionClasses} w-[340px] sm:w-[380px] bg-[#18181b] border border-white/12 rounded-2xl shadow-2xl z-[100] p-3.5 text-white/90 select-none animate-in fade-in zoom-in-95 duration-150`}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between pb-3">
          <h2 className="text-[15px] font-semibold tracking-tight text-white">Usage</h2>
          <div className="flex items-center gap-2">
            <span className="text-[11px] text-white/40 font-medium">all agents</span>
            <button
              type="button"
              onClick={handleRefresh}
              className="p-1 rounded-md text-white/50 hover:text-white hover:bg-white/10 transition cursor-pointer"
              title="Refresh quota & limits"
              aria-label="Refresh usage data"
            >
              <RotateCw className={`size-3.5 ${isRefreshing ? 'animate-spin' : ''}`} />
            </button>
          </div>
        </div>

        {/* Detailed / Compact Segmented Switch */}
        <div className="flex items-center p-0.5 rounded-xl bg-black/40 border border-white/8 mb-3.5">
          <button
            type="button"
            onClick={() => setViewMode('detailed')}
            className={`flex-1 py-1 text-xs font-medium rounded-lg transition cursor-pointer text-center ${
              viewMode === 'detailed'
                ? 'bg-[#2a2a2e] text-white shadow-sm'
                : 'text-white/40 hover:text-white/70'
            }`}
          >
            Detailed
          </button>
          <button
            type="button"
            onClick={() => setViewMode('compact')}
            className={`flex-1 py-1 text-xs font-medium rounded-lg transition cursor-pointer text-center ${
              viewMode === 'compact'
                ? 'bg-[#2a2a2e] text-white shadow-sm'
                : 'text-white/40 hover:text-white/70'
            }`}
          >
            Compact
          </button>
        </div>

        {/* Dynamic Provider Cards from live quotas */}
        {providerGroups.length > 0 ? (
          <div className="space-y-2.5 max-h-[360px] overflow-y-auto scrollbar-none pr-0.5">
            {providerGroups.map(({ key, name, icon: Icon, quota }) => {
              // Primary reset description from first window with a reset time
              const resetWindow = quota.windows.find((w) => w.resets_at);
              const resetStr = resetWindow ? formatResetTime(resetWindow.resets_at) : null;

              return (
                <div key={key} className="rounded-xl bg-white/[0.03] border border-white/6 p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <div className="size-6 rounded-full bg-white/10 flex items-center justify-center p-1 shrink-0">
                        <Icon className="size-3.5 text-white/90" />
                      </div>
                      <span className="text-sm font-semibold text-white">{name}</span>
                    </div>
                    {resetStr && (
                      <span className="text-[11px] text-white/40 font-mono">{resetStr}</span>
                    )}
                  </div>

                  {viewMode === 'detailed' ? (
                    <div className="space-y-2 pt-1 text-[11.5px]">
                      {quota.windows.map((w, idx) => {
                        const pct = Math.min(100, Math.max(0, Math.round(w.used_percent)));
                        const isHigh = pct >= 90;
                        const isMed = pct >= 70 && pct < 90;

                        // Human-friendly title
                        let title = w.name;
                        if (w.name === '3p-weekly') title = 'Claude & GPT Models · Weekly';
                        else if (w.name === '3p-5h') title = 'Claude & GPT Models · 5h Session';
                        else if (w.name === 'gemini-weekly') title = 'Gemini Models · Weekly';
                        else if (w.name === 'gemini-5h') title = 'Gemini Models · 5h Session';
                        else if (w.name === 'Session') title = '5-Hour Session Window';
                        else if (w.name === 'Weekly') title = 'Weekly Window';

                        return (
                          <div key={idx} className="flex items-center justify-between gap-2">
                            <span className="text-white/70 truncate">{title}</span>
                            <div className="flex items-center gap-1.5 shrink-0">
                              <div className="w-14 h-1.5 rounded-full bg-white/10 overflow-hidden">
                                <div
                                  className={`h-full rounded-full transition-all duration-300 ${
                                    isHigh ? 'bg-rose-500' : isMed ? 'bg-amber-400' : 'bg-white/60'
                                  }`}
                                  style={{ width: `${pct}%` }}
                                />
                              </div>
                              <span
                                className={`font-mono text-[11px] w-8 text-right ${
                                  isHigh
                                    ? 'text-rose-400 font-semibold'
                                    : isMed
                                      ? 'text-amber-300 font-medium'
                                      : 'text-white/80'
                                }`}
                              >
                                {pct}%
                              </span>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  ) : (
                    <div className="grid grid-cols-2 gap-2 pt-1 text-[11px]">
                      {quota.windows.slice(0, 2).map((w, idx) => {
                        const pct = Math.min(100, Math.max(0, Math.round(w.used_percent)));
                        const isHigh = pct >= 90;
                        let shortName = w.name.includes('5h') || w.name === 'Session' ? 'Session (5h)' : 'Weekly';
                        if (w.name.includes('gemini')) shortName = 'Gemini (Wk)';
                        if (w.name.includes('3p')) shortName = 'Claude/GPT (Wk)';

                        return (
                          <div key={idx} className="flex items-center justify-between px-2 py-1 rounded bg-black/25">
                            <span className="text-white/50 truncate pr-1">{shortName}</span>
                            <span className={`font-mono font-medium ${isHigh ? 'text-rose-400' : 'text-white/80'}`}>
                              {pct}%
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
          <div className="py-6 px-4 text-center text-xs text-white/50">
            No active rate limits reported. Connect an account in Settings to monitor quotas.
          </div>
        )}

        {/* Footer links */}
        <div className="pt-3 mt-3 border-t border-white/10 space-y-1">
          <button
            type="button"
            onClick={handleOpenQuotas}
            className="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-white/6 text-xs text-white/80 hover:text-white transition cursor-pointer"
          >
            <span>Usage details & cost quotas</span>
            <ChevronRight className="size-3.5 text-white/40" />
          </button>
          <button
            type="button"
            onClick={handleOpenAccounts}
            className="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-white/6 text-xs text-white/80 hover:text-white transition cursor-pointer"
          >
            <span>Manage provider accounts</span>
            <ChevronRight className="size-3.5 text-white/40" />
          </button>
        </div>
      </div>
    </>
  );
};
