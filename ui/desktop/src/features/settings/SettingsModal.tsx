import { NativeSelect } from '@/components/common/NativeSelect';
import React, { useEffect, useState } from 'react';
import {
  Database,
  Info,
  Keyboard,
  Monitor,
  Sun,
  Moon,
  Server,
  Shield,
  SlidersHorizontal,
  X,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { formatKeyCombo } from '@/lib/utils';
import { DaemonPane } from './DaemonPane';

export interface SettingsModalProps {
  isOpen: boolean;
  embedded?: boolean;
  onClose: () => void;
  initialTab?: string;
  onOpenProviders?: () => void;
  onShowToast: (msg: string) => void;
}

type SettingsTab =
  | 'general'
  | 'appearance'
  | 'shortcuts'
  | 'daemon'
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
  embedded = false,
  onClose,
  initialTab = 'general',
  onShowToast: _onShowToast,
}) => {
  const {
    appearance,
    resolvedTheme,
    updateAppearance,
    uiScale,
    handleSetUiScale,
    openAiSetup,
    clientKeys,
    handleGenerateClientKey,
    handleRevokeClientKey,
  } = useAppContext();

  const [activeTab, setActiveTab] = useState<SettingsTab>('general');

  useEffect(() => {
    if (!isOpen) return;
    const allowed: SettingsTab[] = [
      'general',
      'appearance',
      'shortcuts',
      'daemon',
      'security',
      'storage',
      'about',
    ];
    setActiveTab(allowed.includes(initialTab as SettingsTab) ? (initialTab as SettingsTab) : 'general');
  }, [isOpen, initialTab]);

  if (!isOpen) return null;

  const navSections = [
    {
      category: 'Preferences',
      items: [
        { id: 'general' as const, label: 'General', icon: SlidersHorizontal },
        { id: 'appearance' as const, label: 'Appearance', icon: Monitor },
        { id: 'shortcuts' as const, label: 'Keyboard Shortcuts', icon: Keyboard },
      ],
    },
    {
      category: 'Advanced',
      items: [
        { id: 'daemon' as const, label: 'Diagnostics', icon: Server },
        { id: 'security' as const, label: 'Privacy & Access', icon: Shield },
        { id: 'storage' as const, label: 'Storage & Artifacts', icon: Database },
        { id: 'about' as const, label: 'About Custos', icon: Info },
      ],
    },
  ];

  return (
    <div className={embedded ? "flex h-full min-h-0 flex-col bg-canvas" : "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs"}>
      <div className={embedded ? "flex h-full min-h-0 w-full flex-col overflow-hidden bg-canvas" : "flex h-[650px] max-h-[92vh] w-full max-w-4xl flex-col overflow-hidden rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] shadow-2xl"}>
        <header className="flex h-14 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-5">
          <div>
            <h1 className="text-sm font-semibold text-[var(--color-editor-fg)]">Settings</h1>
            <p className="text-[11px] text-[var(--color-fg-muted)]">Preferences for working in Custos</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close settings"
            className="rounded-md p-1.5 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] transition cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </header>

        <div className="flex min-h-0 flex-1">
          <nav className="w-52 shrink-0 border-r border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3 space-y-4 overflow-y-auto">
            {navSections.map((section) => (
              <div key={section.category} className="space-y-1">
                <div className="px-2.5 pb-0.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)] font-mono">
                  {section.category}
                </div>
                {section.items.map(({ id, label, icon: Icon }) => (
                  <button
                    key={id}
                    type="button"
                    onClick={() => setActiveTab(id)}
                    className={`flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-xs transition cursor-pointer ${
                      activeTab === id
                        ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium shadow-xs'
                        : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]/60 hover:text-[var(--color-editor-fg)]'
                    }`}
                  >
                    <Icon className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{label}</span>
                  </button>
                ))}
              </div>
            ))}
          </nav>

          <div className="min-w-0 flex-1 overflow-y-auto p-6">
            {activeTab === 'general' && (
              <div className="space-y-4">
                <div>
                  <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">General Preferences</h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">Appearance, navigation and working preferences for this device.</p>
                </div>


                <div className="space-y-3 pt-1">
                  <SettingRow title="Appearance" description="Theme, density, interface scale and motion for every workbench.">
                    <button type="button" onClick={() => setActiveTab('appearance')} className="rounded border border-border-default px-3 py-1.5 text-xs">Customize</button>
                  </SettingRow>
                  <SettingRow title="Keyboard shortcuts" description="Use the same navigation and editing shortcuts across workbenches.">
                    <button type="button" onClick={() => setActiveTab('shortcuts')} className="rounded border border-border-default px-3 py-1.5 text-xs">View shortcuts</button>
                  </SettingRow>
                  <SettingRow title="Models, agents and tools" description="Accounts and execution environments have a separate management surface.">
                    <button type="button" onClick={() => { onClose(); openAiSetup('cloud'); }} className="rounded border border-border-default px-3 py-1.5 text-xs">Connections & Tools</button>
                  </SettingRow>
                  <SettingRow title="Workspace restoration" description="Layout restoration is local presentation state and never resumes a run automatically.">
                    <span className="text-[10px] uppercase tracking-wider text-[var(--color-fg-subtle)] font-mono">
                      Persistent
                    </span>
                  </SettingRow>
                </div>
              </div>
            )}

            {activeTab === 'daemon' && <DaemonPane />}

            {activeTab === 'appearance' && (
              <div className="space-y-4">
                <div>
                  <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">Appearance</h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">
                    Choose your theme, interface scaling and motion preferences for this device.
                  </p>
                </div>
                <div className="space-y-3">
                  <SettingRow title="Theme" description={`System follows your operating system. Currently using ${resolvedTheme} appearance.`}>
                    <fieldset className="flex gap-2" aria-label="Theme">
                      {([
                        { value: 'light', label: 'Light', icon: Sun },
                        { value: 'dark', label: 'Dark', icon: Moon },
                        { value: 'system', label: 'System', icon: Monitor },
                      ] as const).map(({ value, label, icon: Icon }) => (
                        <label
                          key={value}
                          className={`flex w-20 cursor-pointer flex-col items-center gap-2 rounded-lg border px-3 py-3 transition ${
                            appearance.theme === value
                              ? 'border-teal-500 bg-teal-500/10 font-medium'
                              : 'border-[var(--color-border-default)] hover:bg-[var(--color-surface-2)]'
                          }`}
                        >
                          <input
                            className="sr-only"
                            type="radio"
                            name="appearance-theme"
                            aria-label={`${label} theme`}
                            value={value}
                            checked={appearance.theme === value}
                            onChange={() => updateAppearance({ theme: value })}
                          />
                          <Icon className="h-4 w-4" />
                          <span className="text-xs">{label}</span>
                        </label>
                      ))}
                    </fieldset>
                  </SettingRow>

                  <SettingRow title="Interface scale" description="Saved locally for this device display.">
                    <div className="flex items-center gap-2">
                      <input
                        aria-label="Interface scale"
                        type="range"
                        min="70"
                        max="160"
                        step="10"
                        value={uiScale}
                        onChange={(event) => handleSetUiScale(Number(event.target.value))}
                        className="w-32 accent-teal-500 cursor-pointer"
                      />
                      <span className="w-10 text-right font-mono text-[11px] text-[var(--color-editor-fg)]">
                        {uiScale}%
                      </span>
                    </div>
                  </SettingRow>

                  <SettingRow title="Density" description="Changes spacing while preserving the same information architecture.">
                    <NativeSelect
                      value={appearance.density}
                      onChange={(event) => updateAppearance({ ...appearance, density: event.target.value as 'comfortable' | 'compact' })}
                    >
                      <option value="comfortable">Comfortable</option>
                      <option value="compact">Compact</option>
                    </NativeSelect>
                  </SettingRow>

                  <SettingRow title="Reduce motion" description="Disables nonessential transitions and attention animations.">
                    <input
                      type="checkbox"
                      checked={appearance.reduceMotion}
                      onChange={(event) => updateAppearance({ ...appearance, reduceMotion: event.target.checked })}
                      className="accent-teal-500 cursor-pointer"
                    />
                  </SettingRow>
                </div>
              </div>
            )}

            {activeTab === 'shortcuts' && (
              <div className="space-y-4">
                <div>
                  <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">Keyboard Shortcuts</h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">
                    Fast access shortcuts adapted to your operating system.
                  </p>
                </div>

                <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] divide-y divide-[var(--color-border-muted)] overflow-hidden">
                  {[
                    { action: 'Coding workbench', key: formatKeyCombo({ alt: true, key: '1' }) },
                    { action: 'Research workbench', key: formatKeyCombo({ alt: true, key: '2' }) },
                    { action: 'Connections & Tools', key: formatKeyCombo({ ctrlOrCmd: true, key: '3' }) },
                    { action: 'Copilot workbench', key: formatKeyCombo({ alt: true, key: '3' }) },
                    { action: 'Open Preferences & Settings', key: formatKeyCombo({ ctrlOrCmd: true, key: ',' }) },
                    { action: 'Quick Jump Command Palette', key: formatKeyCombo({ ctrlOrCmd: true, key: 'K' }) },
                    { action: 'Create New Task / Session', key: formatKeyCombo({ ctrlOrCmd: true, key: 'N' }) },
                    { action: 'Toggle Sessions sidebar', key: formatKeyCombo({ ctrlOrCmd: true, key: 'B' }) },
                    { action: 'Toggle Resource Canvas', key: formatKeyCombo({ ctrlOrCmd: true, shift: true, key: 'B' }) },
                    { action: 'Maximize Resource Canvas', key: formatKeyCombo({ ctrlOrCmd: true, shift: true, key: 'F' }) },
                    { action: 'Zoom Interface In', key: formatKeyCombo({ ctrlOrCmd: true, key: '+' }) },
                    { action: 'Zoom Interface Out', key: formatKeyCombo({ ctrlOrCmd: true, key: '-' }) },
                    { action: 'Reset Interface Zoom (100%)', key: formatKeyCombo({ ctrlOrCmd: true, key: '0' }) },
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

            {activeTab === 'security' && (
              <div className="space-y-4">
                <div>
                  <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">Security & Authority</h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">
                    Kernel invariants, sandbox assurance and API client key management.
                  </p>
                </div>
                <div className="space-y-3">
                  <SettingRow title="Effect permits" description="Permit enforcement is a kernel invariant and cannot be disabled from presentation settings.">
                    <Shield className="h-4 w-4 text-emerald-500" />
                  </SettingRow>
                  <SettingRow title="Sandbox assurance" description="Assurance is reported per action from the actual execution path.">
                    <span className="text-[10px] text-[var(--color-fg-subtle)] font-mono">Per receipt</span>
                  </SettingRow>

                  {/* Local Gateway Keys */}
                  <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 space-y-3">
                    <div className="flex items-center justify-between">
                      <div>
                        <div className="text-xs font-semibold text-[var(--color-editor-fg)]">Local API Gateway Keys</div>
                        <div className="text-[11px] text-[var(--color-fg-muted)]">
                          Used by authorized local SDK and CLI integrations to talk to daemon.
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => void handleGenerateClientKey()}
                        className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1.5 text-[11px] font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] transition cursor-pointer"
                      >
                        Generate key
                      </button>
                    </div>
                    {clientKeys.length > 0 && (
                      <div className="space-y-2 pt-1">
                        {clientKeys.map((key) => (
                          <div key={key.id} className="flex items-center justify-between rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-canvas)] p-3">
                            <div>
                              <div className="text-xs font-medium text-[var(--color-editor-fg)]">{key.name}</div>
                              <div className="font-mono text-[10px] text-[var(--color-fg-subtle)]">{key.token}</div>
                            </div>
                            <button
                              type="button"
                              onClick={() => void handleRevokeClientKey(key.id)}
                              className="text-[10px] font-medium text-rose-400 hover:text-rose-300 transition cursor-pointer"
                            >
                              Revoke
                            </button>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}

            {activeTab === 'storage' && (
              <div className="space-y-4">
                <div>
                  <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">Storage & Artifacts</h2>
                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">Durable storage locations and retention model.</p>
                </div>
                <div className="space-y-3">
                  <SettingRow title="Canonical state" description="Tasks, authority, effects and Research metadata are owned by daemon persistence.">
                    <Server className="h-4 w-4 text-[var(--color-fg-muted)]" />
                  </SettingRow>
                  <SettingRow title="Artifacts" description="Content-addressed artifacts and retention require daemon APIs; this client does not delete or invent them.">
                    <Database className="h-4 w-4 text-[var(--color-fg-muted)]" />
                  </SettingRow>
                </div>
              </div>
            )}

            {activeTab === 'about' && (
              <div className="max-w-lg space-y-3">
                <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">Custos Sovereign Studio</h2>
                <p className="text-xs leading-6 text-[var(--color-fg-muted)]">
                  A local-first supervised agent development environment with Copilot, Coding and
                  Research workbenches over one Task, authority and evidence model.
                </p>
                <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 text-[11px] font-mono text-[var(--color-fg-subtle)] space-y-1">
                  <div>Client: Custos Desktop</div>
                  <div>Runtime: daemon connection and configured executors</div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
