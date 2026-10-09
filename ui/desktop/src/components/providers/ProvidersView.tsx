import React, { useState } from 'react';
import {
  Plus,
  Cpu,
  Laptop,
  Terminal,
  Trash2,
  RefreshCw,
  Layers,
  BarChart3,
  Server,
} from 'lucide-react';
import { ProviderItem, ClientApiKey } from '@/types';
import { useAppContext } from '@/context/AppContext';
import { CombosManager } from '@/components/providers/CombosManager';
import { UsageMonitor } from '@/components/providers/UsageMonitor';

interface ProvidersViewProps {
  providers: ProviderItem[];
  clientKeys: ClientApiKey[];
  onOpenAddProviderModal: () => void;
  onGenerateClientKey: () => void;
  onRevokeClientKey: (id: string) => void;
  onTestConnection: (providerName: string) => void;
  onDeleteProvider?: (id: string) => void;
}

const getProviderLogo = (iconType: string): string | null => {
  switch (iconType) {
    case 'anthropic':
      return '/assets/provider-logo/anthropic.jpg';
    case 'openai':
      return '/assets/provider-logo/openai.jpg';
    case 'gemini':
      return '/assets/provider-logo/gemini.jpg';
    case 'deepseek':
      return '/assets/provider-logo/deepseek.jpg';
    default:
      return null;
  }
};

