import React, { useEffect, useState } from 'react';
import {
  Cpu,
  Database,
  Info,
  KeyRound,
  Monitor,
  Plus,
  Server,
  Shield,
  SlidersHorizontal,
  X,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';

export interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialTab?: string;
  onOpenProviders?: () => void;
  onShowToast: (msg: string) => void;
}

type SettingsTab = 'general' | 'appearance' | 'providers' | 'runtime' | 'security' | 'storage' | 'about';

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
    setIsAddProviderOpen,
    handleGenerateClientKey,
    handleRevokeClientKey,
  } = useAppContext();
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');
  const [appearance, setAppearance] = useState<AppearancePreferences>(readAppearance);
  const [daemonOnline, setDaemonOnline] = useState<boolean | null>(null);
  const [checkingDaemon, setCheckingDaemon] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    const allowed: SettingsTab[] = ['general', 'appearance', 'providers', 'runtime', 'security', 'storage', 'about'];
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

            {activeTab === 'providers' && (
              <div>
                <div className="mb-5 flex items-start justify-between">
                  <div>
                    <h2 className="text-sm font-semibold">Providers and local client keys</h2>
                    <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">Records are loaded from the Custos daemon. Secrets remain redacted.</p>
                  </div>
                  <button type="button" onClick={() => setIsAddProviderOpen(true)} className="flex items-center gap-1.5 rounded-md bg-[var(--color-editor-fg)] px-3 py-1.5 text-[11px] font-semibold text-[var(--color-canvas)]">
                    <Plus className="h-3.5 w-3.5" /> Add provider
                  </button>
                </div>
                <div className="space-y-2">
                  {providers.length === 0 && <p className="rounded-lg border border-[var(--color-border-muted)] p-4 text-xs text-[var(--color-fg-muted)]">No provider records returned by the daemon.</p>}
                  {providers.map((provider) => (
                    <div key={provider.id} className="flex items-center justify-between rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3">
                      <div>
                        <div className="text-xs font-semibold">{provider.name}</div>
                        <div className="mt-1 font-mono text-[10px] text-[var(--color-fg-muted)]">{provider.model} · {provider.statusLabel}</div>
                      </div>
                      <span className="font-mono text-[10px] text-[var(--color-fg-subtle)]">{provider.apiKey}</span>
                    </div>
                  ))}
                </div>
                <div className="mt-6 flex items-center justify-between">
                  <div>
                    <div className="text-xs font-semibold">Local API keys</div>
                    <div className="text-[11px] text-[var(--color-fg-muted)]">Used by authorized local clients.</div>
                  </div>
                  <button type="button" onClick={() => void handleGenerateClientKey()} className="rounded-md border border-[var(--color-border-default)] px-2.5 py-1.5 text-[11px]">Generate key</button>
                </div>
                <div className="mt-2 space-y-2">
                  {clientKeys.map((key) => (
                    <div key={key.id} className="flex items-center justify-between rounded-lg border border-[var(--color-border-muted)] p-3">
                      <div>
                        <div className="text-xs">{key.name}</div>
                        <div className="font-mono text-[10px] text-[var(--color-fg-subtle)]">{key.token}</div>
                      </div>
                      <button type="button" onClick={() => void handleRevokeClientKey(key.id)} className="text-[10px] text-rose-400">Revoke</button>
                    </div>
                  ))}
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
