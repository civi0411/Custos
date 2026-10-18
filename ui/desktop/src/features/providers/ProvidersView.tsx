import React, { useState } from 'react';
import {
  Plus,
  Layers,
  BarChart3,
  Server,
  Shield,
  Key,
  Terminal,
} from 'lucide-react';
import { ProviderItem, ClientApiKey } from '@/types';
import { useAppContext } from '@/context/AppContext';
import { CombosManager } from '@/components/providers/CombosManager';
import { UsageMonitor } from '@/components/providers/UsageMonitor';
import { ProviderBrandIcon, getProviderBrandMeta } from '@/components/common/AgentIcons';

interface ProvidersViewProps {
  providers: ProviderItem[];
  clientKeys?: ClientApiKey[];
  onOpenAddProviderModal: () => void;
  onGenerateClientKey?: () => void;
  onRevokeClientKey?: (id: string) => void;
  onTestConnection: (providerName: string) => void;
  onDeleteProvider?: (id: string) => void;
}

export const ProvidersView: React.FC<ProvidersViewProps> = ({
  onOpenAddProviderModal,
  onTestConnection,
}) => {
  const { connectedAccounts, handleDeleteAccount, refreshModels } = useAppContext();
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
            {connectedAccounts.length}
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
            {/* SECTION: Connected OAuth & Account Identifiers */}
            <div className="space-y-4 min-w-0">
              <div className="flex items-center justify-between">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle">
                  Connected Identity & Provider Accounts
                </h3>
                <span className="text-[11px] text-fg-subtle font-mono">
                  {connectedAccounts.length} Active Accounts
                </span>
              </div>

              {connectedAccounts.length === 0 ? (
                <div className="p-8 rounded-2xl bg-surface-1 border border-dashed border-border-muted text-center space-y-3">
                  <div className="w-12 h-12 rounded-2xl bg-surface-2 border border-border-default flex items-center justify-center mx-auto text-fg-muted">
                    <Server className="w-6 h-6" />
                  </div>
                  <div>
                    <h4 className="text-sm font-semibold text-fg-editor">No AI Providers Connected Yet</h4>
                    <p className="text-xs text-fg-muted max-w-md mx-auto mt-1">
                      Connect your OpenAI, Anthropic Claude, Google Antigravity, DeepSeek, or local Ollama models to enable multi-account routing, automatic failover, and prompt execution.
                    </p>
                  </div>
                  <button
                    onClick={onOpenAddProviderModal}
                    className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg workbench-primary-action text-xs font-medium cursor-pointer shadow-xs transition mt-2"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    <span>Connect First Provider</span>
                  </button>
                </div>
              ) : (
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                  {connectedAccounts.map((account) => {
                    const brandMeta = getProviderBrandMeta(account.providerId);
                    const isOauth = account.authType === 'oauth';
                    const isLocal = account.providerId === 'local';

                    return (
                      <div
                        key={account.id}
                        className="p-4 sm:p-5 rounded-2xl bg-surface-1 border border-border-muted hover:border-border-default transition flex flex-col justify-between space-y-4 shadow-xs relative overflow-hidden group"
                      >
                        {/* Top row: Brand Avatar + Name + Provider Tag + Health Dot */}
                        <div className="flex items-start justify-between gap-3 min-w-0">
                          <div className="flex items-start gap-3 min-w-0">
                            {/* Visual Provider Brand Avatar */}
                            <div
                              className="w-10 h-10 rounded-xl flex items-center justify-center shrink-0 transition-transform group-hover:scale-105 shadow-xs"
                              style={{
                                background: brandMeta.bgLight,
                                border: `1px solid ${brandMeta.borderColor}`,
                              }}
                              title={brandMeta.name}
                            >
                              <ProviderBrandIcon provider={account.providerId} size={20} />
                            </div>

                            <div className="min-w-0 flex-1">
                              <div className="flex items-center gap-2 flex-wrap">
                                <h4 className="text-sm font-semibold text-fg-editor truncate">{account.accountName}</h4>
                                
                                {/* Exact Provider Badge */}
                                <span
                                  className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium font-mono shrink-0"
                                  style={{
                                    background: brandMeta.bgLight,
                                    color: brandMeta.brandColor,
                                    border: `1px solid ${brandMeta.borderColor}`,
                                  }}
                                >
                                  <ProviderBrandIcon provider={account.providerId} size={10} />
                                  <span>{brandMeta.name}</span>
                                </span>

                                {/* Auth Type Indicator */}
                                <span className="text-[9px] font-mono px-1.5 py-0.5 rounded border bg-surface-2 border-border-default text-fg-subtle shrink-0 flex items-center gap-1">
                                  {isLocal ? (
                                    <>
                                      <Terminal className="w-2.5 h-2.5" />
                                      <span>Local</span>
                                    </>
                                  ) : isOauth ? (
                                    <>
                                      <Shield className="w-2.5 h-2.5 text-indigo-400" />
                                      <span>OAuth</span>
                                    </>
                                  ) : (
                                    <>
                                      <Key className="w-2.5 h-2.5 text-emerald-400" />
                                      <span>API Key</span>
                                    </>
                                  )}
                                </span>

                                {account.oauthTier && (
                                  <span className="text-[9px] font-mono px-1.5 py-0.5 rounded text-indigo-400 bg-indigo-500/10 border border-indigo-500/20 shrink-0">
                                    {account.oauthTier}
                                  </span>
                                )}
                              </div>

                              {/* Identity subline (OAuth email, masked key, or endpoint) */}
                              <div className="mt-1 font-mono text-[11px] text-fg-muted flex items-center gap-1.5 truncate">
                                {account.oauthEmail ? (
                                  <span className="truncate text-fg-editor/90 font-sans font-medium">{account.oauthEmail}</span>
                                ) : account.apiKeyMasked ? (
                                  <span className="font-mono text-fg-muted">{account.apiKeyMasked}</span>
                                ) : (
                                  <span className="truncate">{brandMeta.name}</span>
                                )}
                                {account.endpointUrl && (
                                  <span className="text-fg-subtle text-[10px] truncate max-w-[200px]">· {account.endpointUrl}</span>
                                )}
                              </div>
                            </div>
                          </div>

                          {/* Health Status Indicator */}
                          <div className="flex items-center gap-1.5 shrink-0 mt-0.5">
                            <span
                              className={`w-2 h-2 rounded-full shrink-0 ${
                                account.status === 'active'
                                  ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]'
                                  : account.status === 'rate_limited'
                                  ? 'bg-amber-400 animate-pulse'
                                  : 'bg-zinc-400'
                              }`}
                              title={account.statusLabel || account.status}
                            />
                            <span className="text-[10px] font-mono capitalize hidden sm:inline text-fg-subtle">
                              {account.status === 'active' ? 'Ready' : account.status === 'rate_limited' ? 'Cooldown' : account.status}
                            </span>
                          </div>
                        </div>

                        {/* Local runtime probe retry if offline */}
                        {isLocal && (
                          <div className="flex items-center justify-between text-xs bg-surface-2/40 px-3 py-1.5 rounded-lg border border-border-muted/50">
                            <span className={account.status === 'active' ? 'text-emerald-400 font-mono text-[11px]' : 'text-fg-muted font-mono text-[11px]'}>
                              {account.status === 'active' ? 'Local engine active' : 'Engine offline'}
                            </span>
                            {account.status !== 'active' && (
                              <button
                                type="button"
                                onClick={() => void refreshModels()}
                                className="rounded border border-border-default bg-surface-2 hover:bg-surface-3 px-2 py-0.5 text-[10px] text-fg-editor transition cursor-pointer"
                              >
                                Retry Probe
                              </button>
                            )}
                          </div>
                        )}

                        {/* Bottom row: Default Model & Action buttons */}
                        <div className="flex items-center justify-between pt-2.5 border-t border-border-muted/50 text-[11px] font-mono text-fg-subtle">
                          <div className="flex items-center gap-2 truncate pr-2">
                            <span className="text-fg-subtle">Default:</span>
                            <span className="font-semibold text-fg-editor truncate">{account.defaultModel || 'auto'}</span>
                          </div>
                          <div className="flex items-center gap-3 shrink-0">
                            <button
                              type="button"
                              onClick={() => onTestConnection(account.id)}
                              className="text-fg-muted hover:text-fg-editor transition cursor-pointer flex items-center gap-1"
                            >
                              <span>Test</span>
                            </button>
                            <button
                              type="button"
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
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

          </div>
        )}
      </div>
    </main>
  );
};
