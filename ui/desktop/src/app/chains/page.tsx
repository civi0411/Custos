import React, { useState } from 'react';
import { GitFork, ShieldCheck, Zap, Activity, RefreshCw } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';

export const ChainsPage: React.FC = () => {
  const { showToast } = useAppContext();
  const [latencyThreshold, setLatencyThreshold] = useState(150);
  const [maxRetries, setMaxRetries] = useState(2);
  const [autoFallback, setAutoFallback] = useState(true);

  return (
    <div className="flex-1 flex flex-col h-full bg-surface overflow-y-auto p-4 sm:p-6 lg:p-8 space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-surface-border">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-brand-blue/10 border border-brand-blue/30 text-brand-blue">
              <GitFork className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-lg font-bold text-white tracking-tight">OmniRoute Chains</h1>
              <p className="text-xs text-neutral-400">Intelligent multi-model routing, latency circuit breakers, and zero-downtime failover</p>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => showToast('Routing topology refreshed')}
            className="px-3 py-1.5 bg-surface-elevated hover:bg-surface-hover border border-surface-border rounded-lg text-xs font-medium text-neutral-300 transition flex items-center gap-1.5"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            <span>Test Chain</span>
          </button>
          <span className="px-2.5 py-1 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-xs font-medium flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            Active Policy
          </span>
        </div>
      </div>

      {/* Visual Pipeline Graph */}
      <div className="p-5 bg-surface-card border border-surface-border rounded-xl">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-neutral-400 mb-4 flex items-center gap-2">
          <Activity className="w-4 h-4 text-brand-blue" />
          Live Route Resolution Pipeline
        </h2>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 relative">
          {/* Step 1: Primary Model */}
          <div className="p-4 rounded-xl bg-surface-elevated/70 border border-blue-500/30 relative overflow-hidden group hover:border-blue-500/60 transition">
            <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-blue-500 to-cyan-400"></div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-blue-500/10 text-blue-400 border border-blue-500/20">
                Tier 1 • Primary
              </span>
              <span className="text-xs font-mono text-emerald-400">42ms avg</span>
            </div>
            <div className="flex items-center gap-2 mb-1">
              <img src="/assets/provider-logo/anthropic.jpg" alt="Anthropic" className="w-5 h-5 rounded object-contain border border-surface-border/60 shadow-sm" />
              <h3 className="text-sm font-semibold text-white">Claude 3.7 Sonnet</h3>
            </div>
            <p className="text-xs text-neutral-400 mt-1">High-reasoning code synthesis & complex architectural refactoring</p>
            <div className="mt-3 pt-3 border-t border-surface-border/60 flex items-center justify-between text-[11px] text-neutral-400">
              <span>Failure trigger:</span>
              <span className="text-neutral-200 font-mono">&gt;{latencyThreshold}ms or 5xx</span>
            </div>
          </div>

          {/* Step 2: Failover Route */}
          <div className="p-4 rounded-xl bg-surface-elevated/70 border border-purple-500/30 relative overflow-hidden group hover:border-purple-500/60 transition">
            <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-purple-500 to-pink-500"></div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-purple-500/10 text-purple-400 border border-purple-500/20">
                Tier 2 • Secondary
              </span>
              <span className="text-xs font-mono text-neutral-400">110ms avg</span>
            </div>
            <div className="flex items-center gap-2 mb-1">
              <img src="/assets/provider-logo/openai.jpg" alt="OpenAI" className="w-5 h-5 rounded object-contain border border-surface-border/60 shadow-sm" />
              <h3 className="text-sm font-semibold text-white">OpenAI GPT-4o</h3>
            </div>
            <p className="text-xs text-neutral-400 mt-1">Fast code generation failover route when primary latency spikes</p>
            <div className="mt-3 pt-3 border-t border-surface-border/60 flex items-center justify-between text-[11px] text-neutral-400">
              <span>Failure trigger:</span>
              <span className="text-neutral-200 font-mono">Rate-limit / Timeout</span>
            </div>
          </div>

          {/* Step 3: Local Offline Fallback */}
          <div className="p-4 rounded-xl bg-surface-elevated/70 border border-emerald-500/30 relative overflow-hidden group hover:border-emerald-500/60 transition">
            <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-emerald-500 to-teal-400"></div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                Tier 3 • Offline Local
              </span>
              <span className="text-xs font-mono text-emerald-400">0ms network</span>
            </div>
            <div className="flex items-center gap-2 mb-1">
              <img src="/assets/provider-logo/deepseek.jpg" alt="DeepSeek" className="w-5 h-5 rounded object-contain border border-surface-border/60 shadow-sm" />
              <h3 className="text-sm font-semibold text-white">DeepSeek R1 (Ollama / Local)</h3>
            </div>
            <p className="text-xs text-neutral-400 mt-1">Air-gapped on-premise execution fallback, zero external API leakage</p>
            <div className="mt-3 pt-3 border-t border-surface-border/60 flex items-center justify-between text-[11px] text-neutral-400">
              <span>Security status:</span>
              <span className="text-emerald-400 font-medium">100% Local Sandboxed</span>
            </div>
          </div>
        </div>
      </div>

      {/* Controls & Configuration */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Latency Gating */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <Zap className="w-4 h-4 text-amber-400" />
            <span>Circuit Breaker Latency Gate</span>
          </div>
          <p className="text-xs text-neutral-400">
            Automatically bypass primary endpoint if TTFT (time-to-first-token) exceeds this threshold:
          </p>
          <div className="space-y-2">
            <div className="flex justify-between text-xs font-mono text-neutral-300">
              <span>Threshold: {latencyThreshold}ms</span>
              <span className="text-neutral-500">Aggressive &lt;100ms — Relaxed &gt;300ms</span>
            </div>
            <input
              type="range"
              min="50"
              max="500"
              step="25"
              value={latencyThreshold}
              onChange={(e) => setLatencyThreshold(parseInt(e.target.value, 10))}
              className="w-full h-1.5 bg-surface-elevated rounded-lg appearance-none cursor-pointer accent-brand-blue"
            />
          </div>
        </div>

        {/* Retry & Failover Policy */}
        <div className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-white">
            <ShieldCheck className="w-4 h-4 text-emerald-400" />
            <span>Resilience & Recovery Rules</span>
          </div>
          <div className="space-y-3">
            <label className="flex items-center justify-between cursor-pointer">
              <span className="text-xs text-neutral-300">Auto-fallback on HTTP 429 / 5xx</span>
              <input
                type="checkbox"
                checked={autoFallback}
                onChange={(e) => setAutoFallback(e.target.checked)}
                className="rounded border-surface-border bg-surface-elevated text-brand-blue focus:ring-0 w-4 h-4"
              />
            </label>
            <div className="flex items-center justify-between">
              <span className="text-xs text-neutral-300">Max retries before failover</span>
              <select
                value={maxRetries}
                onChange={(e) => setMaxRetries(parseInt(e.target.value, 10))}
                className="bg-surface-elevated border border-surface-border rounded-md px-2 py-1 text-xs text-white"
              >
                <option value={1}>1 Retry</option>
                <option value={2}>2 Retries</option>
                <option value={3}>3 Retries</option>
              </select>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ChainsPage;
