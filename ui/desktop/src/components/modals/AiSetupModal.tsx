import React, { useEffect, useState } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import {
  Cpu,
  KeyRound,
  Bot,
  Coins,
  ArrowUp,
  ArrowDown,
  Plus,
  ChevronDown,
  X,
  Layers,
  Sparkles,
} from 'lucide-react';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import { LocalRuntimePane } from '@/app/settings/LocalRuntimePane';
import { SubAgentsPane } from '@/app/settings/SubAgentsPane';
import { CostQuotasPane } from '@/app/settings/CostQuotasPane';

export type AiSetupTab = 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas';

export interface AiSetupModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialTab?: AiSetupTab;
  onShowToast: (msg: string) => void;
}

export const AiSetupModal: React.FC<AiSetupModalProps> = ({
  isOpen,
  onClose,
  initialTab = 'local',
  onShowToast,
}) => {
  const {
    connectedAccounts,
    handleDeleteAccount,
    setIsAddProviderOpen,
    refreshModels,
    appearance,
  } = useAppContext();

  const [activeTab, setActiveTab] = useState<AiSetupTab>(initialTab);
  const [routingPolicies, setRoutingPolicies] = useState<Record<string, { round_robin: boolean; account_order: string[] }>>({});
  const [routingBusy, setRoutingBusy] = useState(false);
  const [routingError, setRoutingError] = useState('');
  const [expandedProviders, setExpandedProviders] = useState<Record<string, boolean>>({});

  const systemReducedMotion = useReducedMotion();
  const reducePanelMotion = appearance?.reduceMotion || systemReducedMotion;

  useEffect(() => {
    if (isOpen && initialTab) {
      setActiveTab(initialTab);
    }
  }, [isOpen, initialTab]);

  const routingServices = [...new Set(connectedAccounts.map((a) => a.providerId))].sort().join(',');

  useEffect(() => {
    if (!isOpen) return;
    let cancelled = false;
    setRoutingError('');
    Promise.all(
      [...new Set(connectedAccounts.map((a) => a.providerId))].map(async (service) => [service, await daemonClient.getProviderRouting(service)] as const)
    )
      .then((values) => {
        if (!cancelled) setRoutingPolicies(Object.fromEntries(values));
      })
      .catch((error) => {
        if (!cancelled) setRoutingError(String(error?.message || error));
      });
    return () => {
      cancelled = true;
    };
  }, [isOpen, routingServices]);

  const orderedAccounts = [...connectedAccounts].sort((a, b) => {
    const serviceOrder = a.providerId.localeCompare(b.providerId);
    if (serviceOrder) return serviceOrder;
    const order = routingPolicies[a.providerId]?.account_order || [];
    const position = (id: string) => {
      const index = order.indexOf(id);
      return index < 0 ? Number.MAX_SAFE_INTEGER : index;
    };
    return position(a.id) - position(b.id);
  });

  const updateRouting = async (service: string, enabled: boolean, order: string[]) => {
    setRoutingBusy(true);
    setRoutingError('');
    try {
      const policy = await daemonClient.saveProviderRouting(service, enabled, order);
      setRoutingPolicies((previous) => ({ ...previous, [service]: policy }));
    } catch (error: any) {
      setRoutingError(error?.message || String(error));
    } finally {
      setRoutingBusy(false);
    }
  };

  const moveAccount = (id: string, service: string, direction: number) => {
    const order = orderedAccounts.filter((a) => a.providerId === service).map((a) => a.id);
    const from = order.indexOf(id);
    const to = from + direction;
    if (from < 0 || to < 0 || to >= order.length) return;
    [order[from], order[to]] = [order[to], order[from]];
    void updateRouting(service, routingPolicies[service]?.round_robin || false, order);
  };

  if (!isOpen) return null;

  const navItems = [
    {
      id: 'local' as const,
      label: 'Local Models & Hardware',
      description: 'Apple Silicon Metal, GGUF weights & VRAM',
      icon: Cpu,
      badge: 'Metal GPU',
    },
    {
      id: 'cloud' as const,
      label: 'Cloud Providers & Keys',
      description: 'Claude, OpenAI, Gemini, DeepSeek & OAuth',
      icon: KeyRound,
      badge: connectedAccounts.length > 0 ? `${connectedAccounts.length}` : undefined,
    },
    {
      id: 'routing' as const,
      label: 'Model Routing & Order',
      description: 'Priority ordering, round-robin & failover',
      icon: Layers,
    },
    {
      id: 'subagents' as const,
      label: 'Subagent Workers',
      description: 'Pack workforce & specialized agent roles',
      icon: Bot,
    },
    {
      id: 'quotas' as const,
      label: 'Usage & Cost Limits',
      description: 'Token tracking & session budget headroom',
      icon: Coins,
    },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs">
      <div className="flex h-[720px] max-h-[92vh] w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] shadow-2xl">
        {/* Header */}
        <header className="flex h-15 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-6">
          <div className="flex items-center gap-3">
            <div className="flex size-8 items-center justify-center rounded-lg bg-teal-500/15 text-teal-400 border border-teal-500/30 shadow-xs">
              <Sparkles className="size-4.5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-sm font-semibold text-[var(--color-editor-fg)]">AI & Model Setup</h1>
                <span className="rounded bg-teal-500/15 border border-teal-500/30 px-1.5 py-0.2 font-mono text-[9px] font-semibold text-teal-300">
                  Sovereign Studio
                </span>
              </div>
              <p className="text-[11px] text-[var(--color-fg-muted)]">
                Local inference engine, cloud credentials, model routing policies and agent workers
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close AI Setup"
            className="rounded-md p-1.5 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] transition cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </header>

        {/* Content Body */}
        <div className="flex min-h-0 flex-1">
          {/* Left Navigation */}
          <nav className="w-64 shrink-0 border-r border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3 space-y-1 overflow-y-auto">
            <div className="px-2.5 pb-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)] font-mono">
              Setup Categories
            </div>
            {navItems.map(({ id, label, description, icon: Icon, badge }) => (
              <button
                key={id}
                type="button"
                onClick={() => setActiveTab(id)}
                className={`flex w-full items-start gap-2.5 rounded-xl p-2.5 text-left transition cursor-pointer ${
                  activeTab === id
                    ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] border border-teal-500/30 shadow-xs'
                    : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]/60 hover:text-[var(--color-editor-fg)] border border-transparent'
                }`}
              >
                <Icon
                  className={`h-4 w-4 shrink-0 mt-0.5 ${
                    activeTab === id ? 'text-teal-400' : 'text-[var(--color-fg-subtle)]'
                  }`}
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center justify-between gap-1">
                    <span className="text-xs font-medium truncate">{label}</span>
                    {badge && (
                      <span className="rounded bg-teal-500/20 px-1 py-0.2 font-mono text-[9px] text-teal-300 font-semibold shrink-0">
                        {badge}
                      </span>
                    )}
                  </div>
                  <p className="mt-0.5 text-[10px] text-[var(--color-fg-subtle)] line-clamp-1">
                    {description}
                  </p>
                </div>
              </button>
            ))}
          </nav>

          {/* Right Tab Content View */}
          <div className="min-w-0 flex-1 overflow-y-auto p-6">
            {activeTab === 'local' && (
              <LocalRuntimePane
                onNavigateToProviders={() => setActiveTab('cloud')}
                onShowToast={onShowToast}
              />
            )}

            {activeTab === 'cloud' && (
              <div className="space-y-6">
                {/* Header with Title and Connect Button */}
                <div className="flex items-start justify-between">
                  <div>
                    <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">
                      Cloud Providers & API Accounts
                    </h2>
                    <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">
                      Connect official accounts via OAuth or configure private API keys.
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() => setIsAddProviderOpen(true)}
                    className="flex items-center gap-1.5 rounded-md bg-[var(--color-editor-fg)] px-3 py-1.5 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 transition cursor-pointer shadow-xs"
                  >
                    <Plus className="h-3.5 w-3.5" /> Connect Provider Account
                  </button>
                </div>

                {/* Connected Accounts List */}
                <div className="space-y-3">
                  <div className="text-[11px] font-semibold uppercase font-mono tracking-wider text-[var(--color-fg-subtle)] flex items-center justify-between">
                    <span>Active Provider Credentials</span>
                    <span className="text-[10px] lowercase text-[var(--color-fg-subtle)]">
                      {connectedAccounts.length} accounts configured
                    </span>
                  </div>

                  {connectedAccounts.length === 0 ? (
                    <div className="rounded-xl border border-dashed border-[var(--color-border-muted)] p-8 text-center text-xs text-[var(--color-fg-muted)] space-y-2">
                      <p>No cloud provider credentials connected yet.</p>
                      <button
                        type="button"
                        onClick={() => setIsAddProviderOpen(true)}
                        className="inline-flex items-center gap-1.5 rounded-md bg-teal-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-teal-500 transition cursor-pointer"
                      >
                        <Plus className="h-3.5 w-3.5" /> Connect First Provider
                      </button>
                    </div>
                  ) : (
                    <div className="space-y-2">
                      {connectedAccounts.map((account) => (
                        <div
                          key={account.id}
                          className="flex items-center justify-between rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 hover:border-[var(--color-border-default)] transition"
                        >
                          <div className="flex items-center gap-3 min-w-0">
                            <ProviderBrandIcon provider={account.providerId} size={22} />
                            <div className="min-w-0">
                              <div className="flex items-center gap-2">
                                <span className="text-xs font-semibold text-[var(--color-editor-fg)] truncate">
                                  {account.accountName}
                                </span>
                                <span className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1.5 py-0.2 font-mono text-[9px] text-[var(--color-fg-muted)]">
                                  {account.providerId === 'local'
                                    ? 'Local HTTP'
                                    : account.authType === 'oauth'
                                    ? 'OAuth'
                                    : 'API Key'}
                                </span>
                              </div>
                              <div className="mt-1 font-mono text-[10px] text-[var(--color-fg-subtle)] truncate">
                                {account.providerName} · {account.defaultModel || 'Default Model'}
                              </div>
                            </div>
                          </div>

                          <div className="flex items-center gap-2 shrink-0">
                            <button
                              type="button"
                              onClick={() => void refreshModels()}
                              className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 text-[11px] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] transition cursor-pointer"
                            >
                              Refresh
                            </button>
                            <button
                              type="button"
                              onClick={() => {
                                if (window.confirm(`Disconnect account "${account.accountName}"?`)) {
                                  void handleDeleteAccount(account.id);
                                }
                              }}
                              className="rounded border border-rose-500/30 bg-rose-500/10 px-2.5 py-1 text-[11px] text-rose-300 hover:bg-rose-500/20 transition cursor-pointer"
                            >
                              Disconnect
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}

            {activeTab === 'routing' && (
              <div className="space-y-6">
                <div>
                  <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">
                    Model Routing & Account Priority
                  </h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">
                    Control which account is chosen first when multiple accounts exist for the same provider, or toggle round-robin load distribution.
                  </p>
                </div>

                {routingError && (
                  <p role="alert" className="text-xs text-rose-400">
                    {routingError}
                  </p>
                )}

                {connectedAccounts.length === 0 ? (
                  <div className="rounded-xl border border-dashed border-[var(--color-border-muted)] p-6 text-center text-xs text-[var(--color-fg-subtle)]">
                    Connect at least one provider account to configure routing policies.
                  </div>
                ) : (
                  <div className="space-y-3">
                    {[...new Set(orderedAccounts.map((a) => a.providerId))].map((service) => {
                      const accounts = orderedAccounts.filter((a) => a.providerId === service);
                      const expanded = expandedProviders[service] ?? true;
                      const enabled = routingPolicies[service]?.round_robin || false;
                      const providerName = accounts[0].providerName;

                      return (
                        <section
                          key={service}
                          aria-label={`${providerName} accounts`}
                          className="overflow-hidden rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)]"
                        >
                          <div className="flex items-center gap-3 py-2 px-3">
                            <button
                              type="button"
                              aria-expanded={expanded}
                              aria-controls={`provider-accounts-${service}`}
                              onClick={() =>
                                setExpandedProviders((previous) => ({
                                  ...previous,
                                  [service]: !expanded,
                                }))
                              }
                              className="flex min-w-0 flex-1 items-center gap-3 rounded-lg p-1 text-left cursor-pointer"
                            >
                              <ProviderBrandIcon provider={service} size={20} />
                              <div className="min-w-0 flex-1">
                                <div className="flex flex-wrap items-center gap-2">
                                  <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
                                    {providerName}
                                  </span>
                                  <span className="rounded border border-[var(--color-border-muted)] px-1.5 py-0.5 font-mono text-[9px] text-[var(--color-fg-muted)]">
                                    {accounts.length} {accounts.length === 1 ? 'account' : 'accounts'}
                                  </span>
                                </div>
                                <p className="mt-0.5 text-[10px] text-[var(--color-fg-muted)]">
                                  {enabled
                                    ? 'Rotate accounts within this provider'
                                    : 'Use the highest-priority eligible account'}
                                </p>
                              </div>
                              <ChevronDown
                                size={15}
                                style={{ transitionDuration: reducePanelMotion ? '0ms' : '240ms' }}
                                className={`shrink-0 text-[var(--color-fg-muted)] transition-transform ${
                                  expanded ? 'rotate-180' : ''
                                }`}
                              />
                            </button>

                            <div className="flex shrink-0 items-center gap-2 border-l border-[var(--color-border-muted)] pl-3">
                              <span className="text-[10px] text-[var(--color-fg-muted)]">Round-robin</span>
                              <button
                                type="button"
                                role="switch"
                                aria-label={`Round-robin for ${providerName}`}
                                aria-checked={enabled}
                                disabled={routingBusy || !routingPolicies[service]}
                                onClick={() =>
                                  void updateRouting(
                                    service,
                                    !enabled,
                                    accounts.map((a) => a.id)
                                  )
                                }
                                className={`relative h-5 w-9 rounded-full border transition-colors disabled:opacity-40 cursor-pointer ${
                                  enabled
                                    ? 'border-emerald-500 bg-emerald-500'
                                    : 'border-[var(--color-border-default)] bg-[var(--color-surface-3)]'
                                }`}
                              >
                                <span
                                  className={`absolute top-0.5 h-3.5 w-3.5 rounded-full bg-white shadow-xs transition-transform ${
                                    enabled ? 'left-0.5 translate-x-4' : 'left-0.5'
                                  }`}
                                />
                              </button>
                              <span
                                className={`w-5 text-[9px] font-mono ${
                                  enabled ? 'text-emerald-400' : 'text-[var(--color-fg-muted)]'
                                }`}
                              >
                                {enabled ? 'On' : 'Off'}
                              </span>
                            </div>
                          </div>

                          <AnimatePresence initial={false}>
                            {expanded && (
                              <motion.div
                                key={service}
                                id={`provider-accounts-${service}`}
                                initial={{ height: 0, opacity: 0 }}
                                animate={{ height: 'auto', opacity: 1 }}
                                exit={{ height: 0, opacity: 0 }}
                                transition={{
                                  duration: reducePanelMotion ? 0 : 0.24,
                                  ease: [0.22, 1, 0.36, 1],
                                }}
                                className="overflow-hidden"
                              >
                                {accounts.map((account, priority) => (
                                  <div
                                    key={account.id}
                                    className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-border-muted)] px-4 py-3"
                                  >
                                    <div className="min-w-0 flex-1">
                                      <div className="text-xs font-semibold flex items-center gap-2 text-[var(--color-editor-fg)]">
                                        <ProviderBrandIcon provider={account.providerId} size={16} />
                                        <span className="truncate">{account.accountName}</span>
                                      </div>
                                    </div>
                                    <div className="flex items-center gap-2">
                                      <span className="text-[10px] font-mono text-[var(--color-fg-muted)]">
                                        Priority {priority + 1}
                                      </span>
                                      <button
                                        type="button"
                                        aria-label={`Increase priority of ${account.accountName}`}
                                        disabled={
                                          routingBusy ||
                                          !routingPolicies[account.providerId] ||
                                          priority === 0
                                        }
                                        onClick={() =>
                                          moveAccount(account.id, account.providerId, -1)
                                        }
                                        className="rounded border border-[var(--color-border-muted)] p-1.5 hover:bg-[var(--color-surface-2)] disabled:opacity-30 cursor-pointer"
                                      >
                                        <ArrowUp size={12} />
                                      </button>
                                      <button
                                        type="button"
                                        aria-label={`Decrease priority of ${account.accountName}`}
                                        disabled={
                                          routingBusy ||
                                          !routingPolicies[account.providerId] ||
                                          priority === accounts.length - 1
                                        }
                                        onClick={() =>
                                          moveAccount(account.id, account.providerId, 1)
                                        }
                                        className="rounded border border-[var(--color-border-muted)] p-1.5 hover:bg-[var(--color-surface-2)] disabled:opacity-30 cursor-pointer"
                                      >
                                        <ArrowDown size={12} />
                                      </button>
                                    </div>
                                  </div>
                                ))}
                              </motion.div>
                            )}
                          </AnimatePresence>
                        </section>
                      );
                    })}
                  </div>
                )}
              </div>
            )}

            {activeTab === 'subagents' && <SubAgentsPane />}
            {activeTab === 'quotas' && <CostQuotasPane />}
          </div>
        </div>
      </div>
    </div>
  );
};
