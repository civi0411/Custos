import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { 
  SlidersHorizontal, 
  Monitor, 
  Cpu, 
  GitFork, 
  Shield, 
  Keyboard, 
  Info, 
  Save, 
  RotateCcw, 
  Check, 
  Sparkles,
  RefreshCw,
  Bell,
  KeyRound,
  Plus,
  Activity,
  Copy,
  ArrowLeft
} from 'lucide-react';
import { ClaudeIcon, GeminiIcon, OpenAIIcon, DeepSeekIcon } from '../../components/common/AgentIcons';
import { useAppContext } from '../../context/AppContext';
import { CustomSelect } from '../../components/CustomSelect';
import { getModifierKey, isMacOS } from '@/lib/utils';

type SettingsTab = 
  | 'general' 
  | 'appearance' 
  | 'providers'
  | 'ai' 
  | 'omniroute' 
  | 'security' 
  | 'shortcuts' 
  | 'about';

export const SettingsPage: React.FC = () => {
  const navigate = useNavigate();
  const { 
    uiScale, 
    handleSetUiScale, 
    providers, 
    clientKeys, 
    setIsAddProviderOpen,
    handleGenerateClientKey,
    handleRevokeClientKey,
    showToast 
  } = useAppContext();

  const modKey = getModifierKey();
  const altKey = isMacOS() ? '⌥' : 'Alt+';

  // Tab State
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');

  // General Settings State
  const [defaultWorkspace, setDefaultWorkspace] = useState('C:\\Users\\Admin\\Desktop\\Custos');
  const [launchAtStartup, setLaunchAtStartup] = useState(false);
  const [reopenLastProject, setReopenLastProject] = useState(true);
  const [confirmExit, setConfirmExit] = useState(true);
  const [autoSave, setAutoSave] = useState(true);
  const [notifications, setNotifications] = useState(true);
  const [language, setLanguage] = useState('en');

  // Appearance State
  const [theme, setTheme] = useState<'dark' | 'midnight' | 'slate' | 'light'>('dark');
  const [accentColor, setAccentColor] = useState<'blue' | 'cyan' | 'emerald' | 'purple'>('blue');
  const [fontFamily, setFontFamily] = useState('JetBrains Mono');
  const [uiDensity, setUiDensity] = useState<'comfortable' | 'compact'>('comfortable');

  // AI & Models State
  const [defaultModel, setDefaultModel] = useState('claude-3-7-sonnet');
  const [temperature, setTemperature] = useState(0.2);
  const [maxTokens, setMaxTokens] = useState('8192');
  const [streamResponse, setStreamResponse] = useState(true);
  const [systemPrompt, setSystemPrompt] = useState(
    'You are Custos, an elite autonomous software engineering assistant. Write concise, idiomatic code adhering to project architectural boundaries.'
  );

  // OmniRoute & Security
  const [latencyThreshold, setLatencyThreshold] = useState(150);
  const [offlineLocalMode, setOfflineLocalMode] = useState(true);
  const [localOllamaUrl, setLocalOllamaUrl] = useState('http://localhost:11434');
  const [enforcePermits, setEnforcePermits] = useState(true);
  const [confirmCommands, setConfirmCommands] = useState(true);
  const [encryptedStorage, setEncryptedStorage] = useState(true);

  const handleSave = () => {
    showToast('Preferences saved successfully');
  };

  const handleReset = () => {
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
    setNotifications(true);
    setLanguage('en');
    setStreamResponse(true);
    setEnforcePermits(true);
    setConfirmCommands(true);
    setEncryptedStorage(true);
    setOfflineLocalMode(true);
    setLatencyThreshold(150);
    showToast('Preferences reset to defaults');
  };

  const navTabs: { id: SettingsTab; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
    { id: 'general', label: 'General', icon: SlidersHorizontal },
    { id: 'appearance', label: 'Appearance', icon: Monitor },
    { id: 'providers', label: 'API Keys & Providers', icon: KeyRound },
    { id: 'ai', label: 'AI & Inference', icon: Cpu },
    { id: 'omniroute', label: 'OmniRoute & Network', icon: GitFork },
    { id: 'security', label: 'Security & Sandbox', icon: Shield },
    { id: 'shortcuts', label: 'Shortcuts', icon: Keyboard },
    { id: 'about', label: 'About Custos', icon: Info },
  ];

  return (
    <div className="flex-1 flex flex-col h-full bg-[#0d1117] text-[#c9d1d9] overflow-hidden select-text">
      {/* Top Header */}
      <div className="h-14 border-b border-[#21262d] px-6 flex items-center justify-between shrink-0 bg-[#0d1117]">
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate('/studio')}
            className="p-1.5 rounded-lg bg-[#161b22] hover:bg-[#21262d] border border-[#30363d] text-[#8b949e] hover:text-white transition flex items-center gap-1.5 text-xs"
          >
            <ArrowLeft className="w-3.5 h-3.5" />
            <span>Studio</span>
          </button>
          <div className="h-4 w-[1px] bg-[#21262d]" />
          <div className="flex items-center gap-2.5">
            <div className="w-7 h-7 rounded-lg bg-[#161b22] border border-[#30363d] flex items-center justify-center text-[#58a6ff]">
              <SlidersHorizontal className="w-4 h-4" />
            </div>
            <div>
              <h1 className="text-sm font-semibold text-white tracking-tight">System Settings</h1>
              <p className="text-[11px] text-[#8b949e]">Configure application preferences, AI model providers, OmniRoute fallback, and security</p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleReset}
            className="px-3 py-1.5 bg-[#161b22] hover:bg-[#21262d] border border-[#30363d] rounded-lg text-xs font-medium text-[#c9d1d9] transition flex items-center gap-1.5"
          >
            <RotateCcw className="w-3.5 h-3.5 text-[#8b949e]" />
            <span>Reset Defaults</span>
          </button>
          <button
            onClick={handleSave}
            className="px-3.5 py-1.5 bg-[#238636] hover:bg-[#2ea043] text-white rounded-lg text-xs font-medium transition flex items-center gap-1.5 shadow-sm"
          >
            <Save className="w-3.5 h-3.5" />
            <span>Save Preferences</span>
          </button>
        </div>
      </div>

      {/* Main Layout: Sidebar Tabs + Content Area */}
      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar Tabs */}
        <aside className="w-60 border-r border-[#21262d] p-3 flex flex-col gap-1 bg-[#0d1117] shrink-0 text-xs overflow-y-auto">
          {navTabs.map((tab) => {
            const Icon = tab.icon;
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`text-left px-3 py-2.5 rounded-lg font-medium flex items-center gap-2.5 transition ${
                  isActive
                    ? 'bg-[#21262d] text-white border border-[#30363d] shadow-sm'
                    : 'text-[#8b949e] hover:text-white hover:bg-[#161b22] border border-transparent'
                }`}
              >
                <Icon className={`w-4 h-4 ${isActive ? 'text-[#58a6ff]' : 'text-[#8b949e]'}`} />
                <span>{tab.label}</span>
              </button>
            );
          })}
        </aside>

        {/* Content Pane */}
        <main className="flex-1 overflow-y-auto p-6 lg:p-8 bg-[#0d1117]">
          <div className="max-w-4xl space-y-6">
            
            {/* 1. GENERAL TAB */}
            {activeTab === 'general' && (
              <div className="space-y-5">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">General Preferences</h3>
                  <p className="text-[#8b949e] text-[11px]">Configure startup behavior, workspace root, and background autosave</p>
                </div>

                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-4">
                  <div>
                    <label className="text-xs text-[#8b949e] block mb-1.5">Default Workspace Directory</label>
                    <div className="flex items-center gap-2">
                      <input
                        type="text"
                        value={defaultWorkspace}
                        onChange={(e) => setDefaultWorkspace(e.target.value)}
                        className="flex-1 bg-[#0d1117] border border-[#30363d] rounded-lg px-3 py-2 text-xs text-[#c9d1d9] font-mono focus:outline-none focus:border-[#58a6ff]"
                      />
                      <button
                        onClick={() => showToast('Selected default directory')}
                        className="px-3 py-2 bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] rounded-lg text-xs text-[#c9d1d9] font-medium transition"
                      >
                        Browse
                      </button>
                    </div>
                  </div>

                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
                    <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                      <span className="text-xs text-[#c9d1d9]">Launch at system startup</span>
                      <input
                        type="checkbox"
                        checked={launchAtStartup}
                        onChange={(e) => setLaunchAtStartup(e.target.checked)}
                        className="accent-[#58a6ff]"
                      />
                    </label>
                    <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                      <span className="text-xs text-[#c9d1d9]">Reopen last project on launch</span>
                      <input
                        type="checkbox"
                        checked={reopenLastProject}
                        onChange={(e) => setReopenLastProject(e.target.checked)}
                        className="accent-[#58a6ff]"
                      />
                    </label>
                    <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                      <span className="text-xs text-[#c9d1d9]">Confirm on application close</span>
                      <input
                        type="checkbox"
                        checked={confirmExit}
                        onChange={(e) => setConfirmExit(e.target.checked)}
                        className="accent-[#58a6ff]"
                      />
                    </label>
                    <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                      <span className="text-xs text-[#c9d1d9]">Auto-save session state</span>
                      <input
                        type="checkbox"
                        checked={autoSave}
                        onChange={(e) => setAutoSave(e.target.checked)}
                        className="accent-[#58a6ff]"
                      />
                    </label>
                  </div>

                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
                    <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                      <div className="flex items-center gap-2">
                        <Bell className="w-3.5 h-3.5 text-[#58a6ff]" />
                        <span className="text-xs text-[#c9d1d9]">Desktop Notifications</span>
                      </div>
                      <input
                        type="checkbox"
                        checked={notifications}
                        onChange={(e) => setNotifications(e.target.checked)}
                        className="accent-[#58a6ff]"
                      />
                    </label>

                    <div className="flex items-center justify-between p-2.5 rounded-lg bg-[#0d1117] border border-[#21262d]">
                      <span className="text-xs text-[#c9d1d9]">Interface Language</span>
                      <div className="w-40 shrink-0">
                        <CustomSelect
                          value={language}
                          onChange={setLanguage}
                          options={[
                            { value: 'en', label: 'English (US)' },
                            { value: 'vi', label: 'Tiếng Việt' },
                          ]}
                          headerTitle="Interface Language"
                        />
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* 2. APPEARANCE TAB */}
            {activeTab === 'appearance' && (
              <div className="space-y-5">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">Appearance & Design</h3>
                  <p className="text-[#8b949e] text-[11px]">Customize theme, interface zoom scale, typography and density</p>
                </div>

                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-4">
                  <span className="font-medium text-white block text-xs">Color Theme Presets</span>
                  <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
                    {[
                      { id: 'dark', label: 'Dark Charcoal', color: '#161b22' },
                      { id: 'midnight', label: 'Midnight Blue', color: '#0b1120' },
                      { id: 'slate', label: 'Slate Gray', color: '#18181b' },
                      { id: 'light', label: 'Monochrome Light', color: '#f8fafc' },
                    ].map((t) => (
                      <button
                        key={t.id}
                        onClick={() => setTheme(t.id as any)}
                        className={`p-3 rounded-xl border text-left flex items-center justify-between text-xs transition ${
                          theme === t.id
                            ? 'border-[#58a6ff] bg-[#21262d] text-white shadow-sm'
                            : 'border-[#21262d] text-[#8b949e] hover:text-white bg-[#0d1117] hover:border-[#30363d]'
                        }`}
                      >
                        <span className="flex items-center gap-2">
                          <span className="w-3.5 h-3.5 rounded-full border border-white/20" style={{ backgroundColor: t.color }} />
                          <span>{t.label}</span>
                        </span>
                        {theme === t.id && <Check className="w-3.5 h-3.5 text-[#58a6ff]" />}
                      </button>
                    ))}
                  </div>

                  {/* Accent Color */}
                  <div className="flex items-center gap-2 pt-2 border-t border-[#21262d]">
                    <span className="text-xs text-[#8b949e]">Accent Color:</span>
                    {[
                      { id: 'blue', label: 'Blue', color: '#58a6ff' },
                      { id: 'cyan', label: 'Cyan', color: '#38bdf8' },
                      { id: 'emerald', label: 'Emerald', color: '#3fb950' },
                      { id: 'purple', label: 'Purple', color: '#a78bfa' }
                    ].map((c) => (
                      <button
                        key={c.id}
                        onClick={() => setAccentColor(c.id as any)}
                        className={`w-5 h-5 rounded-full border transition flex items-center justify-center ${
                          accentColor === c.id ? 'border-white scale-110' : 'border-transparent opacity-70 hover:opacity-100'
                        }`}
                        style={{ backgroundColor: c.color }}
                        title={c.label}
                      />
                    ))}
                  </div>
                </div>

                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="font-medium text-white block text-xs">Interface Scale & Zoom</span>
                      <span className="text-[11px] text-[#6e7681]">Scale the desktop interface smoothly across high-DPI displays</span>
                    </div>
                    <span className="font-mono text-[#58a6ff] bg-[#21262d] px-2 py-0.5 rounded border border-[#30363d] text-xs font-semibold">
                      {uiScale}%
                    </span>
                  </div>

                  <input
                    type="range"
                    min="70"
                    max="150"
                    step="10"
                    value={uiScale}
                    onChange={(e) => handleSetUiScale(parseInt(e.target.value, 10))}
                    className="w-full h-1.5 bg-[#0d1117] rounded-lg appearance-none cursor-pointer accent-[#58a6ff]"
                  />

                  <div className="flex items-center gap-1.5 pt-1 flex-wrap">
                    {[70, 80, 90, 100, 110, 120, 130, 140, 150].map((preset) => (
                      <button
                        key={preset}
                        onClick={() => handleSetUiScale(preset)}
                        className={`px-2 py-0.5 rounded text-[11px] font-mono transition ${
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

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-2">
                    <span className="font-medium text-white block text-xs">Editor Monospace Font</span>
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

                  <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-2">
                    <span className="font-medium text-white block text-xs">Layout Density</span>
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

            {/* 3. API KEYS & PROVIDERS TAB */}
            {activeTab === 'providers' && (
              <div className="space-y-5">
                <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                  <div>
                    <h3 className="text-sm font-semibold text-white mb-1">API Keys & Model Providers</h3>
                    <p className="text-[#8b949e] text-[11px]">Configure upstream model credentials, secrets, and OmniRoute priorities in one place</p>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <button
                      onClick={() => showToast('Health check: Anthropic 340ms, Gemini 140ms, DeepSeek 480ms - All healthy')}
                      className="px-2.5 py-1.5 rounded-lg bg-[#161b22] hover:bg-[#21262d] border border-[#30363d] text-[#c9d1d9] text-[11px] font-medium transition flex items-center gap-1.5"
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
                            <div className="w-8 h-8 rounded-lg bg-[#0d1117] border border-[#21262d] flex items-center justify-center p-1.5 shrink-0">
                              {p.iconType === 'anthropic' && <ClaudeIcon size={18} />}
                              {p.iconType === 'gemini' && <GeminiIcon size={18} />}
                              {p.iconType === 'deepseek' && <DeepSeekIcon size={18} />}
                              {p.iconType === 'openai' && <OpenAIIcon size={18} />}
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
                      <div className="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shrink-0">
                        <Cpu className="w-4 h-4" />
                      </div>
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="font-semibold text-white text-xs">Local Ollama Runtime</span>
                          <span className="text-[10px] px-2 py-0.2 rounded-full font-mono bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">Offline Local</span>
                        </div>
                        <span className="text-[11px] text-[#8b949e] font-mono">{localOllamaUrl} • DeepSeek R1, Llama 3.3</span>
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
                      className="px-2.5 py-1 rounded bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] text-white text-[11px] font-medium flex items-center gap-1 transition"
                    >
                      <Plus className="w-3 h-3" />
                      <span>New Key</span>
                    </button>
                  </div>
                  {clientKeys.map((k) => (
                    <div key={k.id} className="flex items-center justify-between p-2.5 rounded-lg bg-[#0d1117] border border-[#21262d] text-xs font-mono">
                      <div>
                        <span className="text-white block font-sans font-medium text-[11.5px]">{k.name}</span>
                        <span className="text-[#8b949e] text-[10.5px]">{k.token}</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => {
                            navigator.clipboard.writeText(k.token);
                            showToast('Gateway token copied to clipboard');
                          }}
                          className="px-2 py-1 rounded bg-[#161b22] hover:bg-[#21262d] border border-[#30363d] text-[#c9d1d9] hover:text-white transition text-[11px] flex items-center gap-1"
                        >
                          <Copy className="w-3 h-3" />
                          <span>Copy</span>
                        </button>
                        <button
                          onClick={() => handleRevokeClientKey(k.id)}
                          className="px-2 py-1 rounded bg-red-500/10 hover:bg-red-500/20 text-red-400 transition text-[11px]"
                        >
                          Revoke
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* 4. AI REASONING & INFERENCE TAB */}
            {activeTab === 'ai' && (
              <div className="space-y-5">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">AI Reasoning & Inference Defaults</h3>
                  <p className="text-[#8b949e] text-[11px]">Calibrate global temperature, max token window, and model selection</p>
                </div>

                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-4">
                  <div>
                    <label className="text-xs text-[#8b949e] block mb-1.5">Default Primary Model</label>
                    <CustomSelect
                      value={defaultModel}
                      onChange={setDefaultModel}
                      options={[
                        { 
                          value: 'claude-3-7-sonnet', 
                          label: 'Claude 3.7 Sonnet (Anthropic)',
                          icon: <img src="/assets/provider-logo/anthropic.jpg" alt="Anthropic" className="w-3.5 h-3.5 rounded object-cover" /> 
                        },
                        { 
                          value: 'gpt-4o', 
                          label: 'GPT-4o (OpenAI)',
                          icon: <img src="/assets/provider-logo/openai.jpg" alt="OpenAI" className="w-3.5 h-3.5 rounded object-cover" /> 
                        },
                        { 
                          value: 'deepseek-r1', 
                          label: 'DeepSeek R1 (Local Ollama)',
                          icon: <img src="/assets/provider-logo/deepseek.jpg" alt="DeepSeek" className="w-3.5 h-3.5 rounded object-cover" /> 
                        },
                        { 
                          value: 'gemini-2.5-pro', 
                          label: 'Gemini 2.5 Pro (Google Cloud)',
                          icon: <img src="/assets/provider-logo/gemini.jpg" alt="Google" className="w-3.5 h-3.5 rounded object-cover" /> 
                        },
                      ]}
                      headerTitle="Primary Inference Model"
                      headerBadge="4 Models"
                    />
                  </div>

                  <div>
                    <div className="flex justify-between text-xs text-[#8b949e] mb-1.5">
                      <span>Reasoning Temperature:</span>
                      <span className="font-mono text-white font-medium">{temperature}</span>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.05"
                      value={temperature}
                      onChange={(e) => setTemperature(parseFloat(e.target.value))}
                      className="w-full h-1.5 bg-[#0d1117] rounded-lg appearance-none cursor-pointer accent-purple-400"
                    />
                  </div>

                  <div>
                    <label className="text-xs text-[#8b949e] block mb-1.5">Max Output Tokens</label>
                    <CustomSelect
                      value={maxTokens}
                      onChange={setMaxTokens}
                      options={[
                        { value: '4096', label: '4,096 tokens' },
                        { value: '8192', label: '8,192 tokens' },
                        { value: '16384', label: '16,384 tokens' },
                      ]}
                      headerTitle="Max Token Budget"
                    />
                  </div>

                  <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                    <div className="flex items-center gap-2">
                      <Sparkles className="w-3.5 h-3.5 text-purple-400" />
                      <span className="text-xs text-[#c9d1d9]">Stream Token Output</span>
                    </div>
                    <input
                      type="checkbox"
                      checked={streamResponse}
                      onChange={(e) => setStreamResponse(e.target.checked)}
                      className="accent-purple-400"
                    />
                  </label>

                  <div>
                    <label className="text-xs text-[#8b949e] block mb-1">Global System Prompt</label>
                    <textarea
                      rows={3}
                      value={systemPrompt}
                      onChange={(e) => setSystemPrompt(e.target.value)}
                      className="w-full bg-[#0d1117] border border-[#30363d] rounded-lg p-2.5 text-xs text-[#c9d1d9] focus:outline-none focus:border-[#58a6ff] font-mono resize-none leading-relaxed"
                    />
                  </div>
                </div>
              </div>
            )}

            {/* 5. OMNIROUTE TAB */}
            {activeTab === 'omniroute' && (
              <div className="space-y-5">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">OmniRoute Intelligent Fallback</h3>
                  <p className="text-[#8b949e] text-[11px]">Dynamic latency-based model switching and offline Ollama failover</p>
                </div>

                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-4">
                  <div>
                    <div className="flex justify-between text-xs text-[#8b949e] mb-1.5">
                      <span>Auto Failover Trigger Latency:</span>
                      <span className="font-mono text-emerald-400 font-medium">{latencyThreshold}ms</span>
                    </div>
                    <input
                      type="range"
                      min="50"
                      max="500"
                      step="10"
                      value={latencyThreshold}
                      onChange={(e) => setLatencyThreshold(parseInt(e.target.value, 10))}
                      className="w-full h-1.5 bg-[#0d1117] rounded-lg appearance-none cursor-pointer accent-emerald-500"
                    />
                  </div>

                  <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                    <div>
                      <div className="text-xs font-medium text-white">Offline Local Fallback</div>
                      <div className="text-[11px] text-[#8b949e] mt-0.5">Route to local Ollama when internet is down or rate limits hit</div>
                    </div>
                    <input
                      type="checkbox"
                      checked={offlineLocalMode}
                      onChange={(e) => setOfflineLocalMode(e.target.checked)}
                      className="accent-emerald-500"
                    />
                  </label>

                  <div>
                    <label className="text-xs text-[#8b949e] block mb-1.5">Local Ollama API Endpoint</label>
                    <input
                      type="text"
                      value={localOllamaUrl}
                      onChange={(e) => setLocalOllamaUrl(e.target.value)}
                      className="w-full bg-[#0d1117] border border-[#30363d] rounded-lg px-3 py-2 text-xs text-[#c9d1d9] font-mono focus:outline-none focus:border-emerald-500"
                    />
                  </div>
                </div>
              </div>
            )}

            {/* 6. SECURITY TAB */}
            {activeTab === 'security' && (
              <div className="space-y-5">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">Security & Sandbox Isolation</h3>
                  <p className="text-[#8b949e] text-[11px]">Strict capability isolation, file mutation permits, and credential encryption</p>
                </div>

                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                    <div>
                      <div className="text-xs font-medium text-white">Require ExecutionPermit for File Writes</div>
                      <div className="text-[11px] text-[#8b949e] mt-0.5">Enforces sandbox validation before applying AST diff patches</div>
                    </div>
                    <input
                      type="checkbox"
                      checked={enforcePermits}
                      onChange={(e) => setEnforcePermits(e.target.checked)}
                      className="accent-emerald-500"
                    />
                  </label>

                  <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                    <div>
                      <div className="text-xs font-medium text-white">Require Confirmation for Shell Commands</div>
                      <div className="text-[11px] text-[#8b949e] mt-0.5">Prompt before running bash / powershell scripts in terminal</div>
                    </div>
                    <input
                      type="checkbox"
                      checked={confirmCommands}
                      onChange={(e) => setConfirmCommands(e.target.checked)}
                      className="accent-emerald-500"
                    />
                  </label>

                  <label className="flex items-center justify-between p-3 rounded-lg bg-[#0d1117] border border-[#21262d] cursor-pointer hover:border-[#30363d] transition">
                    <div>
                      <div className="text-xs font-medium text-white">Hardware Encrypted Credential Vault</div>
                      <div className="text-[11px] text-[#8b949e] mt-0.5">Protect API keys with OS Keychain / DPAPI</div>
                    </div>
                    <input
                      type="checkbox"
                      checked={encryptedStorage}
                      onChange={(e) => setEncryptedStorage(e.target.checked)}
                      className="accent-emerald-500"
                    />
                  </label>
                </div>
              </div>
            )}

            {/* 7. SHORTCUTS TAB */}
            {activeTab === 'shortcuts' && (
              <div className="space-y-5">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">Keyboard Shortcuts</h3>
                  <p className="text-[#8b949e] text-[11px]">Accelerate your autonomous engineering workflow</p>
                </div>

                <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3">
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5 font-mono text-xs">
                    {[
                      { action: 'Open Settings', key: `${modKey},` },
                      { action: 'API Keys & Models', key: `${modKey}${altKey}A` },
                      { action: 'Toggle Orca Tools', key: `${modKey}${altKey}O` },
                      { action: 'Switch to Chat', key: `${modKey}1` },
                      { action: 'Switch to Code ADE', key: `${modKey}2` },
                      { action: 'Switch to Research', key: `${modKey}3` },
                      { action: 'New Session', key: `${modKey}N` },
                      { action: 'Toggle Sidebar', key: `${modKey}B` },
                      { action: 'Toggle Diff Split', key: `${modKey}\\` },
                      { action: 'Quick Command Palette', key: `${modKey}K` },
                    ].map((s, idx) => (
                      <div key={idx} className="flex items-center justify-between p-2.5 rounded-lg bg-[#0d1117] border border-[#21262d]">
                        <span className="text-[#c9d1d9] font-sans text-xs">{s.action}</span>
                        <kbd className="px-2 py-0.5 rounded bg-[#21262d] text-[#58a6ff] border border-[#30363d] text-[11px] font-mono">
                          {s.key}
                        </kbd>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            )}

            {/* 8. ABOUT TAB */}
            {activeTab === 'about' && (
              <div className="space-y-5">
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">About Custos Desktop</h3>
                  <p className="text-[#8b949e] text-[11px]">System runtime environment and architecture</p>
                </div>

                <div className="p-5 rounded-xl bg-[#161b22] border border-[#21262d] space-y-4">
                  <div className="flex items-center gap-3.5">
                    <div className="w-12 h-12 rounded-xl bg-[#0d1117] border border-[#30363d] flex items-center justify-center p-2 shadow-inner">
                      <img src="/assets/custos-owl.png" alt="Custos" className="w-full h-full object-contain" />
                    </div>
                    <div>
                      <div className="text-sm font-bold text-white">Custos Desktop • SADE Architecture</div>
                      <div className="text-xs text-[#8b949e]">v0.3.1-beta • Rust 1.85 Kernel • Tauri 2.x • React 18</div>
                    </div>
                  </div>

                  <div className="p-3 rounded-lg bg-[#0d1117] border border-[#21262d] text-xs text-[#8b949e] space-y-1.5 leading-relaxed">
                    <p>
                      Custos combines a Claude-inspired interactive chat canvas, an OpenAI Codex AST patch workbench, and an autonomous research lab into a unified multi-modal IDE.
                    </p>
                    <p className="text-[#6e7681]">
                      Engineered for high-density autonomous code execution with hardware cryptographic sandboxing.
                    </p>
                  </div>

                  <div className="flex items-center justify-between pt-2 border-t border-[#21262d]">
                    <span className="text-xs text-[#6e7681] font-mono">Build 2026.10 • Apache 2.0 / MIT</span>
                    <button
                      onClick={() => showToast('Custos is already up to date')}
                      className="px-3 py-1.5 bg-[#21262d] hover:bg-[#30363d] border border-[#30363d] rounded-lg text-xs text-[#c9d1d9] font-medium transition flex items-center gap-1.5"
                    >
                      <RefreshCw className="w-3.5 h-3.5" />
                      <span>Check for Updates</span>
                    </button>
                  </div>
                </div>
              </div>
            )}

          </div>
        </main>
      </div>
    </div>
  );
};

export default SettingsPage;
