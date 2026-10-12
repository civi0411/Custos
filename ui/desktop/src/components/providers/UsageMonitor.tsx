import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { BarChart3, RefreshCw, Search, ChevronDown, ChevronUp } from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import { ProviderQuotaSnapshot, ProviderUsageReport, UsageRecord } from '@/types/provider_types';

export function summarizeUsage(records: UsageRecord[]) {
  const sum = (key: 'input_tokens' | 'output_tokens' | 'total_tokens') => {
    const values = records.map(r => r[key]).filter((v): v is number => typeof v === 'number' && Number.isFinite(v));
    return values.length ? values.reduce((total, value) => total + value, 0) : null;
  };
  return { requests: records.length, input: sum('input_tokens'), output: sum('output_tokens'), total: sum('total_tokens'), unknown: records.filter(r => r.total_tokens == null).length, failed: records.filter(r => r.status.toLowerCase() === 'failed').length, limited: records.filter(r => r.rate_limited).length };
}

const number = (value: number | null) => value == null ? 'Unknown' : value.toLocaleString();
const resetTime = (value: string | number | null) => {
  if (value == null) return 'Not reported';
  const date = new Date(typeof value === 'number' ? value * 1000 : value);
  return Number.isNaN(date.getTime()) ? 'Not reported' : date.toLocaleString();
};

function QuotaView({ quota, compact = false }: { quota?: ProviderQuotaSnapshot; compact?: boolean }) {
  if (!quota) return <span className="text-fg-subtle">Not fetched</span>;
  if (quota.status !== 'available') return <span className="text-fg-subtle" title={quota.error}>{quota.status === 'not_applicable' ? 'Not applicable (local)' : 'No provider quota data'}</span>;
  if (compact) return <div className="flex flex-wrap gap-1.5">
    {quota.windows.map(window => {
      const remaining = Math.max(0, Math.min(100, 100 - window.used_percent));
      return <span key={window.name} title={`Reset: ${resetTime(window.resets_at)} · ${quota.source === 'provider_catalog' ? 'Catalog snapshot; may be stale' : 'Provider snapshot'} · ${new Date(quota.fetched_at).toLocaleString()}`} className={`rounded border px-2 py-1 text-[10px] ${remaining <= 10 ? 'border-amber-400/30 text-amber-400' : 'border-border-muted text-fg-muted'}`}>{window.name}: {remaining.toFixed(1)}% remaining</span>;
    })}
  </div>;
  return <div className="space-y-2">
    {quota.windows.map(window => {
      const remaining = Math.max(0, Math.min(100, 100 - window.used_percent));
      return <div key={window.name} className="space-y-1">
      <div className="flex justify-between gap-3"><span>{window.name}</span><span>{remaining.toFixed(1)}% remaining</span></div>
      <div className="h-1.5 rounded bg-surface-2 overflow-hidden"><div className={remaining <= 10 ? 'h-full bg-amber-400' : 'h-full bg-emerald-400'} style={{ width: `${remaining}%` }} /></div>
      <p className="text-[10px] text-fg-subtle">Reset: {resetTime(window.resets_at)}</p>
    </div>; })}
    <p className="text-[10px] text-fg-subtle">{quota.source === 'provider_catalog' ? 'Catalog snapshot; may be stale' : 'Provider snapshot'} · {new Date(quota.fetched_at).toLocaleString()}</p>
  </div>;
}

