import React, { useState } from 'react';
import { SlidersHorizontal, Monitor, Cpu, Shield, Save, RotateCcw } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';

export const SettingsPage: React.FC = () => {
  const { uiScale, handleSetUiScale, showToast } = useAppContext();
  const [defaultModel, setDefaultModel] = useState('claude-3-7-sonnet');
  const [temperature, setTemperature] = useState(0.2);
  const [enforcePermits, setEnforcePermits] = useState(true);

  const handleSave = () => {
    showToast('Preferences saved successfully');
  };

  const handleReset = () => {
    handleSetUiScale(100);
    setDefaultModel('claude-3-7-sonnet');
    setTemperature(0.2);
    setEnforcePermits(true);
    showToast('Preferences reset to defaults');
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto p-4 sm:p-6 lg:p-8 space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-surface-border">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-brand-blue/10 border border-brand-blue/30 text-brand-blue">
              <SlidersHorizontal className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-lg font-bold text-white tracking-tight">System Preferences</h1>
              <p className="text-xs text-neutral-400">Configure appearance, autonomous reasoning defaults, and security policies</p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleReset}
            className="px-3 py-1.5 bg-surface-elevated hover:bg-surface-hover border border-surface-border rounded-lg text-xs font-medium text-neutral-300 transition flex items-center gap-1.5"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Reset</span>
          </button>
          <button
            onClick={handleSave}
            className="px-3 py-1.5 bg-brand-blue hover:bg-brand-blueHover text-white rounded-lg text-xs font-medium transition flex items-center gap-1.5 shadow-lg shadow-brand-blueGlow"
          >
            <Save className="w-3.5 h-3.5" />
            <span>Save Changes</span>
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Appearance & Scaling */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Monitor className="w-4 h-4 text-brand-blue" />
            <span>Display & Interface Scale</span>
          </div>

          <div className="space-y-3">
            <div className="flex items-center justify-between text-xs">
              <span className="text-neutral-400">Current UI Scale:</span>
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
            <div className="flex gap-2 pt-1">
              {[85, 90, 100, 110, 125].map((preset) => (
                <button
                  key={preset}
                  onClick={() => handleSetUiScale(preset)}
                  className={`px-2 py-1 rounded text-[11px] font-mono transition ${
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
        </div>

        {/* Model Defaults */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Cpu className="w-4 h-4 text-purple-400" />
            <span>Inference Defaults</span>
          </div>

          <div className="space-y-3">
            <div>
              <label className="text-xs text-neutral-400 block mb-1.5">Default Primary Model</label>
              <select
                value={defaultModel}
                onChange={(e) => setDefaultModel(e.target.value)}
                className="w-full bg-surface-elevated border border-surface-border rounded-lg px-3 py-2 text-xs text-white focus:outline-none"
              >
                <option value="claude-3-7-sonnet">Claude 3.7 Sonnet (Anthropic)</option>
                <option value="gpt-4o">GPT-4o (OpenAI)</option>
                <option value="deepseek-r1">DeepSeek R1 (Local Ollama)</option>
              </select>
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
          </div>
        </div>

        {/* Security & Sandboxing */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4 md:col-span-2">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Shield className="w-4 h-4 text-emerald-400" />
            <span>Security & Kernel Enforcement (custos-security)</span>
          </div>

          <div className="space-y-3">
            <label className="flex items-center justify-between p-3 rounded-lg bg-surface-elevated border border-surface-border cursor-pointer">
              <div>
                <div className="text-xs font-medium text-white">Require ExecutionPermit for File Writes</div>
                <div className="text-[11px] text-neutral-400 mt-0.5">Enforces sandbox validation before applying AST diff patches</div>
              </div>
              <input
                type="checkbox"
                checked={enforcePermits}
                onChange={(e) => setEnforcePermits(e.target.checked)}
                className="rounded border-surface-border bg-canvas text-brand-blue focus:ring-0 w-4 h-4"
              />
            </label>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SettingsPage;
