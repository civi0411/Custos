import { NativeSelect } from '@/components/common/NativeSelect';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import React, { useEffect, useState } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import {
  Cpu,
  ChevronDown,
  ArrowUp,
  ArrowDown,
  Database,
  Info,
  Keyboard,
  KeyRound,
  Monitor,
  Sun,
  Moon,
  Plus,
  Server,
  Shield,
  SlidersHorizontal,
  X,
  Bot,
  Coins,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import { formatKeyCombo } from '@/lib/utils';
import { SubAgentsPane } from '@/app/settings/SubAgentsPane';
import { CostQuotasPane } from '@/app/settings/CostQuotasPane';
import { DaemonPane } from '@/app/settings/DaemonPane';

export interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialTab?: string;
  onOpenProviders?: () => void;
  onShowToast: (msg: string) => void;
}

type SettingsTab =
  | 'general'
  | 'subagents'
  | 'quotas'
  | 'daemon'
  | 'appearance'
  | 'shortcuts'
  | 'providers'
  | 'runtime'
  | 'security'
  | 'storage'
  | 'about';

const SettingRow: React.FC<{
  title: string;
  description: string;
  children: React.ReactNode;
}> = ({ title, description, children }) => (
  <div className="flex items-center justify-between gap-6 rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4">
    <div>
      <div className="text-xs font-semibold text-[var(--color-editor-fg)]">{title}</div>
      <div className="mt-1 text-[11px] leading-5 text-[var(--color-fg-muted)]">{description}</div>
    </div>
    <div className="shrink-0">{children}</div>
  </div>
);

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  onClose,
  initialTab,
  onShowToast: _onShowToast,
}) => {
  const {
    appearance,
    resolvedTheme,
    updateAppearance,
    uiScale,
    handleSetUiScale,
    clientKeys,
    connectedAccounts,
    handleDeleteAccount,
    setIsAddProviderOpen,
    handleGenerateClientKey,
    handleRevokeClientKey,
    refreshModels,
  } = useAppContext();
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');
  const [daemonOnline, setDaemonOnline] = useState<boolean | null>(null);
  const [checkingDaemon, setCheckingDaemon] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    const allowed: SettingsTab[] = ['general', 'subagents', 'quotas', 'daemon', 'appearance', 'shortcuts', 'providers', 'runtime', 'security', 'storage', 'about'];
    setActiveTab(allowed.includes(initialTab as SettingsTab) ? initialTab as SettingsTab : 'general');
    void checkDaemon();
  }, [isOpen, initialTab]);

  const [routingPolicies, setRoutingPolicies] = useState<Record<string, { round_robin: boolean; account_order: string[] }>>({});
  const [routingBusy, setRoutingBusy] = useState(false);
  const [routingError, setRoutingError] = useState('');
  const [expandedProviders, setExpandedProviders] = useState<Record<string, boolean>>({});
  const systemReducedMotion = useReducedMotion();
  const reducePanelMotion = appearance.reduceMotion || systemReducedMotion;
  const routingServices = [...new Set(connectedAccounts.map(a => a.providerId))].sort().join(',');
  useEffect(() => {
    if (!isOpen) return;
    let cancelled = false;
    setRoutingError('');
    Promise.all([...new Set(connectedAccounts.map(a => a.providerId))].map(async service => [service, await daemonClient.getProviderRouting(service)] as const))
      .then(values => { if (!cancelled) setRoutingPolicies(Object.fromEntries(values)); })
      .catch(error => { if (!cancelled) setRoutingError(String(error?.message || error)); });
    return () => { cancelled = true; };
  }, [isOpen, routingServices]);
  const orderedAccounts = [...connectedAccounts].sort((a,b) => {
    const serviceOrder = a.providerId.localeCompare(b.providerId);
    if (serviceOrder) return serviceOrder;
    const order = routingPolicies[a.providerId]?.account_order || [];
    const position = (id: string) => { const index = order.indexOf(id); return index < 0 ? Number.MAX_SAFE_INTEGER : index; };
    return position(a.id) - position(b.id);
  });
  const updateRouting = async (service: string, enabled: boolean, order: string[]) => {
    setRoutingBusy(true); setRoutingError('');
    try {
      const policy = await daemonClient.saveProviderRouting(service, enabled, order);
      setRoutingPolicies(previous => ({ ...previous, [service]: policy }));
    } catch (error: any) { setRoutingError(error?.message || String(error)); }
    finally { setRoutingBusy(false); }
  };
  const moveAccount = (id: string, service: string, direction: number) => {
    const order = orderedAccounts.filter(a => a.providerId === service).map(a => a.id);
    const from = order.indexOf(id), to = from + direction;
    if (from < 0 || to < 0 || to >= order.length) return;
    [order[from], order[to]] = [order[to], order[from]];
    void updateRouting(service, routingPolicies[service]?.round_robin || false, order);
  };

  if (!isOpen) return null;

  const checkDaemon = async () => {
    setCheckingDaemon(true);
    const online = await daemonClient.checkHealth();
    setDaemonOnline(online);
    setCheckingDaemon(false);
  };

  const nav = [
    { id: 'general' as const, label: 'General', icon: SlidersHorizontal },
    { id: 'providers' as const, label: 'Models & accounts', icon: KeyRound },
    { id: 'subagents' as const, label: 'Agent runtimes', icon: Bot },
    { id: 'quotas' as const, label: 'Usage & limits', icon: Coins },
    { id: 'daemon' as const, label: 'Daemon status', icon: Server },
    { id: 'appearance' as const, label: 'Appearance', icon: Monitor },
    { id: 'shortcuts' as const, label: 'Shortcuts', icon: Keyboard },
    { id: 'runtime' as const, label: 'Runtime', icon: Cpu },
    { id: 'security' as const, label: 'Security', icon: Shield },
    { id: 'storage' as const, label: 'Storage', icon: Database },
    { id: 'about' as const, label: 'About', icon: Info },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="flex h-[650px] max-h-[92vh] w-full max-w-4xl flex-col overflow-hidden rounded-xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] shadow-2xl">
        <header className="flex h-14 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-5">
          <div>
            <h1 className="text-sm font-semibold text-[var(--color-editor-fg)]">Settings</h1>
            <p className="text-[11px] text-[var(--color-fg-muted)]">Device preferences and daemon-backed configuration</p>
          </div>
          <button type="button" onClick={onClose} aria-label="Close settings" className="rounded-md p-1.5 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]">
            <X className="h-4 w-4" />
          </button>
        </header>

        <div className="flex min-h-0 flex-1">
          <nav className="w-48 shrink-0 border-r border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3">
            {nav.map(({ id, label, icon: Icon }) => (
              <button
                key={id}
                type="button"
                onClick={() => setActiveTab(id)}
                className={`mb-0.5 flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-xs ${
                  activeTab === id
                    ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
                    : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]'
                }`}
              >
                <Icon className="h-3.5 w-3.5" />
                {label}
              </button>
            ))}
          </nav>

          <div className="min-w-0 flex-1 overflow-y-auto p-6">
            {activeTab === 'general' && (
              <div className="space-y-3">
                <div className="mb-5">
                  <h2 className="text-sm font-semibold">General</h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">Only settings with a real persistence owner are editable.</p>
                </div>
                <SettingRow title="Custos daemon" description="Required for Tasks, sessions, providers, Research records and execution.">
                  <div className="flex items-center gap-2">
                    <span className={`h-2 w-2 rounded-full ${daemonOnline ? 'bg-emerald-500' : 'bg-rose-500'}`} />
                    <span className="text-[11px] text-[var(--color-fg-muted)]">{daemonOnline === null ? 'Unknown' : daemonOnline ? 'Connected' : 'Offline'}</span>
                    <button type="button" onClick={() => void checkDaemon()} disabled={checkingDaemon} className="rounded-md border border-[var(--color-border-default)] px-2 py-1 text-[10px] disabled:opacity-50">
                      {checkingDaemon ? 'Checking…' : 'Check'}
                    </button>
                  </div>
                </SettingRow>
                <SettingRow title="Workspace restoration" description="Layout restoration is local presentation state and never resumes a run automatically.">
                  <span className="text-[10px] uppercase tracking-wider text-[var(--color-fg-subtle)]">Planned</span>
                </SettingRow>
              </div>
            )}

            {activeTab === 'subagents' && <SubAgentsPane />}
            {activeTab === 'quotas' && <CostQuotasPane />}
            {activeTab === 'daemon' && <DaemonPane />}

            {activeTab === 'appearance' && (
              <div className="space-y-3">
                <div className="mb-5">
                  <h2 className="text-sm font-semibold">Appearance</h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">Choose your theme, spacing and motion preferences for this device.</p>
                </div>
                <SettingRow title="Theme" description={`System follows your operating system. Currently using ${resolvedTheme} appearance.`}>
                  <fieldset className="flex gap-2" aria-label="Theme">
                    {([{ value: 'light', label: 'Light', icon: Sun }, { value: 'dark', label: 'Dark', icon: Moon }, { value: 'system', label: 'System', icon: Monitor }] as const).map(({ value, label, icon: Icon }) => (
                      <label key={value} className={`flex w-20 cursor-pointer flex-col items-center gap-2 rounded-lg border px-3 py-3 has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-[var(--color-accent)] ${appearance.theme === value ? 'border-[var(--color-accent)] bg-[var(--color-accent-subtle)]' : 'border-[var(--color-border-default)] hover:bg-[var(--color-surface-2)]'}`}>
                        <input className="sr-only" type="radio" name="appearance-theme" aria-label={`${label} theme`} value={value} checked={appearance.theme === value} onChange={() => updateAppearance({ theme: value })} />
                        <Icon className="h-4 w-4" /><span className="text-xs">{label}</span>
                      </label>
                    ))}
                  </fieldset>
                </SettingRow>
                <SettingRow title="Interface scale" description="Saved locally for this device.">
                  <div className="flex items-center gap-2">
                    <input aria-label="Interface scale" style={{ '--range-progress': `${(uiScale - 70) / 90 * 100}%` } as React.CSSProperties} type="range" min="70" max="160" step="10" value={uiScale} onChange={(event) => handleSetUiScale(Number(event.target.value))} />
                    <span className="w-10 text-right font-mono text-[11px]">{uiScale}%</span>
                  </div>
                </SettingRow>
                <SettingRow title="Density" description="Changes spacing while preserving the same information architecture.">
                  <NativeSelect value={appearance.density} onChange={(event) => updateAppearance({ ...appearance, density: event.target.value as 'comfortable' | 'compact' })}>
                    <option value="comfortable">Comfortable</option>
                    <option value="compact">Compact</option>
                  </NativeSelect>
                </SettingRow>
                <SettingRow title="Reduce motion" description="Disables nonessential transitions and attention animations.">
                  <input type="checkbox" checked={appearance.reduceMotion} onChange={(event) => updateAppearance({ ...appearance, reduceMotion: event.target.checked })} />
                </SettingRow>
              </div>
            )}

            {activeTab === 'shortcuts' && (
              <div className="space-y-4">
                <div>
                  <h2 className="text-sm font-semibold">Keyboard Shortcuts</h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">
                    Fast access shortcuts adapted to your operating system.
                  </p>
                </div>

                <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] divide-y divide-[var(--color-border-muted)] overflow-hidden">
                  {[
                    { action: 'Switch to Copilot workbench', key: formatKeyCombo({ ctrlOrCmd: true, key: '1' }) },
                    { action: 'Switch to Coding ADE workbench', key: formatKeyCombo({ ctrlOrCmd: true, key: '2' }) },
                    { action: 'Switch to Research Lab workbench', key: formatKeyCombo({ ctrlOrCmd: true, key: '3' }) },
                    { action: 'Toggle Sessions sidebar', key: formatKeyCombo({ ctrlOrCmd: true, key: 'B' }) },
                    { action: 'Toggle Resource Canvas', key: formatKeyCombo({ ctrlOrCmd: true, shift: true, key: 'B' }) },
                    { action: 'Maximize Resource Canvas', key: formatKeyCombo({ ctrlOrCmd: true, shift: true, key: 'F' }) },
                    { action: 'Quick Jump Command Palette', key: formatKeyCombo({ ctrlOrCmd: true, key: 'K' }) },
                    { action: 'Create New Task / Session', key: formatKeyCombo({ ctrlOrCmd: true, key: 'N' }) },
                    { action: 'Open Preferences & Settings', key: formatKeyCombo({ ctrlOrCmd: true, key: ',' }) },
                    { action: 'Zoom Interface In', key: `${formatKeyCombo({ ctrlOrCmd: true, key: '+' })}` },
                    { action: 'Zoom Interface Out', key: `${formatKeyCombo({ ctrlOrCmd: true, key: '-' })}` },
                    { action: 'Reset Interface Zoom (100%)', key: `${formatKeyCombo({ ctrlOrCmd: true, key: '0' })}` },
                  ].map((s) => (
                    <div key={s.action} className="flex items-center justify-between px-4 py-2.5 hover:bg-[var(--color-surface-2)] transition">
                      <span className="text-xs text-[var(--color-editor-fg)]">{s.action}</span>
                      <kbd className="px-2 py-0.5 rounded font-mono text-[10px] bg-[var(--color-canvas)] border border-[var(--color-border-default)] text-[var(--color-editor-fg)] shadow-xs">
                        {s.key}
                      </kbd>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {activeTab === 'providers' && (
              <div className="space-y-4">
                  <div className="space-y-6">
                    <div className="flex items-start justify-between">
                      <div>
                        <h2 className="text-sm font-semibold">Configured AI Providers & Accounts</h2>
                        <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">OmniRoute / 9Router architecture: Multi-account credentials and live model routes.</p>
                      </div>
                      <button type="button" onClick={() => setIsAddProviderOpen(true)} className="flex items-center gap-1.5 rounded-md bg-[var(--color-editor-fg)] px-3 py-1.5 text-[11px] font-semibold text-[var(--color-canvas)] cursor-pointer">
                        <Plus className="h-3.5 w-3.5" /> Connect Provider
                      </button>
                    </div>

                    {/* Section 2: Connected Accounts (OAuth & Multi-key) */}
                    <div className="space-y-2 pt-2">
                      <div className="text-[11px] font-semibold uppercase font-mono tracking-wider text-[var(--color-fg-subtle)] flex items-center justify-between">
                        <span>Connected Identity & OAuth Accounts</span>
                        <span className="text-[10px] lowercase text-[var(--color-fg-subtle)]">{connectedAccounts.length} accounts</span>
                      </div>
                      {connectedAccounts.length === 0 && <p className="rounded-lg border border-[var(--color-border-muted)] p-4 text-xs text-[var(--color-fg-muted)]">No connected accounts. Use Connect Provider to add one.</p>}
                      {routingError && <p role="alert" className="text-xs text-rose-400">{routingError}</p>}
                      {[...new Set(orderedAccounts.map(a => a.providerId))].map(service => {
                        const accounts = orderedAccounts.filter(a => a.providerId === service);
                        const expanded = expandedProviders[service] || false;
                        const enabled = routingPolicies[service]?.round_robin || false;
                        const providerName = accounts[0].providerName;
                        return (
                          <section key={service} aria-label={`${providerName} accounts`} className="overflow-hidden rounded-xl border border-border-muted bg-surface-1">
                            <div className="flex items-center gap-3 py-1 px-2">
                              <button type="button" aria-expanded={expanded} aria-controls={`provider-accounts-${service}`} onClick={() => setExpandedProviders(previous => ({ ...previous, [service]: !expanded }))} className="flex min-w-0 flex-1 items-center gap-3 rounded-lg p-1 text-left">
                                <ProviderBrandIcon provider={service} size={20} />
                                <div className="min-w-0 flex-1">
                                  <div className="flex flex-wrap items-center gap-2"><span className="text-xs font-semibold">{providerName}</span><span className="rounded border border-border-muted px-1.5 py-0.5 font-mono text-[9px] text-fg-muted">{accounts.length} {accounts.length === 1 ? 'account' : 'accounts'}</span></div>
                                  <p className="mt-1 text-[10px] text-fg-muted">{enabled ? 'Rotate accounts within this provider' : 'Use the highest-priority eligible account'}</p>
                                </div>
                                <ChevronDown size={15} style={{ transitionDuration: reducePanelMotion ? '0ms' : '240ms' }} className={`shrink-0 text-fg-muted transition-transform ${expanded ? 'rotate-180' : ''}`} />
                              </button>
                              <div className="flex shrink-0 items-center gap-2 border-l border-border-muted pl-3">
                                <span className="text-[10px] text-fg-muted">Round-robin</span>
                                <button type="button" role="switch" aria-label={`Round-robin for ${providerName}`} aria-checked={enabled} disabled={routingBusy || !routingPolicies[service]} onClick={() => void updateRouting(service, !enabled, accounts.map(a => a.id))} className={`relative h-5 w-9 rounded-full border transition-colors disabled:opacity-40 ${enabled ? 'border-emerald-500 bg-emerald-500' : 'border-border-default bg-surface-3'}`}>
                                  <span className={`absolute top-0.5 h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${enabled ? 'left-0.5 translate-x-4' : 'left-0.5'}`} />
                                </button>
                                <span className={`w-5 text-[9px] font-mono ${enabled ? 'text-emerald-500' : 'text-fg-muted'}`}>{enabled ? 'On' : 'Off'}</span>
                              </div>
                            </div>
                            <AnimatePresence initial={false}>
                            {expanded && <motion.div
                              key={service}
                              id={`provider-accounts-${service}`}
                              initial={{ height: 0, opacity: 0 }}
                              animate={{ height: 'auto', opacity: 1 }}
                              exit={{ height: 0, opacity: 0 }}
                              transition={{ duration: reducePanelMotion ? 0 : 0.24, ease: [0.22, 1, 0.36, 1] }}
                              className="overflow-hidden"
                              inert={!expanded}
                              aria-hidden={!expanded}
                            >
                      {accounts.map((account, priority) => (
                        <div key={account.id} className="flex flex-wrap items-center justify-between gap-3 border-t border-border-muted px-4 py-3">
                          <div className="min-w-0 flex-1">
                            <div className="text-xs font-semibold flex items-center gap-2">
                              <ProviderBrandIcon provider={account.providerId} size={16} />
                              <span className="truncate">{account.accountName}</span>
                              <span className="text-[9px] px-1.5 py-0.2 rounded font-mono border bg-surface-2 text-fg-editor border-border-default">
                                {account.providerId === 'local' ? 'Local' : account.authType === 'oauth' ? 'OAuth 2.0' : 'API Key'}
                              </span>
                              {account.oauthTier && (
                                <span className="text-[9px] px-1.5 py-0.2 rounded font-mono text-indigo-400 bg-indigo-500/10 border border-indigo-500/20">
                                  {account.oauthTier}
                                </span>
                              )}
                            </div>
                            <div className="mt-1 font-mono text-[10px] text-[var(--color-fg-muted)] flex flex-wrap items-center gap-1.5">
                              {account.providerId == 'local' && <span>{account.providerName} · {account.defaultModel}</span>}
                              {account.providerId === 'local' && <span className={account.status === 'active' ? 'text-emerald-400' : 'text-fg-muted'}>· {account.status === 'active' ? 'Online' : 'Offline'}</span>}
                            </div>
                          </div>
                          <div className="flex items-center gap-3">
                            <span className="text-[10px] text-fg-muted">Priority {priority + 1}</span>
                            <button type="button" aria-label={`Increase priority of ${account.accountName}`} disabled={routingBusy || !routingPolicies[account.providerId] || priority === 0} onClick={() => moveAccount(account.id, account.providerId, -1)} className="rounded border border-border-muted p-1.5 hover:bg-surface-2 disabled:opacity-30"><ArrowUp size={12} /></button>
                            <button type="button" aria-label={`Decrease priority of ${account.accountName}`} disabled={routingBusy || !routingPolicies[account.providerId] || priority === accounts.length - 1} onClick={() => moveAccount(account.id, account.providerId, 1)} className="rounded border border-border-muted p-1.5 hover:bg-surface-2 disabled:opacity-30"><ArrowDown size={12} /></button>

                            <span
                              className="w-2 h-2 rounded-full shrink-0"
                              style={{ backgroundColor: account.badgeColor || '#10b981' }}
                              title={account.providerId === 'local' ? (account.status === 'active' ? 'Online' : 'Offline') : account.statusLabel || account.status}
                            />
                            {account.providerId === 'local' && account.status !== 'active' && <button type="button" onClick={() => void refreshModels()} className="provider-oauth-action rounded px-2 py-1 text-[10px]">Retry</button>}
                            <button
                              type="button"
                              onClick={() => {
                                if (window.confirm(`Disconnect account "${account.accountName}"?`)) {
                                  handleDeleteAccount(account.id);
                                }
                              }}
                              className="text-[10px] text-rose-400 hover:text-rose-300 transition cursor-pointer"
                            >
                              Disconnect
                            </button>
                          </div>
                        </div>
                      ))}
                            </motion.div>}
                            </AnimatePresence>
                          </section>
                        );
                      })}
                    </div>

                    {/* Section 3: Local Gateway Keys */}
                    <div className="pt-2">
                      <div className="flex items-center justify-between">
                        <div>
                          <div className="text-xs font-semibold">Local API Gateway Keys</div>
                          <div className="text-[11px] text-[var(--color-fg-muted)]">Used by authorized local SDK and CLI integrations.</div>
                        </div>
                        <button type="button" onClick={() => void handleGenerateClientKey()} className="rounded-md border border-[var(--color-border-default)] px-2.5 py-1.5 text-[11px] cursor-pointer">Generate key</button>
                      </div>
                      <div className="mt-2 space-y-2">
                        {clientKeys.map((key) => (
                          <div key={key.id} className="flex items-center justify-between rounded-lg border border-[var(--color-border-muted)] p-3">
                            <div>
                              <div className="text-xs font-medium">{key.name}</div>
                              <div className="font-mono text-[10px] text-[var(--color-fg-subtle)]">{key.token}</div>
                            </div>
                            <button type="button" onClick={() => void handleRevokeClientKey(key.id)} className="text-[10px] text-rose-400 cursor-pointer">Revoke</button>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
              </div>
            )}

            {activeTab === 'runtime' && (
              <div className="space-y-3">
                <h2 className="mb-5 text-sm font-semibold">Runtime and compute</h2>
                <SettingRow title="Model execution" description="Provider/model selection belongs to a versioned route plan and attempt, not a global fake default.">
                  <span className="text-[10px] text-[var(--color-fg-subtle)]">Daemon policy</span>
                </SettingRow>
                <SettingRow title="Research kernels" description="Local, SSH, Slurm and managed compute need explicit capability profiles and approval before launch.">
                  <span className="text-[10px] uppercase tracking-wider text-[var(--color-fg-subtle)]">Not connected</span>
                </SettingRow>
              </div>
            )}

            {activeTab === 'security' && (
              <div className="space-y-3">
                <h2 className="mb-5 text-sm font-semibold">Security and authority</h2>
                <SettingRow title="Effect permits" description="Permit enforcement is a kernel invariant and cannot be disabled from presentation settings.">
                  <Shield className="h-4 w-4 text-emerald-500" />
                </SettingRow>
                <SettingRow title="Sandbox assurance" description="Assurance is reported per action from the actual execution path.">
                  <span className="text-[10px] text-[var(--color-fg-subtle)]">Per receipt</span>
                </SettingRow>
              </div>
            )}

            {activeTab === 'storage' && (
              <div className="space-y-3">
                <h2 className="mb-5 text-sm font-semibold">Storage</h2>
                <SettingRow title="Canonical state" description="Tasks, authority, effects and Research metadata are owned by daemon persistence.">
                  <Server className="h-4 w-4 text-[var(--color-fg-muted)]" />
                </SettingRow>
                <SettingRow title="Artifacts" description="Content-addressed artifacts and retention require daemon APIs; this client does not delete or invent them.">
                  <Database className="h-4 w-4 text-[var(--color-fg-muted)]" />
                </SettingRow>
              </div>
            )}

            {activeTab === 'about' && (
              <div className="max-w-lg">
                <h2 className="text-sm font-semibold">Custos SADE</h2>
                <p className="mt-3 text-xs leading-6 text-[var(--color-fg-muted)]">
                  A local-first supervised agent development environment with Copilot, Coding and
                  Research workbenches over one Task, authority and evidence model.
                </p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
