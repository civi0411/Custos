import React, { useEffect, useState } from 'react';
import {
  Cpu,
  Database,
  Info,
  Keyboard,
  KeyRound,
  Monitor,
  Plus,
  Server,
  Shield,
  SlidersHorizontal,
  X,
  Layers,
  BarChart3,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import { formatKeyCombo } from '@/lib/utils';
import { CombosManager } from '@/components/providers/CombosManager';
import { UsageMonitor } from '@/components/providers/UsageMonitor';

export interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialTab?: string;
  onOpenProviders?: () => void;
  onShowToast: (msg: string) => void;
}

type SettingsTab = 'general' | 'appearance' | 'shortcuts' | 'providers' | 'runtime' | 'security' | 'storage' | 'about';

type AppearancePreferences = {
  density: 'comfortable' | 'compact';
  reduceMotion: boolean;
};
const APPEARANCE_KEY = 'custos.appearance.preferences';

const readAppearance = (): AppearancePreferences => {
  try {
    const value = localStorage.getItem(APPEARANCE_KEY);
    if (value) return JSON.parse(value) as AppearancePreferences;
  } catch {
    // Invalid local preferences fall back to safe defaults.
  }
  return { density: 'comfortable', reduceMotion: false };
};

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
  onShowToast,
}) => {
  const {
    uiScale,
    handleSetUiScale,
    providers,
    clientKeys,
    connectedAccounts,
    handleDeleteAccount,
    setIsAddProviderOpen,
    handleGenerateClientKey,
    handleRevokeClientKey,
    handleDeleteProvider,
  } = useAppContext();
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');
  const [providersSubTab, setProvidersSubTab] = useState<'providers' | 'combos' | 'usage'>('providers');
  const [appearance, setAppearance] = useState<AppearancePreferences>(readAppearance);
  const [daemonOnline, setDaemonOnline] = useState<boolean | null>(null);
  const [checkingDaemon, setCheckingDaemon] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    const allowed: SettingsTab[] = ['general', 'appearance', 'shortcuts', 'providers', 'runtime', 'security', 'storage', 'about'];
    setActiveTab(allowed.includes(initialTab as SettingsTab) ? initialTab as SettingsTab : 'general');
    void checkDaemon();
  }, [isOpen, initialTab]);

  if (!isOpen) return null;

  const checkDaemon = async () => {
    setCheckingDaemon(true);
    const online = await daemonClient.checkHealth();
    setDaemonOnline(online);
    setCheckingDaemon(false);
  };

  const updateAppearance = (next: AppearancePreferences) => {
    setAppearance(next);
    localStorage.setItem(APPEARANCE_KEY, JSON.stringify(next));
    document.documentElement.dataset.density = next.density;
    document.documentElement.dataset.reduceMotion = String(next.reduceMotion);
    onShowToast('Appearance preference saved on this device');
  };

  const nav = [
    { id: 'general' as const, label: 'General', icon: SlidersHorizontal },
    { id: 'appearance' as const, label: 'Appearance', icon: Monitor },
    { id: 'shortcuts' as const, label: 'Shortcuts', icon: Keyboard },
    { id: 'providers' as const, label: 'Providers', icon: KeyRound },
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

            {activeTab === 'appearance' && (
              <div className="space-y-3">
                <div className="mb-5">
                  <h2 className="text-sm font-semibold">Appearance</h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">Graphite-on-paper is the shared visual system for all workbenches.</p>
                </div>
                <SettingRow title="Interface scale" description="Saved locally for this device.">
                  <div className="flex items-center gap-2">
                    <input type="range" min="75" max="150" step="5" value={uiScale} onChange={(event) => handleSetUiScale(Number(event.target.value))} />
                    <span className="w-10 text-right font-mono text-[11px]">{uiScale}%</span>
                  </div>
                </SettingRow>
                <SettingRow title="Density" description="Changes spacing while preserving the same information architecture.">
                  <select value={appearance.density} onChange={(event) => updateAppearance({ ...appearance, density: event.target.value as AppearancePreferences['density'] })}>
                    <option value="comfortable">Comfortable</option>
                    <option value="compact">Compact</option>
                  </select>
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
                {/* OmniRoute Sub-navigation */}
                <div className="flex border-b border-[var(--color-border-muted)] pb-2 gap-2 text-xs">
                  <button
                    type="button"
                    onClick={() => setProvidersSubTab('providers')}
                    className={`pb-1.5 px-3 font-medium transition border-b-2 flex items-center gap-1.5 cursor-pointer ${
                      providersSubTab === 'providers'
                        ? 'border-[var(--color-editor-fg)] text-[var(--color-editor-fg)] font-semibold'
                        : 'border-transparent text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                    }`}
                  >
                    <Server className="h-3.5 w-3.5" />
                    <span>Providers & Accounts</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setProvidersSubTab('combos')}
                    className={`pb-1.5 px-3 font-medium transition border-b-2 flex items-center gap-1.5 cursor-pointer ${
                      providersSubTab === 'combos'
                        ? 'border-[var(--color-editor-fg)] text-[var(--color-editor-fg)] font-semibold'
                        : 'border-transparent text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                    }`}
                  >
                    <Layers className="h-3.5 w-3.5 text-workbench-accent" />
                    <span>Routing Combos</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setProvidersSubTab('usage')}
                    className={`pb-1.5 px-3 font-medium transition border-b-2 flex items-center gap-1.5 cursor-pointer ${
                      providersSubTab === 'usage'
                        ? 'border-[var(--color-editor-fg)] text-[var(--color-editor-fg)] font-semibold'
                        : 'border-transparent text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                    }`}
                  >
                    <BarChart3 className="h-3.5 w-3.5 text-emerald-400" />
                    <span>Usage & Quotas</span>
                  </button>
                </div>

                {providersSubTab === 'combos' && <CombosManager />}
                {providersSubTab === 'usage' && <UsageMonitor />}
                {providersSubTab === 'providers' && (
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

                    {/* Section 1: Backend Providers */}
                    <div className="space-y-2">
                      <div className="text-[11px] font-semibold uppercase font-mono tracking-wider text-[var(--color-fg-subtle)]">
                        Persisted Daemon Providers
                      </div>
                      {providers.length === 0 && <p className="rounded-lg border border-[var(--color-border-muted)] p-4 text-xs text-[var(--color-fg-muted)]">No provider records returned by the daemon.</p>}
                      {providers.map((provider) => (
                        <div key={provider.id} className="flex items-center justify-between rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3">
                          <div>
                            <div className="text-xs font-semibold flex items-center gap-2">
                              <span>{provider.name}</span>
                              {provider.fastMode && (
                                <span className="text-[9px] bg-amber-500/10 text-amber-400 border border-amber-500/20 px-1 py-0.2 rounded font-mono">
                                  Fast
                                </span>
                              )}
                            </div>
                            <div className="mt-1 font-mono text-[10px] text-[var(--color-fg-muted)] flex items-center gap-1.5">
                              <span>{provider.model}</span>
                              {provider.contextWindow && (
                                <span className="text-[9px] px-1 rounded bg-[var(--color-surface-2)] text-[var(--color-fg-subtle)]">
                                  {Math.round(provider.contextWindow / 1000)}k ctx
                                </span>
                              )}
                              <span>· {provider.statusLabel}</span>
                            </div>
                          </div>
                          <div className="flex items-center gap-3">
                            <span className="font-mono text-[10px] text-[var(--color-fg-subtle)]">{provider.apiKey}</span>
                            <button
                              type="button"
                              onClick={() => {
                                if (window.confirm(`Xóa provider "${provider.name}"?`)) {
                                  void handleDeleteProvider(provider.id);
                                }
                              }}
                              className="text-[10px] text-rose-400 hover:text-rose-300 transition cursor-pointer"
                            >
                              Delete
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>

                    {/* Section 2: Connected Accounts (OAuth & Multi-key) */}
                    <div className="space-y-2 pt-2">
                      <div className="text-[11px] font-semibold uppercase font-mono tracking-wider text-[var(--color-fg-subtle)] flex items-center justify-between">
                        <span>Connected Identity & OAuth Accounts</span>
                        <span className="text-[10px] lowercase text-[var(--color-fg-subtle)]">{connectedAccounts.length} accounts</span>
                      </div>
                      {connectedAccounts.map((account) => (
                        <div key={account.id} className="flex items-center justify-between rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3">
                          <div>
                            <div className="text-xs font-semibold flex items-center gap-2">
                              <span>{account.accountName}</span>
                              <span className="text-[9px] px-1.5 py-0.2 rounded font-mono border bg-surface-2 text-fg-editor border-border-default">
                                {account.authType === 'oauth' ? 'OAuth 2.0' : 'API Key'}
                              </span>
                              {account.oauthTier && (
                                <span className="text-[9px] px-1.5 py-0.2 rounded font-mono text-indigo-400 bg-indigo-500/10 border border-indigo-500/20">
                                  {account.oauthTier}
                                </span>
                              )}
                            </div>
                            <div className="mt-1 font-mono text-[10px] text-[var(--color-fg-muted)] flex items-center gap-1.5">
                              <span>{account.providerName}</span>
                              {account.oauthEmail && <span>· {account.oauthEmail}</span>}
                              <span>· {account.defaultModel}</span>
                            </div>
                          </div>
                          <div className="flex items-center gap-3">
                            <span
                              className="w-2 h-2 rounded-full shrink-0"
                              style={{ backgroundColor: account.badgeColor || '#10b981' }}
                              title={account.statusLabel || account.status}
                            />
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
                )}
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