export const ProvidersView: React.FC<ProvidersViewProps> = ({
  providers,
  clientKeys,
  onOpenAddProviderModal,
  onGenerateClientKey,
  onRevokeClientKey,
  onTestConnection,
  onDeleteProvider,
}) => {
  const { connectedAccounts, handleDeleteAccount } = useAppContext();
  const [activeSubTab, setActiveSubTab] = useState<'providers' | 'combos' | 'usage'>('providers');

  return (
    <main className="flex-1 bg-canvas flex flex-col overflow-y-auto min-w-0 text-fg-editor font-sans">
      {/* Top Header */}
      <div className="px-4 sm:px-8 pt-6 sm:pt-8 pb-4 border-b border-border-muted flex items-center justify-between min-w-0 bg-surface-1">
        <div className="min-w-0">
          <h1 className="text-lg sm:text-xl font-semibold tracking-[-0.02em] text-fg-editor flex items-center gap-2 truncate">
            <span>Model Routing Gateway</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-500 font-mono font-normal shrink-0">
              OmniRoute / 9Router Architecture
            </span>
          </h1>
          <p className="text-xs text-fg-muted mt-1 truncate">
            Multi-account OAuth & API connections, virtual routing combos, quota monitoring, and failover chains.
          </p>
        </div>

        <div className="flex items-center gap-3 shrink-0 ml-2">
          <button
            onClick={onOpenAddProviderModal}
            className="px-3 sm:px-3.5 py-1.5 rounded-lg workbench-primary-action font-medium text-xs flex items-center gap-1.5 transition shadow-xs cursor-pointer"
          >
            <Plus className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Connect Provider</span>
            <span className="sm:hidden">Connect</span>
          </button>
        </div>
      </div>

      {/* Sub-tab Navigation Bar */}
      <div className="px-4 sm:px-8 border-b border-border-muted bg-surface-1 flex gap-2 text-xs">
        <button
          type="button"
          onClick={() => setActiveSubTab('providers')}
          className={`py-3 px-3 font-medium transition border-b-2 flex items-center gap-1.5 cursor-pointer ${
            activeSubTab === 'providers'
              ? 'border-workbench-accent text-fg-editor font-semibold'
              : 'border-transparent text-fg-muted hover:text-fg-editor'
          }`}
        >
          <Server className="w-3.5 h-3.5" />
          <span>Providers & Accounts</span>
          <span className="text-[10px] font-mono px-1.5 py-0.2 rounded-full bg-surface-2 text-fg-subtle">
            {providers.length + connectedAccounts.length}
          </span>
        </button>

        <button
          type="button"
          onClick={() => setActiveSubTab('combos')}
          className={`py-3 px-3 font-medium transition border-b-2 flex items-center gap-1.5 cursor-pointer ${
            activeSubTab === 'combos'
              ? 'border-workbench-accent text-fg-editor font-semibold'
              : 'border-transparent text-fg-muted hover:text-fg-editor'
          }`}
        >
          <Layers className="w-3.5 h-3.5 text-workbench-accent" />
          <span>Routing Combos</span>
        </button>

        <button
          type="button"
          onClick={() => setActiveSubTab('usage')}
          className={`py-3 px-3 font-medium transition border-b-2 flex items-center gap-1.5 cursor-pointer ${
            activeSubTab === 'usage'
              ? 'border-workbench-accent text-fg-editor font-semibold'
              : 'border-transparent text-fg-muted hover:text-fg-editor'
          }`}
        >
          <BarChart3 className="w-3.5 h-3.5 text-emerald-400" />
          <span>Usage & Quotas</span>
        </button>
      </div>

      {/* Main Tab Content */}
      <div className="px-4 sm:px-8 py-6 sm:py-8 max-w-6xl w-full mx-auto min-w-0">
        {activeSubTab === 'combos' && <CombosManager />}
        {activeSubTab === 'usage' && <UsageMonitor />}
        {activeSubTab === 'providers' && (
          <div className="space-y-8 min-w-0">
            {/* SECTION 1: Configured AI Providers */}
            <div className="space-y-4 min-w-0">
              <div className="flex items-center justify-between">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle">
                  Configured AI Model Providers (Daemon Persisted)
                </h3>
                <span className="text-[11px] text-fg-subtle font-mono">
                  Live Probe Enabled · SQLite Persisted
                </span>
              </div>

              <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                {providers.map((p) => {
                  const logoUrl = getProviderLogo(p.iconType);
                  const ctxK = p.contextWindow ? `${Math.round(p.contextWindow / 1000)}k ctx` : null;

                  return (
                    <div
                      key={p.id}
                      className="p-4 sm:p-5 rounded-2xl bg-surface-1 border border-border-muted hover:border-border-default transition flex flex-col justify-between space-y-4 shadow-xs"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3 min-w-0">
                          {logoUrl ? (
                            <div className="w-10 h-10 rounded-xl overflow-hidden shrink-0 shadow-xs border border-border-default bg-surface-2">
                              <img
                                src={logoUrl}
                                alt={p.name}
                                className="w-full h-full object-cover"
                              />
                            </div>
                          ) : (
                            <div className="w-10 h-10 rounded-xl bg-surface-2 flex items-center justify-center workbench-accent font-bold text-sm shrink-0 border border-border-default">
                              <Cpu className="w-5 h-5" />
                            </div>
                          )}

                          <div className="min-w-0">
                            <h4 className="text-sm font-semibold text-fg-editor truncate">{p.name}</h4>
                            <div className="flex items-center gap-2 mt-0.5">
                              <span className="text-[11px] text-fg-muted font-mono truncate">{p.model}</span>
                              {ctxK && (
                                <span className="text-[10px] px-1.5 py-0.2 rounded bg-surface-2 text-fg-subtle font-mono shrink-0">
                                  {ctxK}
                                </span>
                              )}
                            </div>
                          </div>
                        </div>

                        <div className="flex items-center gap-1.5 shrink-0">
                          <span
                            className="w-2 h-2 rounded-full"
                            style={{ backgroundColor: p.badgeColor }}
                          />
                          <span className="text-xs font-mono text-fg-subtle">{p.statusLabel}</span>
                        </div>
                      </div>

                      {/* Provider Details */}
                      <div className="space-y-2 text-xs font-mono">
                        <div className="flex justify-between text-fg-muted">
                          <span>API Key Secret:</span>
                          <span className="text-fg-subtle">{p.apiKey}</span>
                        </div>
                        {p.endpoint && (
                          <div className="flex justify-between text-fg-muted truncate">
                            <span>Endpoint URL:</span>
                            <span className="text-fg-subtle truncate max-w-[240px]">{p.endpoint}</span>
                          </div>
                        )}
                      </div>

                      {/* Action Bar */}
                      <div className="flex items-center justify-between pt-2 border-t border-border-muted/50">
                        <div className="flex items-center gap-2">
                          <span className="text-[11px] text-fg-subtle font-mono">{p.latency}</span>
                          {onDeleteProvider && (
                            <button
                              onClick={() => {
                                if (window.confirm(`Xóa cấu hình provider "${p.name}"?`)) {
                                  onDeleteProvider(p.id);
                                }
                              }}
                              className="text-fg-subtle hover:text-rose-400 p-1 rounded transition cursor-pointer"
                              title="Delete provider"
                            >
                              <Trash2 className="w-3.5 h-3.5" />
                            </button>
                          )}
                        </div>

                        <button
                          onClick={() => onTestConnection(p.id || p.name)}
                          className="px-2.5 py-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-editor text-[11px] border border-border-default hover:border-border-emphasis transition flex items-center gap-1.5 cursor-pointer"
                        >
                          <RefreshCw className="w-3 h-3 text-fg-muted" />
                          <span>{p.endpoint ? 'Probe & Verify' : 'Test Connection'}</span>
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* SECTION 2: Connected OAuth & Account Identifiers */}
            <div className="space-y-4 min-w-0">
              <div className="flex items-center justify-between">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle">
                  Connected Identity & OAuth Accounts
                </h3>
                <span className="text-[11px] text-fg-subtle font-mono">
                  {connectedAccounts.length} Active Accounts
                </span>
              </div>

              <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                {connectedAccounts.map((account) => (
                  <div
                    key={account.id}
                    className="p-4 sm:p-5 rounded-2xl bg-surface-1 border border-border-muted hover:border-border-default transition flex flex-col justify-between space-y-3 shadow-xs"
                  >
                    <div className="flex items-start justify-between">
                      <div>
                        <div className="flex items-center gap-2">
                          <h4 className="text-sm font-semibold text-fg-editor">{account.accountName}</h4>
                          <span className="text-[9px] font-mono px-1.5 py-0.2 rounded border bg-surface-2 border-border-default text-fg-editor">
                            {account.authType === 'oauth' ? 'OAuth 2.0' : 'API Key'}
                          </span>
                          {account.oauthTier && (
                            <span className="text-[9px] font-mono px-1.5 py-0.2 rounded text-indigo-400 bg-indigo-500/10 border border-indigo-500/20">
                              {account.oauthTier}
                            </span>
                          )}
                        </div>
                        <div className="mt-1 font-mono text-[11px] text-fg-muted flex items-center gap-1.5">
                          <span>{account.providerName}</span>
                          {account.oauthEmail && <span>· {account.oauthEmail}</span>}
                        </div>
                      </div>

                      <span
                        className="w-2.5 h-2.5 rounded-full shrink-0 mt-1"
                        style={{ backgroundColor: account.badgeColor || '#10b981' }}
                        title={account.statusLabel || account.status}
                      />
                    </div>

                    <div className="flex items-center justify-between pt-2 border-t border-border-muted/50 text-[11px] font-mono text-fg-subtle">
                      <span>Default: {account.defaultModel}</span>
                      <button
                        onClick={() => {
                          if (window.confirm(`Disconnect account "${account.accountName}"?`)) {
                            handleDeleteAccount(account.id);
                          }
                        }}
                        className="text-rose-400 hover:text-rose-300 transition cursor-pointer"
                      >
                        Disconnect
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* SECTION 3: Gateway Client API Keys */}
            <div className="space-y-4 min-w-0">
              <div className="flex items-center justify-between">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle">
                  Desktop & CLI Gateway Keys
                </h3>
                <button
                  onClick={onGenerateClientKey}
                  className="text-xs workbench-accent hover:underline flex items-center gap-1 font-medium cursor-pointer"
                >
                  <Plus className="w-3 h-3" />
                  <span>Generate new key</span>
                </button>
              </div>

              <div className="border border-border-default rounded-xl overflow-x-auto bg-surface-1 shadow-xs">
                <div className="min-w-[500px]">
                  <div className="grid grid-cols-12 px-4 py-2.5 border-b border-border-muted text-[11px] font-mono text-fg-subtle bg-surface-2/60">
                    <span className="col-span-4">KEY NAME</span>
                    <span className="col-span-4">KEY TOKEN</span>
                    <span className="col-span-2">CREATED</span>
                    <span className="col-span-2 text-right">ACTION</span>
                  </div>

                  {clientKeys.map((k) => (
                    <div
                      key={k.id}
                      className="grid grid-cols-12 px-4 py-3 items-center border-b border-border-muted/50 last:border-b-0 text-xs text-fg-editor"
                    >
                      <span className="col-span-4 font-medium text-fg-editor flex items-center gap-2 truncate">
                        {k.icon === 'laptop' ? (
                          <Laptop className="w-3.5 h-3.5 workbench-accent shrink-0" />
                        ) : (
                          <Terminal className="w-3.5 h-3.5 text-fg-muted shrink-0" />
                        )}
                        <span className="truncate">{k.name}</span>
                      </span>
                      <span className="col-span-4 font-mono text-[11px] text-fg-muted truncate">{k.token}</span>
                      <span className="col-span-2 text-fg-subtle text-[11px] font-mono">{k.created}</span>
                      <div className="col-span-2 text-right">
                        <button
                          onClick={() => onRevokeClientKey(k.id)}
                          className="text-rose-500 hover:text-rose-400 text-[11px] cursor-pointer"
                        >
                          Revoke
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </main>
  );
};
