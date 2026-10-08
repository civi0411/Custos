import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { GitFork, ShieldCheck, Zap, Activity, RefreshCw, ArrowLeft } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';
import { CustomSelect } from '../../components/CustomSelect';

export const ChainsPage: React.FC = () => {
  const navigate = useNavigate();
  const { showToast } = useAppContext();
  const [latencyThreshold, setLatencyThreshold] = useState(150);
  const [maxRetries, setMaxRetries] = useState(2);
  const [autoFallback, setAutoFallback] = useState(true);

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto p-4 sm:p-6 lg:p-8 space-y-6 text-fg-editor font-sans">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-border-muted">
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate('/studio')}
            className="p-1.5 rounded-lg bg-surface-2 hover:bg-surface-3 border border-border-default text-fg-muted hover:text-fg-editor transition flex items-center gap-1.5 text-xs"
          >
            <ArrowLeft className="w-3.5 h-3.5" />
            <span>Studio</span>
          </button>
          <div className="h-4 w-[1px] bg-border-muted" />
          <div className="flex items-center gap-2.5">
            <div className="w-9 h-9 rounded-xl bg-surface-2 border border-border-default flex items-center justify-center workbench-accent">
              <GitFork className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-base sm:text-lg font-semibold tracking-[-0.02em] text-fg-editor">OmniRoute Chains</h1>
              <p className="text-xs text-fg-muted mt-0.5">Intelligent multi-model routing, latency circuit breakers, and zero-downtime failover</p>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => showToast('Routing topology refreshed')}
            className="px-3 py-1.5 bg-surface-2 hover:bg-surface-3 border border-border-default rounded-lg text-xs font-medium text-fg-editor transition flex items-center gap-1.5"
          >
            <RefreshCw className="w-3.5 h-3.5 text-fg-muted" />
            <span>Test Chain</span>
          </button>
          <span className="px-2.5 py-1 rounded-md bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 text-xs font-medium flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
            Active Policy
          </span>
        </div>
      </div>

      {/* Visual Pipeline Graph */}
      <div className="p-5 bg-surface-1 border border-border-default rounded-xl shadow-xs">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle mb-4 flex items-center gap-2">
          <Activity className="w-4 h-4 workbench-accent" />
          Live Route Resolution Pipeline
        </h2>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 relative">
          {/* Step 1: Primary Model */}
          <div className="p-4 rounded-xl bg-surface-0 border border-border-muted hover:border-border-default relative overflow-hidden transition shadow-xs flex flex-col justify-between space-y-3">
            <div>
              <div className="flex items-center justify-between mb-2">
                <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-surface-2 text-fg-editor border border-border-default">
                  Tier 1 • Primary
                </span>
                <span className="text-xs font-mono text-emerald-500 font-medium">42ms avg</span>
              </div>
              <div className="flex items-center gap-2 mb-1">
                <img src="/assets/provider-logo/anthropic.jpg" alt="Anthropic" className="w-5 h-5 rounded object-contain border border-border-default shadow-xs" />
                <h3 className="text-sm font-semibold text-fg-editor">Claude 3.7 Sonnet</h3>
              </div>
              <p className="text-xs text-fg-muted mt-1">High-reasoning code synthesis & complex architectural refactoring</p>
            </div>
            <div className="pt-3 border-t border-border-muted flex items-center justify-between text-[11px] text-fg-subtle">
              <span>Failure trigger:</span>
              <span className="text-fg-editor font-mono">&gt;{latencyThreshold}ms or 5xx</span>
            </div>
          </div>

          {/* Step 2: Failover Route */}
          <div className="p-4 rounded-xl bg-surface-0 border border-border-muted hover:border-border-default relative overflow-hidden transition shadow-xs flex flex-col justify-between space-y-3">
            <div>
              <div className="flex items-center justify-between mb-2">
                <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-surface-2 text-fg-editor border border-border-default">
                  Tier 2 • Secondary
                </span>
                <span className="text-xs font-mono text-fg-muted">110ms avg</span>
              </div>
              <div className="flex items-center gap-2 mb-1">
                <img src="/assets/provider-logo/openai.jpg" alt="OpenAI" className="w-5 h-5 rounded object-contain border border-border-default shadow-xs" />
                <h3 className="text-sm font-semibold text-fg-editor">OpenAI GPT-4o</h3>
              </div>
              <p className="text-xs text-fg-muted mt-1">Fast code generation failover route when primary latency spikes</p>
            </div>
            <div className="pt-3 border-t border-border-muted flex items-center justify-between text-[11px] text-fg-subtle">
              <span>Failure trigger:</span>
              <span className="text-fg-editor font-mono">Rate-limit / Timeout</span>
            </div>
          </div>

          {/* Step 3: Local Offline Fallback */}
          <div className="p-4 rounded-xl bg-surface-0 border border-border-muted hover:border-border-default relative overflow-hidden transition shadow-xs flex flex-col justify-between space-y-3">
            <div>
              <div className="flex items-center justify-between mb-2">
                <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-surface-2 text-fg-editor border border-border-default">
                  Tier 3 • Offline Local
                </span>
                <span className="text-xs font-mono text-emerald-500 font-medium">0ms network</span>
              </div>
              <div className="flex items-center gap-2 mb-1">
                <img src="/assets/provider-logo/deepseek.jpg" alt="DeepSeek" className="w-5 h-5 rounded object-contain border border-border-default shadow-xs" />
                <h3 className="text-sm font-semibold text-fg-editor">DeepSeek R1 (Ollama / Local)</h3>
              </div>
              <p className="text-xs text-fg-muted mt-1">Air-gapped on-premise execution fallback, zero external API leakage</p>
            </div>
            <div className="pt-3 border-t border-border-muted flex items-center justify-between text-[11px] text-fg-subtle">
              <span>Security status:</span>
              <span className="text-emerald-500 font-medium">100% Local Sandboxed</span>
            </div>
          </div>
        </div>
      </div>

      {/* Controls & Configuration */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Latency Gating */}
        <div className="p-5 bg-surface-1 border border-border-default rounded-xl space-y-4 shadow-xs">
          <div className="flex items-center gap-2 text-sm font-semibold text-fg-editor">
            <Zap className="w-4 h-4 text-amber-500" />
            <span>Circuit Breaker Latency Gate</span>
          </div>
          <p className="text-xs text-fg-muted">
            Automatically bypass primary endpoint if TTFT (time-to-first-token) exceeds this threshold:
          </p>
          <div className="space-y-2">
            <div className="flex justify-between text-xs font-mono text-fg-editor">
              <span>Threshold: {latencyThreshold}ms</span>
              <span className="text-fg-subtle">Aggressive &lt;100ms — Relaxed &gt;300ms</span>
            </div>
            <input
              type="range"
              min="50"
              max="500"
              step="25"
              value={latencyThreshold}
              onChange={(e) => setLatencyThreshold(parseInt(e.target.value, 10))}
              className="w-full h-1.5 bg-surface-2 rounded-lg appearance-none cursor-pointer accent-[var(--workbench-accent)]"
            />
          </div>
        </div>

        {/* Retry & Failover Policy */}
        <div className="p-5 bg-surface-1 border border-border-default rounded-xl space-y-4 shadow-xs">
          <div className="flex items-center gap-2 text-sm font-semibold text-fg-editor">
            <ShieldCheck className="w-4 h-4 text-emerald-500" />
            <span>Resilience & Recovery Rules</span>
          </div>
          <div className="space-y-3">
            <label className="flex items-center justify-between p-3 rounded-lg bg-surface-0 border border-border-muted hover:border-border-default cursor-pointer transition select-none">
              <span className="text-xs text-fg-editor">Auto-fallback on HTTP 429 / 5xx</span>
              <input
                type="checkbox"
                checked={autoFallback}
                onChange={(e) => setAutoFallback(e.target.checked)}
                className="accent-[var(--workbench-accent)]"
              />
            </label>
            <div className="flex items-center justify-between">
              <span className="text-xs text-fg-editor">Max retries before failover</span>
              <div className="w-32 shrink-0">
                <CustomSelect
                  value={maxRetries}
                  onChange={(val) => setMaxRetries(Number(val))}
                  options={[
                    { value: 1, label: '1 Retry' },
                    { value: 2, label: '2 Retries' },
                    { value: 3, label: '3 Retries' },
                  ]}
                  headerTitle="Failover Retries"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ChainsPage;