export const UsageMonitor: React.FC = () => {
  const { connectedAccounts, providers } = useAppContext();
  const [report, setReport] = useState<ProviderUsageReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [search, setSearch] = useState('');
  const [period, setPeriod] = useState('7');
  const [compact, setCompact] = useState(() => {
    try { return localStorage.getItem('custos_usage_view_mode') === 'compact'; } catch { return false; }
  });
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const changeView = (value: boolean) => {
    setCompact(value);
    try { localStorage.setItem('custos_usage_view_mode', value ? 'compact' : 'expanded'); } catch { /* Keep the view usable when browser storage is unavailable. */ }
  };
  const alive = useRef(false);
  const busy = useRef(false);
  const refresh = useCallback(async (quota: boolean) => {
    if (busy.current) return;
    busy.current = true; setLoading(true); setError('');
    try {
      // Show durable usage first, without waiting for remote quota services.
      const local = await daemonClient.getProviderUsage(false);
      if (!alive.current) return;
      setReport(previous => ({ ...local, quotas: previous?.quotas || [] }));
      if (quota) {
        const complete = await daemonClient.getProviderUsage(true);
        if (alive.current) setReport(complete);
      }
    } catch (err) { if (alive.current) setError(err instanceof Error ? err.message : String(err)); }
    finally { busy.current = false; if (alive.current) setLoading(false); }
  }, []);
  useEffect(() => {
    alive.current = true; void refresh(true);
    const timer = window.setInterval(() => { if (!document.hidden) void refresh(false); }, 15000);
    return () => { alive.current = false; window.clearInterval(timer); };
  }, [refresh]);

  const records = useMemo(() => (report?.records || []).filter(r => period === 'all' || Date.parse(r.recorded_at) >= Date.now() - Number(period) * 86400000), [report, period]);
  const summary = summarizeUsage(records);
  const accounts = useMemo(() => {
    const ids = new Set([...connectedAccounts.map(a => a.id), ...records.map(r => r.account_id || 'unattributed')]);
    return [...ids].map(id => {
      const account = connectedAccounts.find(a => a.id === id);
      const saved = providers.find(p => p.id === id);
      const name = account?.oauthEmail || account?.accountName || saved?.name || (id === 'unattributed' ? 'Unattributed historical usage' : `Disconnected account (${id})`);
      return { id, name, provider: account?.providerName || saved?.name || 'Unknown', iconProvider: account?.providerId || saved?.iconType || 'other', rows: records.filter(r => (r.account_id || 'unattributed') === id), quota: report?.quotas.find(q => q.account_id === id) };
    }).filter(a => `${a.name} ${a.provider} ${a.rows.map(r => r.model).join(' ')}`.toLowerCase().includes(search.toLowerCase()));
  }, [connectedAccounts, providers, records, report, search]);

  return <div className="space-y-5">
    <div className="flex flex-wrap items-center justify-between gap-3 p-4 border border-border-default rounded-xl bg-surface-1">
      <div><h2 className="flex items-center gap-2 text-sm font-semibold"><BarChart3 className="w-4 h-4" />Account Usage & Quotas</h2><p className="text-xs text-fg-muted mt-1">Usage recorded by Custos. Provider quota snapshots are separate; missing measurements remain unknown.</p></div>
      <button type="button" onClick={() => void refresh(true)} disabled={loading} className="provider-oauth-action rounded-lg px-3 py-2 text-xs flex items-center gap-2 disabled:opacity-50"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />{loading ? 'Refreshing...' : 'Refresh usage & quota'}</button>
    </div>
    {error && <p role="alert" className="text-xs text-amber-400">Could not refresh usage: {error}. {report ? 'Showing the last successful snapshot.' : 'No data loaded.'}</p>}
    <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
      {[['Requests', summary.requests], ['Reported tokens', number(summary.total)], ['Failed requests', summary.failed], ['Rate-limited requests', summary.limited]].map(([label, value]) => <div key={label} className="p-4 rounded-xl border border-border-default bg-surface-1"><p className="text-xs text-fg-muted">{label}</p><p className="text-xl font-mono mt-2">{value}</p></div>)}
    </div>
    <p className="text-xs text-fg-subtle">{summary.unknown} requests without reported total tokens. {report ? `Updated ${new Date(report.fetched_at).toLocaleString()}.` : ''} No billing amount is inferred for subscriptions or local inference.</p>
    <div className="flex flex-wrap items-center gap-3"><div className="relative flex-1"><Search className="absolute left-3 top-2.5 w-3.5 h-3.5 text-fg-subtle" /><input aria-label="Filter account usage" value={search} onChange={e => setSearch(e.target.value)} placeholder="Filter accounts or models" className="w-full bg-surface-1 border border-border-default rounded-lg pl-9 pr-3 py-2 text-xs" /></div><select aria-label="Usage period" value={period} onChange={e => setPeriod(e.target.value)} className="bg-surface-1 border border-border-default rounded-lg px-3 py-2 text-xs"><option value="1">Last 24 hours</option><option value="7">Last 7 days</option><option value="30">Last 30 days</option><option value="all">All recorded usage</option></select></div>
    <div className="flex items-center justify-between gap-3">
      <span className="text-xs text-fg-muted">Account panels</span>
      <div role="group" aria-label="Usage view mode" className="flex rounded-lg border border-border-default p-0.5 bg-surface-1">
        {(['Expanded', 'Compact'] as const).map(mode => <button key={mode} type="button" aria-pressed={compact === (mode === 'Compact')} onClick={() => changeView(mode === 'Compact')} className={`rounded-md px-3 py-1.5 text-xs transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 ${compact === (mode === 'Compact') ? 'bg-surface-3 text-fg-editor' : 'text-fg-muted hover:bg-surface-2'}`}>{mode}</button>)}
      </div>
    </div>
    {!report && loading && <p role="status" className="text-xs text-fg-muted">Loading durable usage...</p>}
    {report && !accounts.length && <p className="text-xs text-fg-muted">No accounts or recorded usage match this filter.</p>}
    {accounts.map(account => {
      const totals = summarizeUsage(account.rows);
      const models = [...new Set(account.rows.map(r => r.model || 'Unknown model'))];
      const isCollapsed = Boolean(collapsed[account.id]);
      return <section key={account.id} className="rounded-xl border border-border-default bg-surface-1 overflow-hidden">
        <div className={`px-4 ${compact ? 'py-2' : 'py-3'} ${isCollapsed ? '' : 'border-b border-border-muted'} flex items-start justify-between gap-3`}>
          <div className="min-w-0"><h3 className="flex items-center gap-2 text-sm font-semibold"><ProviderBrandIcon provider={account.iconProvider} size={16} className="shrink-0" /><span className="min-w-0 break-words">{account.name}</span></h3><p className="text-xs text-fg-muted">{account.provider} · {totals.requests} requests · {totals.failed} failed · {totals.limited} rate limited</p>
            {isCollapsed && <div className="mt-2 text-xs"><QuotaView quota={account.quota} compact /></div>}
          </div>
          <button type="button" aria-expanded={!isCollapsed} aria-label={`${isCollapsed ? 'Expand' : 'Collapse'} ${account.name}`} onClick={() => setCollapsed(previous => ({ ...previous, [account.id]: !previous[account.id] }))} title={isCollapsed ? 'Expand panel' : 'Collapse panel'} className="shrink-0 rounded-md border border-border-default p-1.5 text-fg-muted hover:bg-surface-2 hover:text-fg-editor focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2">{isCollapsed ? <ChevronDown width={14} height={14} aria-hidden="true" /> : <ChevronUp width={14} height={14} aria-hidden="true" />}</button>
        </div>
        {!isCollapsed && <>
        <div className={`grid md:grid-cols-2 ${compact ? 'gap-3 px-4 py-2' : 'gap-5 p-4'}`}><div className="text-xs space-y-2"><p>Reported tokens: <span className="font-mono">{number(totals.total)}</span></p>{!compact && <><p className="text-fg-muted">Input: {number(totals.input)} · Output: {number(totals.output)}</p><p className="text-fg-subtle">{totals.unknown} requests with unknown total usage</p></>}</div><div className="text-xs"><p className="font-medium mb-2">Remaining provider quota</p><QuotaView quota={account.quota} compact={compact} /></div></div>
        {!compact && models.length > 0 && <div className="overflow-x-auto"><table className="w-full text-xs text-left"><thead className="bg-surface-2 text-fg-subtle"><tr><th className="px-4 py-2">Model</th><th className="px-4 py-2">Requests</th><th className="px-4 py-2">Reported tokens</th><th className="px-4 py-2">Failures</th></tr></thead><tbody>{models.map(model => { const totals = summarizeUsage(account.rows.filter(r => (r.model || 'Unknown model') === model)); return <tr key={model} className="border-t border-border-muted"><td className="px-4 py-2 font-mono">{model}</td><td className="px-4 py-2">{totals.requests}</td><td className="px-4 py-2">{number(totals.total)}{totals.unknown > 0 ? ` (+${totals.unknown} unknown)` : ''}</td><td className="px-4 py-2">{totals.failed}</td></tr>; })}</tbody></table></div>}
        </>}
      </section>;
    })}
  </div>;
};
