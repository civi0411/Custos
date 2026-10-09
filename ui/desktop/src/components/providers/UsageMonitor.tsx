import React, { useState, useMemo } from 'react';
import {
  BarChart3,
  DollarSign,
  Cpu,
  RotateCcw,
  AlertTriangle,
  CheckCircle2,
  TrendingUp,
  Search,
  ShieldAlert
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { getProviderInfo, ProviderIcon } from '@/components/chat/ModelSelector';

export const UsageMonitor: React.FC = () => {
  const { usageRecords, handleResetUsage } = useAppContext();
  const [searchFilter, setSearchFilter] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'normal' | 'throttled'>('all');

  // KPI aggregates
  const summary = useMemo(() => {
    let totalTokens = 0;
    let totalInput = 0;
    let totalOutput = 0;
    let totalCost = 0;
    let totalRequests = 0;
    let throttledCount = 0;

    usageRecords.forEach((r) => {
      totalTokens += r.totalTokens;
      totalInput += r.inputTokens;
      totalOutput += r.outputTokens;
      totalCost += r.estimatedCostUsd;
      totalRequests += r.requestCount;
      if (r.rateLimitStatus === 'throttled' || r.quotaPercent >= 90) {
        throttledCount += 1;
      }
    });

    return {
      totalTokens,
      totalInput,
      totalOutput,
      totalCost,
      totalRequests,
      throttledCount,
      accountsCount: usageRecords.length,
    };
  }, [usageRecords]);

  // Filtered list
  const filteredRecords = useMemo(() => {
    return usageRecords.filter((r) => {
      const matchSearch =
        r.modelName.toLowerCase().includes(searchFilter.toLowerCase()) ||
        r.accountName.toLowerCase().includes(searchFilter.toLowerCase()) ||
        r.providerName.toLowerCase().includes(searchFilter.toLowerCase());

      const matchStatus =
        statusFilter === 'all'
          ? true
          : statusFilter === 'throttled'
          ? r.rateLimitStatus === 'throttled' || r.quotaPercent >= 90
          : r.rateLimitStatus === 'normal' && r.quotaPercent < 90;

      return matchSearch && matchStatus;
    });
  }, [usageRecords, searchFilter, statusFilter]);

  const formatTokens = (val: number) => {
    if (val >= 1_000_000) return `${(val / 1_000_000).toFixed(2)}M`;
    if (val >= 1_000) return `${(val / 1_000).toFixed(1)}k`;
    return val.toString();
  };

  return (
    <div className="space-y-6">
      {/* Top Header & Reset Action */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 p-4 rounded-xl border border-border-default bg-surface-1">
        <div>
          <h2 className="text-sm font-semibold text-fg-editor flex items-center gap-2">
            <BarChart3 className="w-4 h-4 text-workbench-accent" />
            <span>Quota & Usage Analytics</span>
            <span className="text-[11px] font-mono px-2 py-0.5 rounded-full bg-surface-2 border border-border-muted text-fg-subtle">
              Live Monitored
            </span>
          </h2>
          <p className="text-xs text-fg-muted mt-0.5">
            OmniRoute / 9Router telemetry: Real-time quota exhaustion tracking, token metrics, and cost estimations.
          </p>
        </div>

        <button
          onClick={() => {
            if (window.confirm('Reset local token telemetry counters to 0?')) {
              handleResetUsage();
            }
          }}
          className="px-3 py-1.5 rounded-lg border border-border-default hover:border-border-emphasis bg-surface-2 hover:bg-surface-3 text-fg-editor text-xs font-medium flex items-center gap-1.5 cursor-pointer transition shadow-xs"
          title="Reset local token and request counters"
        >
          <RotateCcw className="w-3.5 h-3.5" />
          <span>Reset Metrics</span>
        </button>
      </div>

      {/* KPI Cards Grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        {/* Total Tokens */}
        <div className="p-4 rounded-xl border border-border-default bg-surface-1 shadow-xs space-y-1">
          <div className="text-[11px] font-medium text-fg-muted flex items-center justify-between">
            <span>Total Tokens</span>
            <Cpu className="w-3.5 h-3.5 text-workbench-accent" />
          </div>
          <div className="text-xl font-bold font-mono text-fg-editor">
            {formatTokens(summary.totalTokens)}
          </div>
          <div className="text-[10px] text-fg-subtle font-mono">
            In: {formatTokens(summary.totalInput)} · Out: {formatTokens(summary.totalOutput)}
          </div>
        </div>

        {/* Estimated Cost */}
        <div className="p-4 rounded-xl border border-border-default bg-surface-1 shadow-xs space-y-1">
          <div className="text-[11px] font-medium text-fg-muted flex items-center justify-between">
            <span>Est. Cost</span>
            <DollarSign className="w-3.5 h-3.5 text-emerald-400" />
          </div>
          <div className="text-xl font-bold font-mono text-emerald-400">
            ${summary.totalCost.toFixed(2)}
          </div>
          <div className="text-[10px] text-fg-subtle font-mono">
            Across {summary.totalRequests} queries
          </div>
        </div>

        {/* Total Requests */}
        <div className="p-4 rounded-xl border border-border-default bg-surface-1 shadow-xs space-y-1">
          <div className="text-[11px] font-medium text-fg-muted flex items-center justify-between">
            <span>API Queries</span>
            <TrendingUp className="w-3.5 h-3.5 text-sky-400" />
          </div>
          <div className="text-xl font-bold font-mono text-fg-editor">
            {summary.totalRequests}
          </div>
          <div className="text-[10px] text-fg-subtle font-mono">
            Average {(summary.totalTokens / (summary.totalRequests || 1)).toFixed(0)} tok/req
          </div>
        </div>

        {/* Quota Health */}
        <div className="p-4 rounded-xl border border-border-default bg-surface-1 shadow-xs space-y-1">
          <div className="text-[11px] font-medium text-fg-muted flex items-center justify-between">
            <span>Quota Health</span>
            {summary.throttledCount > 0 ? (
              <AlertTriangle className="w-3.5 h-3.5 text-amber-400" />
            ) : (
              <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
            )}
          </div>
          <div className={`text-xl font-bold font-mono ${summary.throttledCount > 0 ? 'text-amber-400' : 'text-emerald-400'}`}>
            {summary.throttledCount > 0 ? `${summary.throttledCount} Warning` : '100% Healthy'}
          </div>
          <div className="text-[10px] text-fg-subtle font-mono">
            {summary.accountsCount} accounts active
          </div>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-3 pt-2">
        <div className="relative w-full sm:w-72">
          <Search className="w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-fg-subtle" />
          <input
            type="text"
            value={searchFilter}
            onChange={(e) => setSearchFilter(e.target.value)}
            placeholder="Filter models or accounts..."
            className="w-full pl-8 pr-3 py-1.5 rounded-lg bg-surface-1 border border-border-default text-xs text-fg-editor placeholder-fg-subtle focus:outline-none focus:border-border-emphasis font-mono"
          />
        </div>

        <div className="flex items-center gap-1.5 self-end sm:self-auto">
          <button
            onClick={() => setStatusFilter('all')}
            className={`px-2.5 py-1 rounded-md text-xs font-medium transition cursor-pointer ${
              statusFilter === 'all'
                ? 'bg-surface-2 text-fg-editor border border-border-default'
                : 'text-fg-muted hover:text-fg-editor'
            }`}
          >
            All Accounts
          </button>
          <button
            onClick={() => setStatusFilter('normal')}
            className={`px-2.5 py-1 rounded-md text-xs font-medium transition cursor-pointer ${
              statusFilter === 'normal'
                ? 'bg-surface-2 text-fg-editor border border-border-default'
                : 'text-fg-muted hover:text-fg-editor'
            }`}
          >
            Healthy
          </button>
          <button
            onClick={() => setStatusFilter('throttled')}
            className={`px-2.5 py-1 rounded-md text-xs font-medium transition cursor-pointer ${
              statusFilter === 'throttled'
                ? 'bg-surface-2 text-amber-400 border border-border-default'
                : 'text-fg-muted hover:text-fg-editor'
            }`}
          >
            Near Limit ({summary.throttledCount})
          </button>
        </div>
      </div>

      {/* Detailed Table */}
      <div className="rounded-xl border border-border-default bg-surface-1 overflow-hidden shadow-xs">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs border-collapse">
            <thead>
              <tr className="border-b border-border-muted bg-surface-2/50 text-[11px] font-mono text-fg-subtle uppercase tracking-wider">
                <th className="py-2.5 px-4">Model & Account</th>
                <th className="py-2.5 px-3">Provider</th>
                <th className="py-2.5 px-3">Quota Utilization</th>
                <th className="py-2.5 px-3 text-right">Tokens Used</th>
                <th className="py-2.5 px-3 text-right">Est. Cost</th>
                <th className="py-2.5 px-4 text-right">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border-muted/60">
              {filteredRecords.length === 0 ? (
                <tr>
                  <td colSpan={6} className="py-8 text-center text-xs text-fg-muted">
                    No usage records found matching filters.
                  </td>
                </tr>
              ) : (
                filteredRecords.map((record) => {
                  const info = getProviderInfo(record.modelId);
                  const isHighUsage = record.quotaPercent >= 80;
                  const isCritical = record.quotaPercent >= 90;

                  return (
                    <tr key={record.id} className="hover:bg-surface-2/40 transition">
                      {/* Model & Account */}
                      <td className="py-3 px-4">
                        <div className="font-semibold text-fg-editor">{record.modelName}</div>
                        <div className="text-[11px] text-fg-muted font-mono flex items-center gap-1.5 mt-0.5">
                          <span>{record.accountName}</span>
                          <span>·</span>
                          <span>{record.lastUsedAt}</span>
                        </div>
                      </td>

                      {/* Provider */}
                      <td className="py-3 px-3">
                        <div className="flex items-center gap-1.5 text-fg-editor">
                          <ProviderIcon provider={info.provider} size={14} />
                          <span>{record.providerName}</span>
                        </div>
                      </td>

                      {/* Quota Meter */}
                      <td className="py-3 px-3 min-w-[160px]">
                        <div className="space-y-1">
                          <div className="flex items-center justify-between text-[10px] font-mono">
                            <span className={isCritical ? 'text-rose-400 font-bold' : isHighUsage ? 'text-amber-400' : 'text-fg-subtle'}>
                              {record.quotaPercent}% utilized
                            </span>
                            <span className="text-fg-muted">
                              Limit: {formatTokens(record.quotaLimitTokens)}
                            </span>
                          </div>
                          <div className="h-1.5 w-full bg-surface-2 rounded-full overflow-hidden">
                            <div
                              className={`h-full rounded-full transition-all duration-300 ${
                                isCritical
                                  ? 'bg-rose-500'
                                  : isHighUsage
                                  ? 'bg-amber-400'
                                  : 'bg-emerald-500'
                              }`}
                              style={{ width: `${Math.min(100, record.quotaPercent)}%` }}
                            />
                          </div>
                          <div className="text-[9px] text-fg-subtle truncate">
                            {record.resetPeriod}
                          </div>
                        </div>
                      </td>

                      {/* Tokens */}
                      <td className="py-3 px-3 text-right font-mono">
                        <div className="font-medium text-fg-editor">
                          {formatTokens(record.totalTokens)}
                        </div>
                        <div className="text-[10px] text-fg-subtle">
                          {record.requestCount} reqs
                        </div>
                      </td>

                      {/* Est Cost */}
                      <td className="py-3 px-3 text-right font-mono text-emerald-400 font-medium">
                        ${record.estimatedCostUsd.toFixed(2)}
                      </td>

                      {/* Status */}
                      <td className="py-3 px-4 text-right">
                        <span
                          className={`inline-flex items-center gap-1 text-[10px] font-medium px-2 py-0.5 rounded-full border ${
                            record.rateLimitStatus === 'throttled' || isCritical
                              ? 'bg-rose-500/10 text-rose-400 border-rose-500/20'
                              : isHighUsage
                              ? 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                              : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                          }`}
                        >
                          {record.rateLimitStatus === 'throttled' || isCritical ? (
                            <>
                              <ShieldAlert className="w-3 h-3" />
                              <span>Throttled</span>
                            </>
                          ) : (
                            <>
                              <CheckCircle2 className="w-3 h-3" />
                              <span>Normal</span>
                            </>
                          )}
                        </span>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
