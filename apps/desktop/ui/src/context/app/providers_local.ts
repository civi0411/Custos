import { useEffect, useMemo } from 'react';
import type { ConnectedAccount } from '@/types';
import { daemonClient } from '@/api/daemon_client';

export function useLocalAccountsPoll(
  connectedAccounts: ConnectedAccount[],
  setConnectedAccounts: React.Dispatch<React.SetStateAction<ConnectedAccount[]>>
) {
  const localAccountsKey = useMemo(() => {
    return connectedAccounts
      .filter(a => a.providerId === 'local' && a.endpointUrl && (a.endpointUrl?.includes(':11434') || a.defaultModel?.startsWith('ollama/') || a.supportedModels.some(m => m.startsWith('ollama/'))))
      .map(a => `${a.id}:${a.endpointUrl}`)
      .join('|');
  }, [connectedAccounts]);

  useEffect(() => {
    const accounts = connectedAccounts.filter(a => a.providerId === 'local' && a.endpointUrl && (a.endpointUrl?.includes(':11434') || a.defaultModel?.startsWith('ollama/') || a.supportedModels.some(m => m.startsWith('ollama/'))));
    if (!accounts.length) return;
    let cancelled = false;
    let probing = false;
    const check = async () => {
      if (probing || document.hidden) return;
      probing = true;
      try {
        await Promise.all(accounts.map(async account => {
          let updates: Partial<ConnectedAccount>;
          try {
            const result = await daemonClient.probeModels(account.endpointUrl!, 'ollama');
            updates = {
              status: 'active',
              statusLabel: 'Connected',
              badgeColor: '#10b981',
              catalogSource: 'live',
              catalogError: undefined,
              catalogFetchedAt: Date.now(),
              supportedModels: result.models.map(m => m.id),
              catalogModels: result.models.map((m, i) => ({
                id: m.id,
                label: m.name || m.id,
                provider_type: 'local',
                is_default: i === 0,
                efforts: [],
                supports_fast_mode: false,
                context_window: m.context_window,
              })),
            };
          } catch {
            updates = {
              status: 'offline',
              statusLabel: 'Ollama offline',
              badgeColor: '#6b7280',
              catalogSource: account.catalogModels?.length ? 'cached' : 'unavailable',
              catalogError: 'Ollama is unavailable. Start Ollama, then retry. The selected model has not changed.',
            };
          }
          if (!cancelled) {
            setConnectedAccounts(previous => previous.map(a => a.id === account.id && a.endpointUrl === account.endpointUrl ? { ...a, ...updates } : a));
          }
        }));
      } finally {
        probing = false;
      }
    };
    const timer = window.setInterval(() => { if (!document.hidden) void check(); }, 15000);
    const onVisible = () => { if (!document.hidden) void check(); };
    window.addEventListener('focus', onVisible);
    window.addEventListener('visibilitychange', onVisible);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
      window.removeEventListener('focus', onVisible);
      window.removeEventListener('visibilitychange', onVisible);
    };
  }, [localAccountsKey, setConnectedAccounts]);
}
