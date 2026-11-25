import React, { useCallback, useEffect, useState } from 'react';
import { RefreshCw } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';

export const DaemonPane: React.FC = () => {
  const [online, setOnline] = useState<boolean | null>(null);
  const [checking, setChecking] = useState(false);
  const check = useCallback(async () => {
    setChecking(true);
    try { setOnline(await daemonClient.checkHealth()); }
    finally { setChecking(false); }
  }, []);
  useEffect(() => { void check(); }, [check]);

  return <div className="max-w-2xl space-y-4">
    <div><h2 className="text-sm font-semibold text-fg-editor">Local daemon</h2><p className="mt-1 text-xs text-fg-muted">Live health of the daemon used by this desktop.</p></div>
    <div className="flex items-center justify-between gap-4 rounded-lg border border-border-default bg-surface-1 p-4">
      <div><div className="text-xs font-medium text-fg-editor">Connection</div>
        <div role="status" className={`mt-1 text-xs ${online === true ? 'text-emerald-500' : online === false ? 'text-rose-500' : 'text-fg-muted'}`}>
          {online === null ? 'Checking…' : online ? 'Connected' : 'Unavailable'}
        </div>
      </div>
      <button type="button" onClick={() => void check()} disabled={checking} className="flex items-center gap-2 rounded-md border border-border-default px-3 py-2 text-xs text-fg-editor hover:bg-surface-2 disabled:opacity-50">
        <RefreshCw className={`h-3.5 w-3.5 ${checking ? 'animate-spin' : ''}`} />Check again
      </button>
    </div>
    <p className="text-xs leading-5 text-fg-muted">Endpoint and authentication are managed by the desktop transport. This screen does not change the active connection.</p>
  </div>;
};
