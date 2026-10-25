import React, { useCallback, useEffect, useState } from 'react';
import { RefreshCw } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { HarnessDescriptor } from '@/types/domain';

/** A harness is an executable adapter, not a registered autonomous sub-agent. */
export const SubAgentsPane: React.FC = () => {
  const [harnesses, setHarnesses] = useState<HarnessDescriptor[] | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError('');
    try { setHarnesses(await daemonClient.listHarnesses()); }
    catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  return <div className="max-w-2xl space-y-4">
    <div className="flex items-start justify-between gap-4">
      <div>
        <h2 className="text-sm font-semibold text-fg-editor">Agent runtimes</h2>
        <p className="mt-1 text-xs leading-5 text-fg-muted">
          Runtimes discovered by the daemon. Availability means an installed harness, not an active worker.
          Sub-agent permissions and nesting limits are not configurable until the scheduler enforces them.
        </p>
      </div>
      <button type="button" onClick={() => void refresh()} disabled={loading} aria-label="Refresh agent runtimes" className="rounded-md border border-border-default p-2 text-fg-muted hover:bg-surface-2 disabled:opacity-50">
        <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
      </button>
    </div>
    {error && <p role="alert" className="text-xs text-rose-500">Could not load runtimes: {error}</p>}
    {harnesses === null && !error && <p role="status" className="text-xs text-fg-muted">Loading daemon runtimes…</p>}
    {harnesses?.length === 0 && <p className="rounded-lg border border-border-default p-4 text-xs text-fg-muted">No agent runtimes are registered with this daemon.</p>}
    {harnesses?.map(harness => <div key={harness.id} className="flex items-start justify-between gap-4 rounded-lg border border-border-default bg-surface-1 p-4">
      <div className="min-w-0">
        <div className="text-xs font-semibold text-fg-editor">{harness.name}</div>
        <p className="mt-1 text-xs text-fg-muted">{harness.description}</p>
        <p className="mt-1 break-all font-mono text-[10px] text-fg-subtle">{harness.binary_path || 'No executable path reported'}</p>
      </div>
      <span className={`shrink-0 text-[11px] ${harness.is_available ? 'text-emerald-500' : 'text-fg-subtle'}`}>{harness.is_available ? 'Available' : 'Unavailable'}</span>
    </div>)}
  </div>;
};
