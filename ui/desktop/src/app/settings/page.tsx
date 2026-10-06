import React, { useState } from 'react';
import { 
  SlidersHorizontal, 
  Monitor, 
  Cpu, 
  Shield, 
  Save, 
  RotateCcw, 
  Folder, 
  Bell, 
  Sparkles, 
  GitFork, 
  Keyboard, 
  RefreshCw,
  Check
} from 'lucide-react';
import { useAppContext } from '../../context/AppContext';
import { CustomSelect } from '../../components/CustomSelect';
import { Tooltip } from '../../components/Tooltip';

export const SettingsPage: React.FC = () => {
  const { uiScale, handleSetUiScale, showToast } = useAppContext();

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

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto p-4 sm:p-6 lg:p-8 space-y-6 select-text">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-surface-border">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-brand-blue/10 border border-brand-blue/30 text-brand-blue">
              <SlidersHorizontal className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-lg font-bold text-white tracking-tight">System Preferences</h1>
              <p className="text-xs text-neutral-400">Configure application behavior, appearance, autonomous reasoning defaults, and security policies</p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleReset}
            className="px-3 py-1.5 bg-surface-elevated hover:bg-surface-hover border border-surface-border rounded-lg text-xs font-medium text-neutral-300 transition flex items-center gap-1.5"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Reset Defaults</span>
          </button>
          <button
            onClick={handleSave}
            className="px-3.5 py-1.5 bg-brand-blue hover:bg-blue-600 text-white rounded-lg text-xs font-medium transition flex items-center gap-1.5 shadow-lg shadow-brand-blue/20"
          >
            <Save className="w-3.5 h-3.5" />
            <span>Save Preferences</span>
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 max-w-6xl">
        {/* 1. General & Workspace */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4 md:col-span-2">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Folder className="w-4 h-4 text-brand-blue" />
            <span>General & Workspace</span>
          </div>

          <div className="space-y-3">
            <div>
              <label className="text-xs text-neutral-400 block mb-1.5">Default Workspace Directory</label>
              <div className="flex items-center gap-2">
                <input
                  type="text"
                  value={defaultWorkspace}
                  onChange={(e) => setDefaultWorkspace(e.target.value)}
                  className="flex-1 bg-surface-elevated border border-surface-border rounded-lg px-3 py-2 text-xs text-neutral-200 font-mono focus:outline-none"
                />
                <button
                  onClick={() => showToast('Folder path selected')}
                  className="px-3 py-2 bg-surface hover:bg-surface-hover border border-surface-border rounded-lg text-xs text-neutral-300 font-medium transition"
                >
                  Browse
                </button>
              </div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 pt-1">
              <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
                <span className="text-xs text-neutral-300">Launch at startup</span>
                <input
                  type="checkbox"
                  checked={launchAtStartup}
                  onChange={(e) => setLaunchAtStartup(e.target.checked)}
                />
              </label>
              <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
                <span className="text-xs text-neutral-300">Reopen last project</span>
                <input
                  type="checkbox"
                  checked={reopenLastProject}
                  onChange={(e) => setReopenLastProject(e.target.checked)}
                />
              </label>
              <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
                <span className="text-xs text-neutral-300">Confirm on close</span>
                <input
                  type="checkbox"
                  checked={confirmExit}
                  onChange={(e) => setConfirmExit(e.target.checked)}
                />
              </label>
              <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
                <span className="text-xs text-neutral-300">Auto-save sessions</span>
                <input
                  type="checkbox"
                  checked={autoSave}
                  onChange={(e) => setAutoSave(e.target.checked)}
                />
              </label>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
              <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
                <div className="flex items-center gap-2">
                  <Bell className="w-3.5 h-3.5 text-brand-blue" />
                  <span className="text-xs text-neutral-300">System Notifications</span>
                </div>
                <input
                  type="checkbox"
                  checked={notifications}
                  onChange={(e) => setNotifications(e.target.checked)}
                />
              </label>

              <div className="flex items-center justify-between p-2.5 rounded-lg bg-surface-elevated border border-surface-border">
                <span className="text-xs text-neutral-300">Interface Language</span>
                <div className="w-44 shrink-0">
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

        {/* 2. Appearance & Scaling */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Monitor className="w-4 h-4 text-brand-blue" />
            <span>Appearance & Themes</span>
          </div>

          <div className="space-y-3">
            {/* Theme Presets */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
              {[
                { id: 'dark', label: 'Dark', color: '#131419' },
                { id: 'midnight', label: 'Midnight', color: '#0b1120' },
                { id: 'slate', label: 'Slate', color: '#18181b' },
                { id: 'light', label: 'Light', color: '#f8fafc' }
              ].map((t) => (
                <button
                  key={t.id}
                  onClick={() => setTheme(t.id as any)}
                  className={`p-2 rounded-lg border text-left flex items-center justify-between text-xs transition ${
                    theme === t.id
                      ? 'border-brand-blue bg-surface-elevated text-white'
                      : 'border-surface-border text-neutral-400 hover:text-white bg-surface'
                  }`}
                >
                  <span className="flex items-center gap-1.5">
                    <span className="w-3 h-3 rounded-full border border-white/20" style={{ backgroundColor: t.color }} />
                    <span>{t.label}</span>
                  </span>
                  {theme === t.id && <Check className="w-3 h-3 text-brand-blue" />}
                </button>
              ))}
            </div>

            {/* Accent Color */}
            <div className="flex items-center gap-2 pt-1">
              <span className="text-xs text-neutral-400">Accent:</span>
              {[
                { id: 'blue', label: 'Blue', color: '#0070f3' },
                { id: 'cyan', label: 'Cyan', color: '#00d4ff' },
                { id: 'emerald', label: 'Emerald', color: '#10b981' },
                { id: 'purple', label: 'Purple', color: '#a855f7' }
              ].map((c) => (
                <Tooltip key={c.id} content={c.label} position="top">
                  <button
                    onClick={() => setAccentColor(c.id as any)}
                    className={`w-5 h-5 rounded-full border transition flex items-center justify-center ${
                      accentColor === c.id ? 'border-white scale-110' : 'border-transparent opacity-70 hover:opacity-100'
                    }`}
                    style={{ backgroundColor: c.color }}
                  />
                </Tooltip>
              ))}
            </div>

            {/* UI Scale */}
            <div className="pt-2">
              <div className="flex items-center justify-between text-xs mb-1.5">
                <span className="text-neutral-400">Interface Zoom:</span>
                <span className="font-mono text-white font-medium">{uiScale}%</span>
              </div>
              <input
                type="range"
                min="75"
                max="150"
                step="5"
                value={uiScale}
                onChange={(e) => handleSetUiScale(parseInt(e.target.value, 10))}
                className="w-full h-1.5 bg-surface-elevated rounded-lg appearance-none cursor-pointer accent-brand-blue"
              />
              <div className="flex gap-2 pt-2">
                {[85, 90, 100, 110, 125].map((preset) => (
                  <button
                    key={preset}
                    onClick={() => handleSetUiScale(preset)}
                    className={`px-2 py-0.5 rounded text-[11px] font-mono transition ${
                      uiScale === preset
                        ? 'bg-brand-blue text-white'
                        : 'bg-surface-elevated text-neutral-400 hover:text-white'
                    }`}
                  >
                    {preset}%
                  </button>
                ))}
              </div>
            </div>

            <div>
              <label className="text-xs text-neutral-400 block mb-1">Monospace Font</label>
              <CustomSelect
                value={fontFamily}
                onChange={setFontFamily}
                options={[
                  { value: 'JetBrains Mono', label: 'JetBrains Mono' },
                  { value: 'Fira Code', label: 'Fira Code' },
                  { value: 'Cascadia Code', label: 'Cascadia Code' },
                ]}
                headerTitle="Monospace Font"
              />
            </div>
          </div>
        </div>

        {/* 3. Model Defaults */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Cpu className="w-4 h-4 text-purple-400" />
            <span>AI Reasoning & Inference Defaults</span>
          </div>

          <div className="space-y-3">
            <div>
              <label className="text-xs text-neutral-400 block mb-1.5">Default Primary Model</label>
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
              <div className="flex justify-between text-xs text-neutral-400 mb-1.5">
                <span>Reasoning Temperature:</span>
                <span className="font-mono text-white">{temperature}</span>
              </div>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                value={temperature}
                onChange={(e) => setTemperature(parseFloat(e.target.value))}
                className="w-full h-1.5 bg-surface-elevated rounded-lg appearance-none cursor-pointer accent-purple-500"
              />
            </div>

            <div>
              <label className="text-xs text-neutral-400 block mb-1.5">Max Output Tokens</label>
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

            <label className="flex items-center justify-between p-2.5 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
              <div className="flex items-center gap-2">
                <Sparkles className="w-3.5 h-3.5 text-purple-400" />
                <span className="text-xs text-neutral-300">Stream Token Output</span>
              </div>
              <input
                type="checkbox"
                checked={streamResponse}
                onChange={(e) => setStreamResponse(e.target.checked)}
                className="accent-purple-500"
              />
            </label>

            <div>
              <label className="text-xs text-neutral-400 block mb-1">Global System Prompt</label>
              <textarea
                rows={2}
                value={systemPrompt}
                onChange={(e) => setSystemPrompt(e.target.value)}
                className="w-full bg-surface-elevated border border-surface-border rounded-lg p-2 text-xs text-neutral-200 focus:outline-none font-mono resize-none"
              />
            </div>
          </div>
        </div>

        {/* 4. OmniRoute & Failover */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <GitFork className="w-4 h-4 text-emerald-400" />
            <span>OmniRoute & Local Fallback</span>
          </div>

          <div className="space-y-3">
            <div>
              <div className="flex justify-between text-xs text-neutral-400 mb-1.5">
                <span>Auto Failover Trigger:</span>
                <span className="font-mono text-white">{latencyThreshold}ms</span>
              </div>
              <input
                type="range"
                min="50"
                max="500"
                step="10"
                value={latencyThreshold}
                onChange={(e) => setLatencyThreshold(parseInt(e.target.value, 10))}
                className="w-full h-1.5 bg-surface-elevated rounded-lg appearance-none cursor-pointer accent-emerald-500"
              />
            </div>

            <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
              <div>
                <div className="text-xs font-medium text-white">Offline Local Fallback</div>
                <div className="text-[11px] text-neutral-400 mt-0.5">Route to local Ollama when internet is down</div>
              </div>
              <input
                type="checkbox"
                checked={offlineLocalMode}
                onChange={(e) => setOfflineLocalMode(e.target.checked)}
                className="accent-emerald-500"
              />
            </label>
          </div>
        </div>

        {/* 5. Security & Sandboxing */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Shield className="w-4 h-4 text-emerald-400" />
            <span>Security & Kernel Enforcement</span>
          </div>

          <div className="space-y-3">
            <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
              <div>
                <div className="text-xs font-medium text-white">Require ExecutionPermit for File Writes</div>
                <div className="text-[11px] text-neutral-400 mt-0.5">Enforces sandbox validation before applying AST diff patches</div>
              </div>
              <input
                type="checkbox"
                checked={enforcePermits}
                onChange={(e) => setEnforcePermits(e.target.checked)}
                className="accent-emerald-500"
              />
            </label>

            <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
              <div>
                <div className="text-xs font-medium text-white">Require Confirmation for Shell Commands</div>
                <div className="text-[11px] text-neutral-400 mt-0.5">Prompt before running scripts in terminal</div>
              </div>
              <input
                type="checkbox"
                checked={confirmCommands}
                onChange={(e) => setConfirmCommands(e.target.checked)}
                className="accent-emerald-500"
              />
            </label>

            <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer hover:border-surface-borderHover transition select-none">
              <div>
                <div className="text-xs font-medium text-white">Hardware Encrypted Credential Vault</div>
                <div className="text-[11px] text-neutral-400 mt-0.5">Protect API keys with OS Keychain / DPAPI</div>
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

        {/* 6. Shortcuts Reference */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4 md:col-span-2">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Keyboard className="w-4 h-4 text-cyan-400" />
            <span>Keyboard Shortcuts</span>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 font-mono text-[11px]">
            <div className="flex justify-between bg-surface-elevated px-2.5 py-2 rounded-lg border border-surface-border/60">
              <span className="text-neutral-400">Settings</span>
              <kbd className="text-neutral-200">Ctrl+,</kbd>
            </div>
            <div className="flex justify-between bg-surface-elevated px-2.5 py-2 rounded-lg border border-surface-border/60">
              <span className="text-neutral-400">Sessions</span>
              <kbd className="text-neutral-200">Ctrl+B</kbd>
            </div>
            <div className="flex justify-between bg-surface-elevated px-2.5 py-2 rounded-lg border border-surface-border/60">
              <span className="text-neutral-400">Split View</span>
              <kbd className="text-neutral-200">Ctrl+\</kbd>
            </div>
            <div className="flex justify-between bg-surface-elevated px-2.5 py-2 rounded-lg border border-surface-border/60">
              <span className="text-neutral-400">New Session</span>
              <kbd className="text-neutral-200">Ctrl+N</kbd>
            </div>
          </div>
        </div>

        {/* 7. About */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-3 md:col-span-2 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-surface-elevated border border-surface-border flex items-center justify-center p-1.5">
              <img src="/assets/custos-owl.png" alt="Custos" className="w-full h-full object-contain" />
            </div>
            <div>
              <div className="text-xs font-bold text-white">Custos Desktop v0.3.1-beta</div>
              <div className="text-[11px] text-neutral-400">Tauri 2.x • Rust 1.85 • WebView2 • Apache 2.0 / MIT</div>
            </div>
          </div>
          <button
            onClick={() => showToast('Custos is already up to date')}
            className="px-3 py-1.5 bg-surface-elevated hover:bg-surface-hover border border-surface-border rounded-lg text-xs text-neutral-300 font-medium transition flex items-center gap-1.5 self-start sm:self-auto"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            <span>Check for Updates</span>
          </button>
        </div>
      </div>
    </div>
  );
};

export default SettingsPage;
