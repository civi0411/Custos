import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft, RefreshCw } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import type { ProviderUsageReport } from '@/types/provider_types';

/** A small operational summary. No inferred reliability, process ID, or simulated route metrics. */
export const DashboardPage: React.FC = () => {
  const navigate = useNavigate();
  const { currentSessions, connectedAccounts, openSettings } = useAppContext();
  const [online, setOnline] = useState<boolean | null>(null);
  const [report, setReport] = useState<ProviderUsageReport | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const healthy = await daemonClient.checkHealth();
      setOnline(healthy);
      if (!healthy) { setReport(null); return; }
      setReport(await daemonClient.getProviderUsage(false));
    } catch (cause) {
      setOnline(false);
      setReport(null);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally { setLoading(false); }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  const knownTokens = report?.records.reduce((total, record) => total + (record.total_tokens ?? 0), 0);
  const unknownAttempts = report?.records.filter((record) => record.total_tokens == null).length ?? 0;

  return <main className="flex h-full flex-1 flex-col overflow-y-auto bg-canvas text-fg-editor">
    <header className="flex h-12 shrink-0 items-center gap-3 border-b border-border-muted px-4">
      <button type="button" onClick={() => navigate('/studio')} aria-label="Back to Studio" className="rounded p-1.5 text-fg-muted hover:bg-surface-2"><ArrowLeft className="h-4 w-4" /></button>
      <h1 className="flex-1 text-sm font-medium">Custos status</h1>
      <button type="button" onClick={() => void refresh()} disabled={loading} aria-label="Refresh status" className="rounded p-1.5 text-fg-muted hover:bg-surface-2 disabled:opacity-50"><RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} /></button>
    </header>
    <div className="mx-auto w-full max-w-3xl space-y-5 p-6">
      {error && <p role="alert" className="text-xs text-rose-500">Could not load daemon usage: {error}</p>}
      <dl className="grid gap-3 sm:grid-cols-2">
        <div className="rounded-lg border border-border-default bg-surface-1 p-4"><dt className="text-xs text-fg-muted">Daemon</dt><dd className="mt-1 text-sm font-medium">{online === null ? 'Checking…' : online ? 'Connected' : 'Unavailable'}</dd></div>
        <div className="rounded-lg border border-border-default bg-surface-1 p-4"><dt className="text-xs text-fg-muted">Loaded sessions</dt><dd className="mt-1 text-sm font-medium">{currentSessions.length}</dd></div>
        <div className="rounded-lg border border-border-default bg-surface-1 p-4"><dt className="text-xs text-fg-muted">Configured accounts</dt><dd className="mt-1 text-sm font-medium">{connectedAccounts.length}</dd></div>
        <div className="rounded-lg border border-border-default bg-surface-1 p-4"><dt className="text-xs text-fg-muted">Recorded model attempts</dt><dd className="mt-1 text-sm font-medium">{report ? report.records.length : 'Unavailable'}</dd></div>
      </dl>
      <section className="text-xs leading-5 text-fg-muted">
        {report ? `Known token total: ${knownTokens?.toLocaleString() ?? 0}. Attempts with unknown usage: ${unknownAttempts}.` : 'Usage appears only when the daemon returns attempt records.'}
        {' '}This screen does not estimate uptime, reliability or active routing policy.
      </section>
      <div className="flex gap-2">
        <button type="button" onClick={() => openSettings('providers')} className="rounded border border-border-default px-3 py-1.5 text-xs hover:bg-surface-2">Models & accounts</button>
        <button type="button" onClick={() => openSettings('quotas')} className="rounded border border-border-default px-3 py-1.5 text-xs hover:bg-surface-2">Usage</button>
      </div>
    </div>
  </main>;
};

export default DashboardPage;
