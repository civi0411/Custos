import React, { useState } from 'react';
import { SlidersHorizontal, X, GitFork, KeyRound, ShieldAlert, Database } from 'lucide-react';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onOpenProviders: () => void;
  onShowToast: (msg: string) => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  onClose,
  onOpenProviders,
  onShowToast
}) => {
  const [activeTab, setActiveTab] = useState<'omniroute' | 'failover' | 'cache'>('omniroute');

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/85 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-3xl h-[560px] max-h-[92vh] bg-black border border-surface-border rounded-2xl shadow-2xl flex flex-col overflow-hidden">
        {/* Header */}
        <div className="h-12 border-b border-surface-border px-4 sm:px-6 flex items-center justify-between shrink-0 bg-surface">
          <div className="flex items-center gap-2.5">
            <SlidersHorizontal className="w-4 h-4 text-brand-blue" />
            <span className="text-sm font-semibold text-white">Platform Preferences</span>
          </div>
          <button 
            onClick={onClose} 
            className="p-1 hover:bg-surface-elevated rounded-lg text-neutral-400 hover:text-white transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 flex flex-col md:flex-row overflow-hidden">
          {/* Tab Sidebar */}
          <aside className="w-full md:w-52 border-b md:border-b-0 md:border-r border-surface-border p-2 md:p-3 flex md:flex-col gap-1 bg-surface/30 shrink-0 text-xs overflow-x-auto">
            <button 
              onClick={() => setActiveTab('omniroute')}
              className={`text-left px-3 py-2 rounded-lg font-medium flex items-center gap-2 shrink-0 transition ${
                activeTab === 'omniroute'
                  ? 'bg-surface-card text-brand-blue border border-surface-border'
                  : 'text-neutral-400 hover:text-neutral-200 hover:bg-surface-elevated border border-transparent'
              }`}
            >
              <GitFork className="w-3.5 h-3.5" />
              <span>OmniRoute</span>
            </button>

            <button 
              onClick={() => {
                onClose();
                onOpenProviders();
              }} 
              className="text-left px-3 py-2 rounded-lg text-neutral-400 hover:text-neutral-200 hover:bg-surface-elevated flex items-center gap-2 transition shrink-0"
            >
              <KeyRound className="w-3.5 h-3.5" />
              <span>Providers</span>
            </button>

            <button 
              onClick={() => setActiveTab('failover')}
              className={`text-left px-3 py-2 rounded-lg font-medium flex items-center gap-2 shrink-0 transition ${
                activeTab === 'failover'
                  ? 'bg-surface-card text-brand-blue border border-surface-border'
                  : 'text-neutral-400 hover:text-neutral-200 hover:bg-surface-elevated border border-transparent'
              }`}
            >
              <ShieldAlert className="w-3.5 h-3.5" />
              <span>Failover</span>
            </button>

            <button 
              onClick={() => setActiveTab('cache')}
              className={`text-left px-3 py-2 rounded-lg font-medium flex items-center gap-2 shrink-0 transition ${
                activeTab === 'cache'
                  ? 'bg-surface-card text-brand-blue border border-surface-border'
                  : 'text-neutral-400 hover:text-neutral-200 hover:bg-surface-elevated border border-transparent'
              }`}
            >
              <Database className="w-3.5 h-3.5" />
              <span>Cache</span>
            </button>
          </aside>

          {/* Content Area */}
          <div className="flex-1 overflow-y-auto p-4 sm:p-6 space-y-6 text-xs text-neutral-300">
            {activeTab === 'omniroute' && (
              <>
                <div>
                  <h3 className="text-sm font-semibold text-white mb-1">OmniRoute Routing Chain</h3>
                  <p className="text-neutral-500 text-[11px] mb-4">
                    Define priority routing fallback paths when token quotas or 429 rate limits occur.
                  </p>
                  
                  <div className="border border-surface-border rounded-xl overflow-x-auto bg-surface-card">
                    <div className="min-w-[420px]">
                      <div className="grid grid-cols-12 px-3 py-2 border-b border-surface-border text-[11px] font-mono text-neutral-500 bg-surface">
                        <span className="col-span-1">PRI</span>
                        <span className="col-span-4">ALIAS</span>
                        <span className="col-span-5">TARGET MODEL</span>
                        <span className="col-span-2 text-right">STATUS</span>
                      </div>
                      <div className="grid grid-cols-12 px-3 py-2.5 items-center border-b border-surface-border/50 font-mono text-[11px]">
                        <span className="col-span-1 text-brand-blue font-bold">1</span>
                        <span className="col-span-4 text-white">default-agent</span>
                        <span className="col-span-5 text-neutral-300">claude-3-7-sonnet</span>
                        <div className="col-span-2 text-right">
                          <span className="text-emerald-400 bg-emerald-500/10 px-1.5 py-0.5 rounded text-[10px]">Active</span>
                        </div>
                      </div>
                      <div className="grid grid-cols-12 px-3 py-2.5 items-center font-mono text-[11px]">
                        <span className="col-span-1 text-neutral-500 font-bold">2</span>
                        <span className="col-span-4 text-neutral-400">fallback-rate-limit</span>
                        <span className="col-span-5 text-neutral-300">gemini-2.5-pro</span>
                        <div className="col-span-2 text-right">
                          <span className="text-neutral-400 bg-surface-elevated px-1.5 py-0.5 rounded text-[10px]">Standby</span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="space-y-3 pt-2">
                  <h4 className="text-xs font-semibold text-white">Latency Thresholds</h4>
                  <div className="flex items-center justify-between bg-surface-card p-3 rounded-xl border border-surface-border">
                    <div>
                      <div className="font-medium text-neutral-200">Auto Failover Trigger</div>
                      <div className="text-[11px] text-neutral-500">Route to next standby if provider latency exceeds target</div>
                    </div>
                    <span className="text-neutral-300 font-mono bg-surface-elevated px-2 py-1 rounded border border-surface-border">
                      150ms
                    </span>
                  </div>
                </div>
              </>
            )}

            {activeTab === 'failover' && (
              <div className="space-y-4">
                <h3 className="text-sm font-semibold text-white mb-1">Failover & Circuit Breaker</h3>
                <p className="text-neutral-500 text-[11px]">
                  Configure automatic trip conditions for degrading remote endpoints.
                </p>

                <div className="space-y-2">
                  <div className="flex items-center justify-between p-3 rounded-xl bg-surface-card border border-surface-border">
                    <div>
                      <div className="font-medium text-white">Consecutive Failures Threshold</div>
                      <div className="text-[11px] text-neutral-500">Number of 5xx errors before opening circuit</div>
                    </div>
                    <span className="font-mono text-neutral-300 bg-surface px-2 py-1 rounded">3 strikes</span>
                  </div>

                  <div className="flex items-center justify-between p-3 rounded-xl bg-surface-card border border-surface-border">
                    <div>
                      <div className="font-medium text-white">Offline Local Mode</div>
                      <div className="text-[11px] text-neutral-500">Fall back to Ollama / local inference when network drops</div>
                    </div>
                    <span className="text-emerald-400 font-mono text-xs">Enabled</span>
                  </div>
                </div>
              </div>
            )}

            {activeTab === 'cache' && (
              <div className="space-y-4">
                <h3 className="text-sm font-semibold text-white mb-1">SQLite & Key-Value Cache</h3>
                <p className="text-neutral-500 text-[11px]">
                  Local encrypted cache store for prompt tokens, embeddings and schema introspection.
                </p>

                <div className="p-3 rounded-xl bg-surface-card border border-surface-border space-y-2">
                  <div className="flex justify-between items-center text-xs">
                    <span className="text-neutral-400">Cache Size</span>
                    <span className="font-mono text-white">12.4 MB (420 entries)</span>
                  </div>
                  <div className="flex justify-between items-center text-xs">
                    <span className="text-neutral-400">Database Location</span>
                    <span className="font-mono text-neutral-400 text-[11px]">~/.custos/store.db</span>
                  </div>
                </div>

                <button 
                  onClick={() => onShowToast('Local cache purged successfully')} 
                  className="px-3 py-1.5 rounded-lg bg-surface-elevated hover:bg-surface-hover text-red-400 text-xs border border-surface-border transition"
                >
                  Purge Local Cache
                </button>
              </div>
            )}

            {/* Keyboard Shortcuts Reference */}
            <div className="pt-4 border-t border-surface-border/80">
              <h4 className="text-xs font-semibold uppercase tracking-wider text-neutral-500 mb-2">
                Desktop Keyboard Shortcuts
              </h4>
              <div className="grid grid-cols-2 gap-2 font-mono text-[11px]">
                <div className="flex justify-between bg-surface-card px-2.5 py-1.5 rounded border border-surface-border/60">
                  <span className="text-neutral-400">Preferences</span>
                  <kbd className="text-neutral-200">⌘,</kbd>
                </div>
                <div className="flex justify-between bg-surface-card px-2.5 py-1.5 rounded border border-surface-border/60">
                  <span className="text-neutral-400">Toggle Sessions</span>
                  <kbd className="text-neutral-200">⌘B</kbd>
                </div>
                <div className="flex justify-between bg-surface-card px-2.5 py-1.5 rounded border border-surface-border/60">
                  <span className="text-neutral-400">Toggle Split View</span>
                  <kbd className="text-neutral-200">⌘\</kbd>
                </div>
                <div className="flex justify-between bg-surface-card px-2.5 py-1.5 rounded border border-surface-border/60">
                  <span className="text-neutral-400">UI Zoom In / Out</span>
                  <kbd className="text-neutral-200">⌘+ / ⌘-</kbd>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="h-12 border-t border-surface-border px-4 sm:px-6 flex items-center justify-end gap-2 bg-surface shrink-0">
          <button 
            onClick={onClose} 
            className="px-3 py-1.5 rounded-lg hover:bg-surface-elevated text-neutral-400 transition text-xs"
          >
            Cancel
          </button>
          <button 
            onClick={() => {
              onShowToast('Preferences saved');
              onClose();
            }} 
            className="px-3.5 py-1.5 rounded-lg bg-brand-blue hover:bg-blue-600 text-white font-medium transition text-xs"
          >
            Save Preferences
          </button>
        </div>
      </div>
    </div>
  );
};
