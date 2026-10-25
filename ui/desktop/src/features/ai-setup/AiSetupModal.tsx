import React, { useEffect, useState } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import {
  Cpu,
  Key,
  Bot,
  Coins,
  ArrowUp,
  ArrowDown,
  ChevronDown,
  X,
  Layers,
  Plug,
  Server,
} from 'lucide-react';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import { LocalAiUnifiedPane } from './LocalAiUnifiedPane';
import { CloudProvidersUnifiedPane } from './CloudProvidersUnifiedPane';
import { SubAgentsPane } from './SubAgentsPane';
import { DaemonPane } from '@/features/settings/DaemonPane';
import { RemoteFleetWorkbenchPane } from '@/components/workbenches/shared/RemoteFleetWorkbenchPane';
import type { CapabilityDescriptor } from '@/types/domain';
import { CostQuotasPane } from './CostQuotasPane';

export type AiSetupTab = 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas' | 'api_keys' | 'oauth' | 'tools' | 'environments' | 'diagnostics';

export interface AiSetupModalProps {
  isOpen: boolean;
  embedded?: boolean;
  onClose: () => void;
  initialTab?: AiSetupTab;
  onShowToast: (msg: string) => void;
}

export const AiSetupModal: React.FC<AiSetupModalProps> = ({
  isOpen,
  embedded = false,
  onClose,
  initialTab = 'local',
  onShowToast,
}) => {
  const {
    connectedAccounts,
    appearance,
    openPlugins,
    openSkills,
  } = useAppContext();

  const [activeTab, setActiveTab] = useState<AiSetupTab>(initialTab);
  const [capabilities, setCapabilities] = useState<CapabilityDescriptor[]>([]);
  const [environmentError, setEnvironmentError] = useState('');
  const [environmentLoading, setEnvironmentLoading] = useState(false);
  useEffect(() => {
    if (!isOpen || activeTab !== 'environments') return;
    let live = true;
    setEnvironmentLoading(true); setEnvironmentError(''); setCapabilities([]);
    daemonClient.listCapabilities().then(value => { if (live) setCapabilities(value); }, cause => { if (live) setEnvironmentError(String(cause?.message || cause)); }).finally(() => { if (live) setEnvironmentLoading(false); });
    return () => { live = false; };
  }, [isOpen, activeTab]);
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
    if (!isOpen || activeTab !== 'routing') return;
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
  }, [isOpen, activeTab, routingServices]);

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

  const hasLMStudio = connectedAccounts.some(
    (a) => a.providerId === 'lmstudio' || a.endpointUrl?.includes('1234')
  );

  const cloudAccounts = connectedAccounts.filter((a) => a.providerId !== 'lmstudio');

  const navItems = [
    { id: 'tools' as const, label: 'Tools & Extensions', description: 'Plugins, skills and MCP connections', icon: Plug, badge: undefined },
    { id: 'environments' as const, label: 'Execution Environments', description: 'Hosts and remote execution capabilities', icon: Server, badge: undefined },
    {
      id: 'local' as const,
      label: 'Local Models & Runtimes',
      description: 'LM Studio, Apple Silicon Metal & multi-model VRAM',
      icon: Cpu,
      badge: hasLMStudio ? 'LM Studio' : 'Local Engine',
    },
    {
      id: 'cloud' as const,
      label: 'Cloud Providers & Keys',
      description: 'Claude, OpenAI, Gemini, DeepSeek & OAuth PKCE',
      icon: Key,
      badge: cloudAccounts.length > 0 ? `${cloudAccounts.length}` : undefined,
    },
    {
      id: 'routing' as const,
      label: 'Model Routing & Order',
      description: 'Priority ordering, round-robin & failover',
      icon: Layers,
    },
    {
      id: 'subagents' as const,
      label: 'Agent Workers & Harnesses',
      description: 'Daemon-backed workforce & runtime roles',
      icon: Bot,
    },
    {
      id: 'quotas' as const,
      label: 'Usage & Cost Limits',
      description: 'Token tracking & session budget headroom',
      icon: Coins,
    },
    { id: 'diagnostics' as const, label: 'Diagnostics', description: 'Daemon connectivity and local runtime status', icon: Server, badge: undefined },
  ];

  return (
    <div className={embedded ? "flex h-full min-h-0 flex-col bg-canvas" : "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs"}>
      <div className={embedded ? "flex h-full min-h-0 w-full flex-col overflow-hidden bg-canvas" : "flex h-[740px] max-h-[92vh] w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] shadow-2xl"}>
        {/* Header: Clean, Crisp, Executive */}
        <header className="flex h-15 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-6">
          <div className="flex items-center gap-3">
            <div className="flex size-8 items-center justify-center rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]">
              <Cpu className="size-4.5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-sm font-semibold text-[var(--color-editor-fg)]">Connections & Tools</h1>
                <span className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-0.5 font-mono text-[9px] text-[var(--color-fg-muted)]">
                  Accounts · Agents · Tools
                </span>
              </div>
              <p className="text-[11px] text-[var(--color-fg-muted)]">
                Manage the connections and executors available to your workbenches
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close Connections & Tools"
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
              Manage connections
            </div>
            {navItems.map(({ id, label, description, icon: Icon, badge }) => (
              <button
                key={id}
                type="button"
                onClick={() => setActiveTab(id)}
                className={`flex w-full items-start gap-2.5 rounded-xl p-2.5 text-left transition cursor-pointer ${
                  activeTab === id
                    ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] border border-[var(--color-border-default)] font-medium shadow-xs'
                    : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]/60 hover:text-[var(--color-editor-fg)] border border-transparent'
                }`}
              >
                <Icon
                  className={`h-4 w-4 shrink-0 mt-0.5 ${
                    activeTab === id ? 'text-[var(--color-editor-fg)]' : 'text-[var(--color-fg-subtle)]'
                  }`}
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center justify-between gap-1">
                    <span className="text-xs truncate">{label}</span>
                    {badge && (
                      <span className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-3)] px-1.5 py-0.2 font-mono text-[9px] text-[var(--color-fg-muted)] shrink-0">
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
            {activeTab === 'tools' && <section className="space-y-4"><h2 className="text-sm font-semibold">Tools & Extensions</h2><p className="text-xs text-fg-muted">Manage installed plugins, skills and MCP connections in their existing managers.</p><div className="flex gap-3"><button type="button" className="rounded border border-border-default px-3 py-2 text-xs" onClick={() => { onClose(); openPlugins(); }}>Plugins & MCP</button><button type="button" className="rounded border border-border-default px-3 py-2 text-xs" onClick={() => { onClose(); openSkills(); }}>Skills</button></div></section>}
            {activeTab === 'diagnostics' && <DaemonPane />}
            {activeTab === 'environments' && (() => {
              const fleet = capabilities.find(capability => (capability.resourceId || capability.resource_id) === 'fleet');
              return environmentLoading ? <p role="status" className="text-xs text-fg-muted">Checking execution environments…</p> : environmentError ? <div role="alert" className="space-y-3 text-xs"><p>{environmentError}</p><button type="button" className="rounded border border-border-default px-3 py-2" onClick={() => { setActiveTab('diagnostics'); }}>Open diagnostics</button></div> : <RemoteFleetWorkbenchPane supportedOperations={!fleet || !['available', 'degraded'].includes(fleet.status?.type) ? [] : fleet.supportedOperations || fleet.supported_operations || []} capabilityReason={fleet?.description || 'The daemon does not advertise remote execution operations.'} onShowToast={onShowToast} />;
            })()}
            {activeTab === 'local' && (
              <LocalAiUnifiedPane
                onShowToast={onShowToast}
                onNavigateToWorkbench={onClose}
              />
            )}

            {(activeTab === 'cloud' || (activeTab as string) === 'api_keys' || (activeTab as string) === 'oauth') && (
              <CloudProvidersUnifiedPane
                onShowToast={onShowToast}
                initialSubTab={(activeTab as string) === 'oauth' ? 'oauth' : 'api_keys'}
              />
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
