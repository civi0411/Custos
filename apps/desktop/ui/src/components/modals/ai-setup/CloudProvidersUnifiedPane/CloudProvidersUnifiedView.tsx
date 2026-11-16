import React, { useState, useEffect } from 'react';
import { Key, ShieldCheck } from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { ApiKeyProvidersSection } from './ApiKeyProvidersSection';
import { OAuthProvidersSection } from './OAuthProvidersSection';

export interface CloudProvidersUnifiedPaneProps {
  onShowToast?: (msg: string) => void;
  initialSubTab?: 'api_keys' | 'oauth';
}

export const CloudProvidersUnifiedView: React.FC<CloudProvidersUnifiedPaneProps> = ({
  onShowToast,
  initialSubTab = 'api_keys',
}) => {
  const {
    connectedAccounts,
    handleDeleteAccount,
    refreshModels,
  } = useAppContext();

  const [subTab, setSubTab] = useState<'api_keys' | 'oauth'>(initialSubTab);
  const [refreshingId, setRefreshingId] = useState<string | null>(null);

  useEffect(() => {
    if (initialSubTab) {
      setSubTab(initialSubTab);
    }
  }, [initialSubTab]);

  const apiKeyAccounts = connectedAccounts.filter((a) => a.authType !== 'oauth' && a.providerId !== 'lmstudio');
  const oauthAccounts = connectedAccounts.filter((a) => a.authType === 'oauth');

  const handleRefreshAccount = async (id: string) => {
    setRefreshingId(id);
    try {
      await refreshModels();
      onShowToast?.('Account credentials refreshed.');
    } finally {
      setRefreshingId(null);
    }
  };

  const handleDisconnect = async (id: string, name: string) => {
    if (window.confirm(`Disconnect account "${name}"?`)) {
      await handleDeleteAccount(id);
      await refreshModels();
      onShowToast?.(`Disconnected "${name}".`);
    }
  };

  return (
    <div className="space-y-4 max-w-4xl text-[var(--color-editor-fg)] font-sans">
      {/* ── Top Pane Header with Integrated Mode Switcher ── */}
      <div className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-4 shadow-xs">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-editor-fg)] font-mono">
                Cloud Providers & Accounts
              </h2>
              <span className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-0.5 font-mono text-[10px] text-[var(--color-fg-muted)]">
                {connectedAccounts.length} Connected
              </span>
            </div>
            <p className="text-[11px] text-[var(--color-fg-muted)] mt-0.5">
              Configure private API keys or authorize official accounts via OAuth 2.0 PKCE with auto-pooling.
            </p>
          </div>

          {/* Integrated Mode Switcher */}
          <div className="flex items-center rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-1 text-[11px] gap-1 shrink-0">
            <button
              type="button"
              onClick={() => setSubTab('api_keys')}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1 transition cursor-pointer ${
                subTab === 'api_keys'
                  ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium shadow-xs border border-[var(--color-border-emphasis)]'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-1)] border border-transparent'
              }`}
            >
              <Key className={`size-3 ${subTab === 'api_keys' ? 'text-emerald-400' : 'text-[var(--color-fg-subtle)]'}`} />
              <span>Cloud API Keys</span>
              <span
                className={`text-[10px] font-mono px-1.5 py-0.2 rounded-full ${
                  subTab === 'api_keys'
                    ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                    : 'bg-[var(--color-surface-2)] text-[var(--color-fg-subtle)]'
                }`}
              >
                {apiKeyAccounts.length}
              </span>
            </button>

            <button
              type="button"
              onClick={() => setSubTab('oauth')}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1 transition cursor-pointer ${
                subTab === 'oauth'
                  ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium shadow-xs border border-[var(--color-border-emphasis)]'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-1)] border border-transparent'
              }`}
            >
              <ShieldCheck className={`size-3 ${subTab === 'oauth' ? 'text-sky-400' : 'text-[var(--color-fg-subtle)]'}`} />
              <span>OAuth Sign-In</span>
              <span
                className={`text-[10px] font-mono px-1.5 py-0.2 rounded-full ${
                  subTab === 'oauth'
                    ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                    : 'bg-[var(--color-surface-2)] text-[var(--color-fg-subtle)]'
                }`}
              >
                {oauthAccounts.length}
              </span>
            </button>
          </div>
        </div>
      </div>

      {/* ── MODE 1: CLOUD API KEYS ── */}
      {subTab === 'api_keys' && (
        <ApiKeyProvidersSection
          apiKeyAccounts={apiKeyAccounts}
          onRefreshAccount={handleRefreshAccount}
          onDisconnect={handleDisconnect}
          refreshingId={refreshingId}
          onShowToast={onShowToast}
        />
      )}

      {/* ── MODE 2: OAUTH SSO ── */}
      {subTab === 'oauth' && (
        <OAuthProvidersSection
          oauthAccounts={oauthAccounts}
          onRefreshAccount={handleRefreshAccount}
          onDisconnect={handleDisconnect}
          refreshingId={refreshingId}
          onShowToast={onShowToast}
        />
      )}
    </div>
  );
};
