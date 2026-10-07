import React, { useState } from 'react';
import { 
  SlidersHorizontal, 
  X, 
  Monitor, 
  Cpu, 
  GitFork, 
  Shield, 
  Keyboard, 
  Database, 
  Info, 
  Save, 
  RotateCcw, 
  Check, 
  Folder, 
  Sparkles,
  ExternalLink,
  RefreshCw,
  Bell,
  KeyRound,
  Plus,
  Activity,
  Copy
} from 'lucide-react';
import { ClaudeIcon, GeminiIcon, OpenAIIcon, DeepSeekIcon } from '../common/AgentIcons';
import { useAppContext } from '../../context/AppContext';
import { CustomSelect } from '../CustomSelect';
import { Tooltip } from '../Tooltip';

export interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialTab?: string;
  onOpenProviders?: () => void;
  onShowToast: (msg: string) => void;
}

type SettingsTab = 
  | 'general' 
  | 'appearance' 
  | 'providers'
  | 'ai' 
  | 'omniroute' 
  | 'security' 
  | 'telemetry'
  | 'storage'
  | 'shortcuts' 
  | 'about';

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  onClose,
  initialTab,
  onOpenProviders: _onOpenProviders,
  onShowToast
}) => {
  const { 
    uiScale, 
    handleSetUiScale, 
    providers, 
    clientKeys, 
    setIsAddProviderOpen,
    handleGenerateClientKey 
  } = useAppContext();

  // Tab State
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');

  React.useEffect(() => {
    if (isOpen && initialTab) {
      setActiveTab(initialTab as SettingsTab);
    }
  }, [isOpen, initialTab]);

  // General Settings State
  const [launchAtStartup, setLaunchAtStartup] = useState(false);
  const [reopenLastProject, setReopenLastProject] = useState(true);
  const [confirmExit, setConfirmExit] = useState(true);
  const [autoSave, setAutoSave] = useState(true);
  const [autoSaveInterval, setAutoSaveInterval] = useState('1m');
  const [defaultWorkspace, setDefaultWorkspace] = useState('C:\\Users\\Admin\\Desktop\\Custos');
  const [language, setLanguage] = useState('en');
  const [notifications, setNotifications] = useState(true);

  // Appearance State
  const [theme, setTheme] = useState<'dark' | 'midnight' | 'slate' | 'light'>('dark');
  const [accentColor, setAccentColor] = useState<'blue' | 'cyan' | 'emerald' | 'purple'>('blue');
  const [fontFamily, setFontFamily] = useState('JetBrains Mono');
  const [uiDensity, setUiDensity] = useState<'comfortable' | 'compact'>('comfortable');

  // AI & Models State
  const [defaultModel, setDefaultModel] = useState('claude-3-7-sonnet');
  const [temperature, setTemperature] = useState(0.2);
  const [maxTokens, setMaxTokens] = useState('8192');
  const [systemPrompt, setSystemPrompt] = useState(
    'You are Custos, an elite autonomous software engineering assistant. Write concise, idiomatic code adhering to project architectural boundaries.'
  );
  const [streamResponse, setStreamResponse] = useState(true);

  // OmniRoute State
  const [latencyThreshold, setLatencyThreshold] = useState(150);
  const [consecutiveFailures, setConsecutiveFailures] = useState(3);
  const [offlineLocalMode, setOfflineLocalMode] = useState(true);
  const [localOllamaUrl, setLocalOllamaUrl] = useState('http://localhost:11434');

  // Security State
  const [enforcePermits, setEnforcePermits] = useState(true);
  const [confirmCommands, setConfirmCommands] = useState(true);
  const [encryptedStorage, setEncryptedStorage] = useState(true);
  const [telemetry, setTelemetry] = useState(false);

  if (!isOpen) return null;

  const handleSavePreferences = () => {
    onShowToast('Preferences saved successfully');
    onClose();
  };

  const handleResetDefaults = () => {
    handleSetUiScale(100);
    setTheme('dark');
    setAccentColor('blue');
    setFontFamily('JetBrains Mono');
    setUiDensity('comfortable');
    setDefaultModel('claude-3-7-sonnet');
    setTemperature(0.2);
    setMaxTokens('8192');
    setLaunchAtStartup(false);
    setReopenLastProject(true);
    setConfirmExit(true);
    setAutoSave(true);
    setEnforcePermits(true);
    setConfirmCommands(true);
    setOfflineLocalMode(true);
    setLatencyThreshold(150);
    onShowToast('All settings reset to defaults');
  };

  const navTabs: { id: SettingsTab; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
    { id: 'general', label: 'General', icon: SlidersHorizontal },
    { id: 'appearance', label: 'Appearance', icon: Monitor },
    { id: 'providers', label: 'API Keys & Providers', icon: KeyRound },
    { id: 'ai', label: 'AI & Inference', icon: Cpu },
    { id: 'omniroute', label: 'OmniRoute & Network', icon: GitFork },
    { id: 'security', label: 'Security & Sandbox', icon: Shield },
    { id: 'telemetry', label: 'Telemetry & Logs', icon: Activity },
    { id: 'storage', label: 'Storage & KV Cache', icon: Database },
    { id: 'shortcuts', label: 'Shortcuts', icon: Keyboard },
    { id: 'about', label: 'About Custos', icon: Info },
  ];

  return (
    <div className="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-4xl h-[640px] max-h-[94vh] bg-[#161b22] border border-[#30363d] rounded-2xl shadow-2xl flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="h-13 border-b border-[#21262d] px-5 flex items-center justify-between shrink-0 bg-[#0d1117]">
          <div className="flex items-center gap-2.5">
            <div className="w-7 h-7 rounded-lg bg-[#21262d] border border-[#30363d] flex items-center justify-center text-[#58a6ff]">
              <SlidersHorizontal className="w-4 h-4" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-white">App Settings</h2>
              <p className="text-[11px] text-[#8b949e]">Configure application behavior, interface appearance, and AI runtime defaults</p>
            </div>
          </div>
          <Tooltip content="Close Settings (Esc)" position="bottom">
            <button 
              onClick={onClose} 
              className="p-1.5 hover:bg-[#21262d] rounded-lg text-[#8b949e] hover:text-white transition"
            >
              <X className="w-4 h-4" />
            </button>
          </Tooltip>
        </div>

        {/* Body */}
        <div className="flex-1 flex flex-col md:flex-row overflow-hidden">
          {/* Tab Sidebar */}
          <aside className="w-full md:w-56 border-b md:border-b-0 md:border-r border-[#21262d] p-2 md:p-3 flex md:flex-col gap-1 bg-[#0d1117] shrink-0 text-xs overflow-x-auto">
            {navTabs.map((tab) => {
              const Icon = tab.icon;
              const isActive = activeTab === tab.id;
              return (
                <button 
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`text-left px-3 py-2 rounded-lg font-medium flex items-center gap-2.5 shrink-0 transition ${
                    isActive
                      ? 'bg-[#21262d] text-white border border-[#30363d] shadow-sm font-medium'
                      : 'text-[#8b949e] hover:text-white hover:bg-[#161b22] border border-transparent'
                  }`}
                >
                  <Icon className={`w-4 h-4 ${isActive ? 'text-white' : 'text-[#8b949e]'}`} />
                  <span>{tab.label}</span>
                </button>
              );
            })}
          </aside>

          {/* Content Area */}
          <div className="flex-1 overflow-y-auto p-4 sm:p-6 space-y-6 text-xs text-[#c9d1d9] select-text bg-[#0d1117]/60">
            
            {/* 1. GENERAL TAB */}
            {activeTab === 'general' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">General Settings</h3>
                  <p className="text-[#8b949e] text-[11px]">System startup, window lifecycle, and workspace defaults</p>
                </div>

                {/* Workspace Directory */}
                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="flex items-center gap-2 text-white font-medium">
                    <Folder className="w-4 h-4 text-[#58a6ff]" />
                    <span>Default Workspace Directory</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <input 
                      type="text"
                      value={defaultWorkspace}
                      onChange={(e) => setDefaultWorkspace(e.target.value)}
                      className="flex-1 bg-[#0d1117] border border-[#21262d] rounded-lg px-3 py-2 text-xs text-white font-mono focus:outline-none focus:border-[#58a6ff]"
                    />
                    <button 
                      onClick={() => onShowToast('Workspace folder path selected')}
                      className="px-3 py-2 bg-[#0d1117] hover:bg-[#21262d] border border-[#21262d] rounded-lg text-xs text-[#c9d1d9] font-medium transition"
                    >
                      Browse
                    </button>
                  </div>
                  <p className="text-[11px] text-[#6e7681]">
                    Initial project path loaded upon startup when no session path is specified.
                  </p>
                </div>

                {/* Startup & Window */}
                <div className="space-y-2">
                  <h4 className="text-xs font-semibold text-white uppercase tracking-wider text-[#8b949e]">Application Lifecycle</h4>
                  
                  <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition select-none">
                    <div>
                      <div className="font-medium text-white">Launch Custos at system login</div>
                      <div className="text-[11px] text-[#6e7681]">Automatically run Custos background daemon and tray app upon computer startup</div>
                    </div>
                    <input 
                      type="checkbox"
                      checked={launchAtStartup}
                      onChange={(e) => setLaunchAtStartup(e.target.checked)}
                    />
                  </label>

                  <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition select-none">
                    <div>
                      <div className="font-medium text-white">Reopen last active project automatically</div>
                      <div className="text-[11px] text-[#6e7681]">Restore recent sessions, tabs, and layout when opening the application</div>
                    </div>
                    <input 
                      type="checkbox"
                      checked={reopenLastProject}
                      onChange={(e) => setReopenLastProject(e.target.checked)}
                    />
                  </label>

                  <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition select-none">
                    <div>
                      <div className="font-medium text-white">Confirm before closing active sessions</div>
                      <div className="text-[11px] text-[#6e7681]">Prompt a confirmation dialog when unapplied diffs or background runs exist</div>
                    </div>
                    <input 
                      type="checkbox"
                      checked={confirmExit}
                      onChange={(e) => setConfirmExit(e.target.checked)}
                    />
                  </label>
                </div>

                {/* Auto-Save & Language */}
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div className="p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] space-y-2">
                    <div className="flex items-center justify-between">
                      <span className="font-medium text-white">Auto-Save Sessions</span>
                      <input 
                        type="checkbox"
                        checked={autoSave}
                        onChange={(e) => setAutoSave(e.target.checked)}
                      />
                    </div>
                    <div className="text-[11px] text-[#6e7681]">Frequency:</div>
                    <CustomSelect 
                      value={autoSaveInterval}
                      onChange={setAutoSaveInterval}
                      options={[
                        { value: 'immediate', label: 'On every change' },
                        { value: '1m', label: 'Every 1 minute' },
                        { value: '5m', label: 'Every 5 minutes' },
                      ]}
                      headerTitle="Auto-Save Frequency"
                    />
                  </div>

                  <div className="p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] space-y-2">
                    <span className="font-medium text-white block">Interface Language</span>
                    <div className="text-[11px] text-[#6e7681]">Application display language:</div>
                    <CustomSelect 
                      value={language}
                      onChange={setLanguage}
                      options={[
                        { value: 'en', label: 'English (US)' },
                        { value: 'vi', label: 'Tiếng Việt (Vietnamese)' },
                      ]}
                      headerTitle="Select Language"
                    />
                  </div>
                </div>

                {/* Notifications */}
                <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition select-none">
                  <div className="flex items-center gap-2.5">
                    <Bell className="w-4 h-4 text-[#58a6ff]" />
                    <div>
                      <div className="font-medium text-white">Desktop System Notifications</div>
                      <div className="text-[11px] text-[#6e7681]">Notify when autonomous tasks, agent execution, or long commands finish</div>
                    </div>
                  </div>
                  <input 
                    type="checkbox"
                    checked={notifications}
                    onChange={(e) => setNotifications(e.target.checked)}
                  />
                </label>
              </div>
            )}

            {/* 2. APPEARANCE TAB */}
            {activeTab === 'appearance' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">Appearance & Theming</h3>
                  <p className="text-[#8b949e] text-[11px]">Customize themes, accent colors, typography, and display scale</p>
                </div>

                {/* Theme Selector */}
                <div className="space-y-2">
                  <h4 className="text-xs font-semibold text-[#8b949e] uppercase tracking-wider">Color Theme</h4>
                  <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                    {[
                      { id: 'dark', label: 'Dark Canvas', bg: '#131419', border: '#2e303d' },
                      { id: 'midnight', label: 'Midnight Blue', bg: '#0b1120', border: '#1e293b' },
                      { id: 'slate', label: 'Slate Dark', bg: '#18181b', border: '#27272a' },
                      { id: 'light', label: 'Light (Preview)', bg: '#f8fafc', border: '#e2e8f0', lightText: true },
                    ].map((t) => (
                      <button
                        key={t.id}
                        onClick={() => setTheme(t.id as any)}
                        className={`p-3 rounded-xl border text-left transition flex flex-col justify-between h-20 ${
                          theme === t.id
                            ? 'border-[#58a6ff] ring-1 ring-brand-blue bg-[#161b22]'
                            : 'border-[#21262d] hover:border-[#384252] bg-[#161b22]'
                        }`}
                      >
                        <div className="flex items-center justify-between">
                          <span 
                            className="w-5 h-5 rounded-full border border-white/20 shadow-sm"
                            style={{ backgroundColor: t.bg }}
                          />
                          {theme === t.id && <Check className="w-3.5 h-3.5 text-[#58a6ff]" />}
                        </div>
                        <span className="text-xs font-medium text-white">{t.label}</span>
                      </button>
                    ))}
                  </div>
                </div>

                {/* Accent Color */}
                <div className="space-y-2">
                  <h4 className="text-xs font-semibold text-[#8b949e] uppercase tracking-wider">Accent Color</h4>
                  <div className="flex items-center gap-3">
                    {[
                      { id: 'blue', label: 'Custos Blue', color: '#0070f3' },
                      { id: 'cyan', label: 'Cyber Cyan', color: '#00d4ff' },
                      { id: 'emerald', label: 'Emerald', color: '#10b981' },
                      { id: 'purple', label: 'Violet Glow', color: '#a855f7' },
                    ].map((c) => (
                      <button
                        key={c.id}
                        onClick={() => setAccentColor(c.id as any)}
                        className={`flex items-center gap-2 px-3 py-1.5 rounded-lg border transition ${
                          accentColor === c.id
                            ? 'border-white text-white bg-[#0d1117]'
                            : 'border-[#21262d] text-[#8b949e] hover:text-white bg-[#161b22]'
                        }`}
                      >
                        <span className="w-3 h-3 rounded-full" style={{ backgroundColor: c.color }} />
                        <span className="text-xs">{c.label}</span>
                      </button>
                    ))}
                  </div>
                </div>

                {/* UI Scale Slider */}
                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="font-medium text-white block">Interface Zoom & Scale</span>
                      <span className="text-[11px] text-[#6e7681]">Scale the desktop interface smoothly across high-DPI displays</span>
                    </div>
                    <span className="font-mono text-[#58a6ff] bg-[#21262d] px-2 py-0.5 rounded border border-[#30363d] text-xs font-semibold">
                      {uiScale}%
                    </span>
                  </div>

                  <input
                    type="range"
                    min="75"
                    max="150"
                    step="5"
                    value={uiScale}
                    onChange={(e) => handleSetUiScale(parseInt(e.target.value, 10))}
                    className="w-full h-1.5 bg-[#0d1117] rounded-lg appearance-none cursor-pointer accent-[#58a6ff]"
                  />

                  <div className="flex items-center gap-2 pt-1">
                    {[85, 90, 100, 110, 125].map((preset) => (
                      <button
                        key={preset}
                        onClick={() => handleSetUiScale(preset)}
                        className={`px-2.5 py-1 rounded text-[11px] font-mono transition ${
                          uiScale === preset
                            ? 'bg-[#21262d] text-white border border-[#30363d] font-medium'
                            : 'bg-[#0d1117] hover:bg-[#21262d] text-[#8b949e] hover:text-white border border-[#21262d]'
                        }`}
                      >
                        {preset}%
                      </button>
                    ))}
                  </div>
                </div>

                {/* Font & Density */}
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div className="p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] space-y-2">
                    <span className="font-medium text-white block">Editor Monospace Font</span>
                    <CustomSelect
                      value={fontFamily}
                      onChange={setFontFamily}
                      options={[
                        { value: 'JetBrains Mono', label: 'JetBrains Mono' },
                        { value: 'Fira Code', label: 'Fira Code' },
                        { value: 'Cascadia Code', label: 'Cascadia Code' },
                        { value: 'Consolas', label: 'Consolas / Courier' },
                      ]}
                      headerTitle="Editor Monospace Font"
                    />
                  </div>

                  <div className="p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] space-y-2">
                    <span className="font-medium text-white block">Layout Density</span>
                    <CustomSelect
                      value={uiDensity}
                      onChange={(val: string | number) => setUiDensity(val as any)}
                      options={[
                        { value: 'comfortable', label: 'Comfortable (Standard)' },
                        { value: 'compact', label: 'Compact (High Information Density)' },
                      ]}
                      headerTitle="Layout Density Preset"
                    />
                  </div>
                </div>
              </div>
            )}

            {/* 2.5 API KEYS & MODEL PROVIDERS TAB */}
            {activeTab === 'providers' && (
              <div className="space-y-6">
                <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                  <div>
                    <h3 className="text-sm font-semibold text-white mb-1">API Keys & Model Providers</h3>
                    <p className="text-[#8b949e] text-[11px]">Manage upstream model credentials, API secrets, and OmniRoute priorities</p>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <button
                      onClick={() => onShowToast('Health check: Anthropic 340ms, Gemini 140ms, DeepSeek 480ms - All healthy')}
                      className="px-2.5 py-1.5 rounded-lg bg-[#0d1117] hover:bg-[#21262d] border border-[#21262d] text-[#c9d1d9] text-[11px] font-medium transition flex items-center gap-1.5"
                    >
                      <Activity className="w-3.5 h-3.5 text-emerald-400" />
                      <span>Health Check</span>
                    </button>
                    <button
                      onClick={() => setIsAddProviderOpen(true)}
                      className="px-3 py-1.5 rounded-lg bg-white text-black hover:bg-neutral-200 text-[11px] font-semibold transition flex items-center gap-1.5 shadow-sm"
                    >
                      <Plus className="w-3.5 h-3.5" />
                      <span>Add Provider Key</span>
                    </button>
                  </div>
                </div>

                {/* Connected Providers List */}
                <div className="space-y-3">
                  {providers.map((p) => {
                    const isPrimary = p.status === 'primary';
                    const isStandby = p.status === 'standby';
                    return (
                      <div
                        key={p.id}
                        className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] hover:border-[#384252] transition space-y-3"
                      >
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-3">
                            <div className="w-7 h-7 rounded-lg bg-[#0d1117] border border-[#21262d] flex items-center justify-center p-1 shrink-0">
                              {p.iconType === 'anthropic' && <ClaudeIcon size={16} />}
                              {p.iconType === 'gemini' && <GeminiIcon size={16} />}
                              {p.iconType === 'deepseek' && <DeepSeekIcon size={16} />}
                              {p.iconType === 'openai' && <OpenAIIcon size={16} />}
                              {!['anthropic', 'gemini', 'deepseek', 'openai'].includes(p.iconType) && <Cpu className="w-4 h-4 text-purple-400" />}
                            </div>
                            <div>
                              <div className="flex items-center gap-2">
                                <span className="font-semibold text-white text-xs">{p.name}</span>
                                <span className={`text-[10px] px-2 py-0.2 rounded-full font-mono ${
                                  isPrimary ? 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30' :
                                  isStandby ? 'bg-blue-500/15 text-blue-400 border border-blue-500/30' :
                                  'bg-purple-500/15 text-purple-400 border border-purple-500/30'
                                }`}>
                                  {p.statusLabel}
                                </span>
                              </div>
                              <span className="text-[11px] text-[#8b949e] font-mono">{p.model}</span>
                            </div>
                          </div>
                          <div className="flex items-center gap-2">
                            <span className="font-mono text-[10px] text-[#8b949e]">{p.latency}</span>
                            <span className="w-2 h-2 rounded-full bg-emerald-400" />
                          </div>
                        </div>

                        <div className="flex items-center justify-between pt-2 border-t border-[#21262d]/50 text-[11px]">
                          <div className="flex items-center gap-2">
                            <span className="text-[#6e7681]">Key:</span>
                            <span className="font-mono text-[#c9d1d9] bg-[#0d1117] px-2 py-0.5 rounded border border-[#21262d]/60">{p.apiKey}</span>
                          </div>
                          <div className="flex items-center gap-3 text-[#8b949e] font-mono text-[10.5px]">
                            <span>Quota: {p.quotaUsed} / {p.quotaTotal}</span>
                            <span>Limit: {p.rateLimit}</span>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>

                {/* Local Ollama Gateway Card */}
                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2.5">
                      <div className="w-7 h-7 rounded-lg bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shrink-0">
                        <Cpu className="w-4 h-4" />
                      </div>
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="font-semibold text-white text-xs">Local Ollama Runtime</span>
                          <span className="text-[10px] px-2 py-0.2 rounded-full font-mono bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">Offline Local</span>
                        </div>
                        <span className="text-[11px] text-[#8b949e] font-mono">http://localhost:11434 • DeepSeek R1, Llama 3.3</span>
                      </div>
                    </div>
                    <span className="w-2 h-2 rounded-full bg-emerald-400" />
                  </div>
                  <div className="text-[11px] text-[#8b949e] bg-[#0d1117] p-2.5 rounded-lg border border-[#21262d]/60">
                    Zero-IO local fallback is active. When external internet connectivity or cloud rate limits are triggered, inference automatically shifts to Ollama.
                  </div>
                </div>

                {/* Client Gateway Keys */}
                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="font-semibold text-white block text-xs">Local Client Gateway Keys</span>
                      <span className="text-[11px] text-[#6e7681]">API tokens used by local CLI, IDE extensions & automated scripts</span>
                    </div>
                    <button
                      onClick={handleGenerateClientKey}
                      className="px-2.5 py-1 rounded-lg bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] text-white text-[11px] font-medium flex items-center gap-1 transition"
                    >
                      <Plus className="w-3 h-3" />
                      <span>New Gateway Key</span>
                    </button>
                  </div>
                  {clientKeys.map((k) => (
                    <div key={k.id} className="flex items-center justify-between p-2.5 rounded-lg bg-[#0d1117] border border-[#21262d] text-xs font-mono">
                      <div>
                        <span className="text-white block font-sans font-medium text-[11.5px]">{k.name}</span>
                        <span className="text-[#8b949e] text-[10.5px]">{k.token}</span>
                      </div>
                      <button
                        onClick={() => {
                          navigator.clipboard.writeText(k.token);
                          onShowToast('Gateway token copied to clipboard');
                        }}
                        className="px-2 py-1 rounded bg-[#0d1117] hover:bg-[#21262d] text-[#c9d1d9] hover:text-white transition text-[11px] flex items-center gap-1"
                      >
                        <Copy className="w-3 h-3" />
                        <span>Copy</span>
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* 3. AI & INFERENCE TAB */}
            {activeTab === 'ai' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">AI Reasoning & Inference Defaults</h3>
                  <p className="text-[#8b949e] text-[11px]">Control autonomous LLM behavior, prompt temperature, and code synthesis defaults</p>
                </div>

                {/* Default Primary Model */}
                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="font-medium text-white block">Default Agent Model</span>
                      <span className="text-[11px] text-[#6e7681]">Primary model assigned when creating new sessions</span>
                    </div>
                    <Sparkles className="w-4 h-4 text-purple-400" />
                  </div>
                  <CustomSelect
                    value={defaultModel}
                    onChange={setDefaultModel}
                    options={[
                      { 
                        value: 'claude-3-7-sonnet', 
                        label: 'Claude 3.7 Sonnet (Anthropic - Hybrid Reasoning)',
                        icon: <img src="/assets/provider-logo/anthropic.jpg" alt="Anthropic" className="w-3.5 h-3.5 rounded object-cover" /> 
                      },
                      { 
                        value: 'gpt-4o', 
                        label: 'OpenAI GPT-4o (High-speed code synthesis)',
                        icon: <img src="/assets/provider-logo/openai.jpg" alt="OpenAI" className="w-3.5 h-3.5 rounded object-cover" /> 
                      },
                      { 
                        value: 'deepseek-r1', 
                        label: 'DeepSeek R1 (Ollama / Local inference)',
                        icon: <img src="/assets/provider-logo/deepseek.jpg" alt="DeepSeek" className="w-3.5 h-3.5 rounded object-cover" /> 
                      },
                      { 
                        value: 'gemini-2.5-pro', 
                        label: 'Gemini 2.5 Pro (Google Cloud - 1M token context)',
                        icon: <img src="/assets/provider-logo/gemini.jpg" alt="Google" className="w-3.5 h-3.5 rounded object-cover" /> 
                      },
                    ]}
                    headerTitle="Primary Inference Model"
                    headerBadge="4 Models"
                  />
                </div>

                {/* Temperature & Token Limits */}
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                    <div className="flex items-center justify-between">
                      <span className="font-medium text-white">Reasoning Temperature</span>
                      <span className="font-mono text-purple-400 bg-purple-500/10 px-2 py-0.5 rounded border border-purple-500/20 text-xs">
                        {temperature}
                      </span>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.05"
                      value={temperature}
                      onChange={(e) => setTemperature(parseFloat(e.target.value))}
                      className="w-full h-1.5 bg-[#0d1117] rounded-lg appearance-none cursor-pointer accent-purple-500"
                    />
                    <div className="flex justify-between text-[10px] text-[#6e7681] font-mono">
                      <span>0.0 (Strict / Code)</span>
                      <span>1.0 (Creative)</span>
                    </div>
                  </div>

                  <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                    <span className="font-medium text-white block">Max Output Tokens</span>
                    <CustomSelect
                      value={maxTokens}
                      onChange={setMaxTokens}
                      options={[
                        { value: '4096', label: '4,096 tokens (Standard)' },
                        { value: '8192', label: '8,192 tokens (Extended diffs)' },
                        { value: '16384', label: '16,384 tokens (Deep refactor)' },
                      ]}
                      headerTitle="Max Token Budget"
                    />
                    <p className="text-[11px] text-[#6e7681]">Max tokens returned per code completion step.</p>
                  </div>
                </div>

                {/* System Prompt Instructions */}
                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-2">
                  <span className="font-medium text-white block">Global Developer Instructions</span>
                  <textarea
                    rows={3}
                    value={systemPrompt}
                    onChange={(e) => setSystemPrompt(e.target.value)}
                    className="w-full bg-[#0d1117] border border-[#21262d] rounded-lg p-2.5 text-xs text-white focus:outline-none focus:border-[#58a6ff] resize-none font-mono"
                    placeholder="Custom rules for coding style, test frameworks, or language constraints..."
                  />
                  <p className="text-[11px] text-[#6e7681]">Injected into the system prompt of all coding sessions.</p>
                </div>

                {/* Streaming Response */}
                <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition">
                  <div>
                    <div className="font-medium text-white">Stream Token Response Rendering</div>
                    <div className="text-[11px] text-[#6e7681]">Stream tokens in real-time as the model generates AST patches</div>
                  </div>
                  <input
                    type="checkbox"
                    checked={streamResponse}
                    onChange={(e) => setStreamResponse(e.target.checked)}
                    className="w-4 h-4 rounded bg-[#0d1117] border-[#21262d] accent-[#58a6ff] cursor-pointer"
                  />
                </label>
              </div>
            )}

            {/* 4. OMNIROUTE & NETWORK TAB */}
            {activeTab === 'omniroute' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">OmniRoute Routing Chain & Failover</h3>
                  <p className="text-[#8b949e] text-[11px]">Define priority fallback topology when rate limits (HTTP 429) or latency spikes occur</p>
                </div>

                {/* Routing Chain Topology Table */}
                <div className="border border-[#21262d] rounded-xl overflow-x-auto bg-[#161b22]">
                  <div className="min-w-[420px]">
                    <div className="grid grid-cols-12 px-3 py-2 border-b border-[#21262d] text-[11px] font-mono text-[#6e7681] bg-[#0d1117]">
                      <span className="col-span-1">PRI</span>
                      <span className="col-span-4">ALIAS</span>
                      <span className="col-span-5">TARGET MODEL</span>
                      <span className="col-span-2 text-right">STATUS</span>
                    </div>
                    <div className="grid grid-cols-12 px-3 py-2.5 items-center border-b border-[#21262d]/50 font-mono text-[11px]">
                      <span className="col-span-1 text-[#58a6ff] font-bold">1</span>
                      <span className="col-span-4 text-white">default-agent</span>
                      <span className="col-span-5 text-[#c9d1d9]">claude-3-7-sonnet</span>
                      <div className="col-span-2 text-right">
                        <span className="text-emerald-400 bg-emerald-500/10 px-1.5 py-0.5 rounded text-[10px]">Active</span>
                      </div>
                    </div>
                    <div className="grid grid-cols-12 px-3 py-2.5 items-center font-mono text-[11px]">
                      <span className="col-span-1 text-[#6e7681] font-bold">2</span>
                      <span className="col-span-4 text-[#8b949e]">fallback-rate-limit</span>
                      <span className="col-span-5 text-[#c9d1d9]">gpt-4o</span>
                      <div className="col-span-2 text-right">
                        <span className="text-[#8b949e] bg-[#0d1117] px-1.5 py-0.5 rounded text-[10px]">Standby</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Latency Thresholds */}
                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="font-medium text-white">Auto Failover Latency Trigger</div>
                      <div className="text-[11px] text-[#6e7681]">Route to secondary standby provider if primary latency exceeds target</div>
                    </div>
                    <span className="text-[#c9d1d9] font-mono bg-[#0d1117] px-2 py-1 rounded border border-[#21262d] text-xs">
                      {latencyThreshold}ms
                    </span>
                  </div>
                  <input
                    type="range"
                    min="50"
                    max="500"
                    step="10"
                    value={latencyThreshold}
                    onChange={(e) => setLatencyThreshold(parseInt(e.target.value, 10))}
                    className="w-full h-1.5 bg-[#0d1117] rounded-lg appearance-none cursor-pointer accent-[#58a6ff]"
                  />
                </div>

                {/* Circuit Breaker & Offline Mode */}
                <div className="space-y-3">
                  <div className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d]">
                    <div>
                      <div className="font-medium text-white">Circuit Breaker Trip Condition</div>
                      <div className="text-[11px] text-[#6e7681]">Consecutive remote 5xx errors before isolating route</div>
                    </div>
                    <div className="w-36 shrink-0">
                      <CustomSelect
                        value={consecutiveFailures}
                        onChange={(val: string | number) => setConsecutiveFailures(Number(val))}
                        options={[
                          { value: 2, label: '2 strikes' },
                          { value: 3, label: '3 strikes' },
                          { value: 5, label: '5 strikes' },
                        ]}
                        headerTitle="Trip Condition"
                      />
                    </div>
                  </div>

                  <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition select-none">
                    <div>
                      <div className="font-medium text-white">Offline Local Mode (Ollama fallback)</div>
                      <div className="text-[11px] text-[#6e7681]">Automatically switch inference to local Ollama when internet connection drops</div>
                    </div>
                    <input
                      type="checkbox"
                      checked={offlineLocalMode}
                      onChange={(e) => setOfflineLocalMode(e.target.checked)}
                    />
                  </label>

                  {offlineLocalMode && (
                    <div className="p-3.5 rounded-xl bg-[#0d1117] border border-[#21262d] space-y-1.5">
                      <span className="text-[#8b949e] font-medium">Local Ollama Endpoint</span>
                      <input
                        type="text"
                        value={localOllamaUrl}
                        onChange={(e) => setLocalOllamaUrl(e.target.value)}
                        className="w-full bg-[#161b22] border border-[#21262d] rounded-lg px-3 py-1.5 font-mono text-xs text-white focus:outline-none"
                      />
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* 5. SECURITY & SANDBOX TAB */}
            {activeTab === 'security' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">Security & Sandbox Enforcement</h3>
                  <p className="text-[#8b949e] text-[11px]">Guardrails managed by Custos Kernel (custos-security) and AST parser</p>
                </div>

                <div className="space-y-3">
                  <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition select-none">
                    <div>
                      <div className="font-medium text-white">Require ExecutionPermit for File Writes</div>
                      <div className="text-[11px] text-[#6e7681]">Enforces AST diff checks and sandbox validation before modifying any disk files</div>
                    </div>
                    <input
                      type="checkbox"
                      checked={enforcePermits}
                      onChange={(e) => setEnforcePermits(e.target.checked)}
                    />
                  </label>

                  <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition select-none">
                    <div>
                      <div className="font-medium text-white">Require Confirmation for Terminal Commands</div>
                      <div className="text-[11px] text-[#6e7681]">Prompt user for approval before running proposed shell commands (build, test, deploy)</div>
                    </div>
                    <input
                      type="checkbox"
                      checked={confirmCommands}
                      onChange={(e) => setConfirmCommands(e.target.checked)}
                    />
                  </label>

                  <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition select-none">
                    <div>
                      <div className="font-medium text-white">Hardware Encrypted Credential Vault</div>
                      <div className="text-[11px] text-[#6e7681]">Store API tokens encrypted using Windows DPAPI / OS Keychain hardware keys</div>
                    </div>
                    <input
                      type="checkbox"
                      checked={encryptedStorage}
                      onChange={(e) => setEncryptedStorage(e.target.checked)}
                    />
                  </label>

                  <label className="flex items-center justify-between p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] cursor-pointer hover:border-[#384252] transition select-none">
                    <div>
                      <div className="font-medium text-white">Anonymous Error & Crash Reporting</div>
                      <div className="text-[11px] text-[#6e7681]">Send anonymized crash diagnostics to help improve Custos desktop stability</div>
                    </div>
                    <input
                      type="checkbox"
                      checked={telemetry}
                      onChange={(e) => setTelemetry(e.target.checked)}
                    />
                  </label>
                </div>
              </div>
            )}

            {/* TELEMETRY & LOGS TAB */}
            {activeTab === 'telemetry' && (
              <div className="space-y-6">
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-sm font-semibold text-white mb-1">Kernel Telemetry & Daemon Spans</h3>
                    <p className="text-[#8b949e] text-[11px]">Real-time Custos daemon execution traces, local PID status, and AST event pipeline</p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                    <span className="font-mono text-xs text-emerald-400">Daemon PID 4120 • 200 OK</span>
                  </div>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                  <div className="p-3.5 rounded-xl bg-[#161b22] border border-[#21262d]">
                    <span className="text-[11px] text-[#8b949e] block">Uptime</span>
                    <span className="text-sm font-mono font-bold text-white">4h 12m 38s</span>
                  </div>
                  <div className="p-3.5 rounded-xl bg-[#161b22] border border-[#21262d]">
                    <span className="text-[11px] text-[#8b949e] block">Avg Event Latency</span>
                    <span className="text-sm font-mono font-bold text-emerald-400">14.2 ms</span>
                  </div>
                  <div className="p-3.5 rounded-xl bg-[#161b22] border border-[#21262d]">
                    <span className="text-[11px] text-[#8b949e] block">AST Mutations Checked</span>
                    <span className="text-sm font-mono font-bold text-[#58a6ff]">1,428 passes</span>
                  </div>
                </div>

                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="flex items-center justify-between text-xs">
                    <span className="font-semibold text-white">Live Execution Spans</span>
                    <button
                      onClick={() => onShowToast('Telemetry spans flushed')}
                      className="text-[#58a6ff] hover:underline text-[11px]"
                    >
                      Flush Spans
                    </button>
                  </div>
                  <div className="bg-[#0d1117] rounded-lg p-3 font-mono text-[11px] text-[#8b949e] space-y-2 border border-[#21262d]">
                    <div className="flex items-center justify-between text-neutral-300">
                      <span>[INFO] daemon::runtime: initialized local IPC socket</span>
                      <span className="text-[#6e7681]">0.8ms</span>
                    </div>
                    <div className="flex items-center justify-between text-emerald-400">
                      <span>[PASS] custos-security: verified kernel capability lease</span>
                      <span className="text-[#6e7681]">1.2ms</span>
                    </div>
                    <div className="flex items-center justify-between text-neutral-300">
                      <span>[ROUT] omniroute: healthy ping to Claude 3.7 Sonnet</span>
                      <span className="text-[#6e7681]">340ms</span>
                    </div>
                    <div className="flex items-center justify-between text-neutral-400">
                      <span>[SYNC] sqlite-kv: committed AST diff snapshot #a3f2d1e</span>
                      <span className="text-[#6e7681]">2.4ms</span>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* 6. KEYBOARD SHORTCUTS TAB */}
            {activeTab === 'shortcuts' && (
              <div className="space-y-4">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">Desktop Keyboard Shortcuts</h3>
                  <p className="text-[#8b949e] text-[11px]">Productivity hotkeys for quick session management and layout control</p>
                </div>

                <div className="border border-[#21262d] rounded-xl overflow-hidden bg-[#161b22] divide-y divide-surface-border/60">
                  {[
                    { action: 'Open Preferences / Settings', key: 'Ctrl+,' },
                    { action: 'Toggle Sessions Sidebar', key: 'Ctrl+B' },
                    { action: 'Toggle Split / Chat View', key: 'Ctrl+\\' },
                    { action: 'Create New Session', key: 'Ctrl+N' },
                    { action: 'Accept & Apply Code Diff', key: 'Ctrl+Enter' },
                    { action: 'Clear Current History', key: 'Ctrl+K' },
                    { action: 'Zoom Interface In', key: 'Ctrl++' },
                    { action: 'Zoom Interface Out', key: 'Ctrl+-' },
                    { action: 'Reset Zoom (100%)', key: 'Ctrl+0' },
                    { action: 'Focus Chat Input', key: 'Ctrl+/' },
                  ].map((s) => (
                    <div key={s.action} className="flex items-center justify-between px-4 py-2.5 hover:bg-[#0d1117]/50 transition">
                      <span className="text-[#c9d1d9] font-medium">{s.action}</span>
                      <kbd className="px-2 py-1 rounded bg-[#0d1117] border border-[#21262d] font-mono text-[11px] text-white shadow-sm">
                        {s.key}
                      </kbd>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* 7. STORAGE & CACHE TAB */}
            {activeTab === 'storage' && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">SQLite & Key-Value Cache</h3>
                  <p className="text-[#8b949e] text-[11px]">Local encrypted storage for prompt tokens, embeddings, and AST cache</p>
                </div>

                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="flex justify-between items-center text-xs">
                    <span className="text-[#8b949e]">Total Cache Footprint:</span>
                    <span className="font-mono text-white font-medium">12.4 MB (420 entries)</span>
                  </div>
                  <div className="flex justify-between items-center text-xs">
                    <span className="text-[#8b949e]">SQLite Database Path:</span>
                    <span className="font-mono text-[#8b949e] text-[11px]">~/.custos/store.db</span>
                  </div>
                  <div className="flex justify-between items-center text-xs">
                    <span className="text-[#8b949e]">Session Storage:</span>
                    <span className="font-mono text-[#8b949e] text-[11px]">Active in memory & disk sync</span>
                  </div>
                </div>

                <div className="flex flex-col sm:flex-row gap-3 pt-2">
                  <button 
                    onClick={() => onShowToast('Local cache purged successfully')} 
                    className="px-4 py-2 rounded-lg bg-[#0d1117] hover:bg-[#21262d] text-red-400 text-xs font-medium border border-[#21262d] transition text-center"
                  >
                    Purge Local Cache
                  </button>
                  <button 
                    onClick={handleResetDefaults} 
                    className="px-4 py-2 rounded-lg bg-[#0d1117] hover:bg-[#21262d] text-[#c9d1d9] text-xs font-medium border border-[#21262d] transition flex items-center justify-center gap-1.5"
                  >
                    <RotateCcw className="w-3.5 h-3.5" />
                    <span>Reset All Preferences</span>
                  </button>
                </div>
              </div>
            )}

            {/* 8. ABOUT CUSTOS TAB */}
            {activeTab === 'about' && (
              <div className="space-y-6">
                <div className="flex items-center gap-4 p-5 rounded-2xl bg-[#161b22] border border-[#21262d]">
                  <div className="w-14 h-14 rounded-2xl bg-[#0d1117] border border-[#21262d] flex items-center justify-center p-2 shadow-inner">
                    <img src="/assets/custos-logo.png" alt="Custos" className="w-full h-full object-contain" />
                  </div>
                  <div>
                    <h3 className="text-base font-bold text-white tracking-tight">Custos Desktop</h3>
                    <p className="text-xs text-[#8b949e]">Autonomous Pair-Programming Engine & OmniRoute LLM Gateway</p>
                    <div className="flex items-center gap-2 mt-1.5 font-mono text-[11px] text-[#6e7681]">
                      <span>Version 0.3.1-beta</span>
                      <span>•</span>
                      <span className="text-emerald-400">Up to date</span>
                    </div>
                  </div>
                </div>

                <div className="border border-[#21262d] rounded-xl p-4 bg-[#161b22] space-y-2 text-xs">
                  <div className="flex justify-between">
                    <span className="text-[#8b949e]">Tauri Runtime</span>
                    <span className="font-mono text-[#c9d1d9]">v2.12.1 (Rust 1.85)</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-[#8b949e]">Webview Engine</span>
                    <span className="font-mono text-[#c9d1d9]">Microsoft Edge WebView2 (Chromium)</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-[#8b949e]">Platform</span>
                    <span className="font-mono text-[#c9d1d9]">Windows 11 (x86_64)</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-[#8b949e]">License</span>
                    <span className="font-mono text-[#c9d1d9]">Apache 2.0 / MIT</span>
                  </div>
                </div>

                <div className="flex items-center gap-3">
                  <button
                    onClick={() => onShowToast('Custos is already on the latest version')}
                    className="px-3.5 py-2 bg-brand-blue hover:bg-blue-600 text-white rounded-lg text-xs font-medium transition flex items-center gap-1.5 shadow-md shadow-brand-blue/20"
                  >
                    <RefreshCw className="w-3.5 h-3.5" />
                    <span>Check for Updates</span>
                  </button>
                  <a
                    href="https://github.com/civi0411/Custos"
                    target="_blank"
                    rel="noreferrer"
                    className="px-3.5 py-2 bg-[#0d1117] hover:bg-[#21262d] text-[#c9d1d9] rounded-lg text-xs font-medium border border-[#21262d] transition flex items-center gap-1.5"
                  >
                    <ExternalLink className="w-3.5 h-3.5 text-[#8b949e]" />
                    <span>GitHub Repository</span>
                  </a>
                </div>
              </div>
            )}

          </div>
        </div>

        {/* Footer */}
        <div className="h-13 border-t border-[#21262d] px-5 flex items-center justify-between bg-[#0d1117] shrink-0">
          <button 
            onClick={handleResetDefaults} 
            className="text-xs text-[#6e7681] hover:text-[#c9d1d9] transition flex items-center gap-1"
          >
            <RotateCcw className="w-3 h-3" />
            <span>Restore Defaults</span>
          </button>

          <div className="flex items-center gap-2">
            <button 
              onClick={onClose} 
              className="px-3.5 py-1.5 rounded-lg hover:bg-[#0d1117] text-[#8b949e] hover:text-white transition text-xs font-medium"
            >
              Cancel
            </button>
            <button 
              onClick={handleSavePreferences} 
              className="px-4 py-1.5 rounded-lg bg-[#238636] hover:bg-[#2ea043] text-white font-medium transition text-xs shadow-sm flex items-center gap-1.5"
            >
              <Save className="w-3.5 h-3.5" />
              <span>Save Preferences</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SettingsModal;
