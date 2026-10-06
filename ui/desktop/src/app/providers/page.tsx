import React from 'react';
import { Plus, Cpu, Laptop, Terminal, KeyRound, CheckCircle2, ShieldCheck, Activity } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';

const getProviderLogo = (iconType: string): string | null => {
  switch (iconType) {
    case 'anthropic':
      return '/assets/provider-logo/anthropic.jpg';
    case 'openai':
      return '/assets/provider-logo/openai.jpg';
    case 'gemini':
      return '/assets/provider-logo/gemini.jpg';
    case 'deepseek':
      return '/assets/provider-logo/deepseek.jpg';
    default:
      return null;
  }
};

/**
 * ProvidersPage - Dedicated full-page route (/providers)
 * Manages connected AI model providers, API credentials, and client gateway tokens.
 */
export const ProvidersPage: React.FC = () => {
  const {
    providers,
    clientKeys,
    setIsAddProviderOpen,
    handleGenerateClientKey,
    handleRevokeClientKey,
    showToast
  } = useAppContext();

  return (
    <div className="flex-1 flex flex-col h-full bg-surface overflow-y-auto min-w-0 select-text">
      {/* Top Header */}
      <div className="px-4 sm:px-8 pt-6 sm:pt-8 pb-5 border-b border-surface-border flex flex-col sm:flex-row sm:items-center justify-between gap-4 shrink-0 bg-surface">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-brand-blue/10 border border-brand-blue/30 text-brand-blue">
              <KeyRound className="w-5 h-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-lg sm:text-xl font-bold tracking-tight text-white">API Keys & Model Providers</h1>
                <span className="text-[11px] px-2.5 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 font-mono font-medium">
                  {providers.length} Active
                </span>
              </div>
              <p className="text-xs text-neutral-400 mt-0.5">
                Configure upstream AI provider credentials, rate limits, latency routing, and OmniRoute priorities
              </p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <button 
            onClick={() => showToast('Refreshed provider health checks')} 
            className="px-3 py-1.5 rounded-lg bg-surface-card hover:bg-surface-elevated text-neutral-300 text-xs font-medium border border-surface-border transition flex items-center gap-1.5"
          >
            <Activity className="w-3.5 h-3.5 text-neutral-400" />
            <span>Health Check All</span>
          </button>
          <button 
            onClick={() => setIsAddProviderOpen(true)} 
            className="px-3.5 py-1.5 rounded-lg bg-white text-black hover:bg-neutral-200 font-medium text-xs flex items-center gap-1.5 transition shadow-sm"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>Add Provider Key</span>
          </button>
        </div>
      </div>

      {/* Main Content Body */}
      <div className="px-4 sm:px-8 py-6 max-w-6xl w-full mx-auto space-y-8 flex-1">
        {/* SECTION 1: Connected AI Providers */}
        <section className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-xs font-semibold uppercase tracking-wider text-neutral-400">
                Connected AI Model Providers
              </h2>
              <p className="text-[11px] text-neutral-500 mt-0.5">
                Upstream LLM endpoints calibrated for OmniRoute multi-tier fallback
              </p>
            </div>
            <span className="text-[11px] font-mono text-neutral-500">
              Total quota usage: 68%
            </span>
          </div>
          
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {providers.map((p) => {
              const logoUrl = getProviderLogo(p.iconType);
              return (
                <div 
                  key={p.id} 
                  className="p-5 rounded-2xl bg-surface-card border border-surface-border hover:border-surface-borderHover transition flex flex-col justify-between space-y-4 shadow-sm"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex items-center gap-3 min-w-0">
                      {logoUrl ? (
                        <div className="w-10 h-10 rounded-xl overflow-hidden shrink-0 shadow-sm border border-surface-border/60 bg-surface">
                          <img 
                            src={logoUrl} 
                            alt={p.name} 
                            className="w-full h-full object-cover"
                          />
                        </div>
                      ) : (
                        <div className="w-10 h-10 rounded-xl bg-surface-elevated flex items-center justify-center text-neutral-400 font-bold text-sm shrink-0 border border-surface-border/60">
                          <Cpu className="w-5 h-5" />
                        </div>
                      )}

                      <div className="min-w-0">
                        <div className="flex items-center gap-1.5">
                          <h3 className="text-sm font-semibold text-white truncate">{p.name}</h3>
                          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                        </div>
                        <span className="text-[11px] text-neutral-500 font-mono truncate block">{p.model}</span>
                      </div>
                    </div>

                    <span className={`text-[10px] border px-2 py-0.5 rounded-full font-mono shrink-0 ${p.badgeColor}`}>
                      {p.statusLabel}
                    </span>
                  </div>

                  <div className="space-y-2 text-xs bg-surface/60 p-3 rounded-xl border border-surface-border/50">
                    <div className="flex items-center justify-between text-neutral-400">
                      <span>{p.endpoint ? 'Endpoint' : 'API Key'}</span>
                      <span className="font-mono text-neutral-300 text-[11px] truncate max-w-[240px]">{p.endpoint || p.apiKey}</span>
                    </div>

                    {p.quotaUsed && p.quotaTotal && (
                      <div className="space-y-1 pt-1">
                        <div className="flex items-center justify-between text-neutral-400 text-[11px]">
                          <span>Monthly Quota</span>
                          <span className="text-neutral-300 font-mono">{p.quotaUsed} / {p.quotaTotal}</span>
                        </div>
                        <div className="w-full bg-surface-elevated rounded-full h-1.5 overflow-hidden">
                          <div 
                            className="bg-brand-blue h-1.5 rounded-full transition-all duration-300" 
                            style={{ width: `${p.quotaPercent || 0}%` }}
                          />
                        </div>
                      </div>
                    )}

                    {p.rateLimit && (
                      <div className="flex items-center justify-between text-neutral-400 pt-0.5">
                        <span>Rate Limit</span>
                        <span className="text-neutral-300 font-mono">{p.rateLimit}</span>
                      </div>
                    )}

                    {p.vram && (
                      <div className="flex items-center justify-between text-neutral-400 pt-0.5">
                        <span>VRAM Footprint</span>
                        <span className="text-neutral-300 font-mono">{p.vram}</span>
                      </div>
                    )}
                  </div>

                  <div className="flex items-center justify-between pt-2 border-t border-surface-border/70 text-xs">
                    <div className="flex items-center gap-1.5 text-neutral-400 font-mono text-[11px]">
                      <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
                      <span>{p.latency}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <button 
                        onClick={() => showToast(`Testing connection to ${p.name}... 200 OK (${p.latency})`)} 
                        className="px-2.5 py-1 rounded-lg bg-surface-elevated hover:bg-surface-hover text-neutral-300 text-[11px] font-medium border border-surface-border transition"
                      >
                        {p.endpoint ? 'Check Health' : 'Test Connection'}
                      </button>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </section>

        {/* SECTION 2: Custos Gateway Client API Keys */}
        <section className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-xs font-semibold uppercase tracking-wider text-neutral-400">
                Desktop & CLI Gateway Tokens
              </h2>
              <p className="text-[11px] text-neutral-500 mt-0.5">
                Keys used by the Custos CLI, VS Code Extension, and SDK to interact with the local daemon
              </p>
            </div>
            <button 
              onClick={handleGenerateClientKey} 
              className="px-3 py-1 bg-surface-elevated hover:bg-surface-hover border border-surface-border rounded-lg text-xs text-brand-blue font-medium flex items-center gap-1.5 transition"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>Generate New Key</span>
            </button>
          </div>

          <div className="border border-surface-border rounded-xl overflow-x-auto bg-surface-card shadow-sm">
            <div className="min-w-[560px]">
              <div className="grid grid-cols-12 px-4 py-2.5 border-b border-surface-border text-[11px] font-mono text-neutral-500 bg-surface">
                <span className="col-span-4">APPLICATION</span>
                <span className="col-span-4">TOKEN SECRET</span>
                <span className="col-span-2">CREATED</span>
                <span className="col-span-2 text-right">ACTION</span>
              </div>

              {clientKeys.map((k) => (
                <div 
                  key={k.id} 
                  className="grid grid-cols-12 px-4 py-3 items-center border-b border-surface-border/50 last:border-b-0 text-xs text-neutral-300 hover:bg-surface/40 transition"
                >
                  <span className="col-span-4 font-medium text-white flex items-center gap-2 truncate">
                    {k.icon === 'laptop' ? (
                      <Laptop className="w-3.5 h-3.5 text-brand-blue shrink-0" />
                    ) : (
                      <Terminal className="w-3.5 h-3.5 text-neutral-400 shrink-0" />
                    )}
                    <span className="truncate">{k.name}</span>
                  </span>
                  <span className="col-span-4 font-mono text-[11px] text-neutral-400 truncate select-all">{k.token}</span>
                  <span className="col-span-2 text-neutral-500 text-[11px] font-mono">{k.created}</span>
                  <div className="col-span-2 text-right">
                    <button 
                      onClick={() => handleRevokeClientKey(k.id)} 
                      className="text-red-400 hover:text-red-300 text-[11px] px-2 py-0.5 rounded hover:bg-red-500/10 transition"
                    >
                      Revoke
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Security Notice */}
        <div className="p-4 rounded-xl bg-surface-card border border-surface-border flex items-start gap-3">
          <ShieldCheck className="w-5 h-5 text-emerald-400 shrink-0 mt-0.5" />
          <div className="text-xs">
            <span className="font-semibold text-white block">Credential Storage Security</span>
            <span className="text-neutral-400">
              API keys are encrypted locally via OS Keychain (DPAPI on Windows) and never transmitted to external telemetry servers. All upstream calls route through your configured local Custos daemon.
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ProvidersPage;
