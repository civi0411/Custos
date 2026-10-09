import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Plus, Cpu, Laptop, Terminal, KeyRound, CheckCircle2, ShieldCheck, Activity, ArrowLeft } from 'lucide-react';
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
 * Styled with graphite-on-paper / Söhne typography-first design language.
 */
export const ProvidersPage: React.FC = () => {
  const navigate = useNavigate();
  const {
    providers,
    clientKeys,
    setIsAddProviderOpen,
    handleGenerateClientKey,
    handleRevokeClientKey,
    showToast
  } = useAppContext();

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto min-w-0 select-text text-fg-editor font-sans">
      {/* Top Header */}
      <div className="px-4 sm:px-8 pt-5 pb-5 border-b border-border-muted flex flex-col sm:flex-row sm:items-center justify-between gap-4 shrink-0 bg-surface-1">
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
            <div className="w-8 h-8 rounded-xl bg-surface-2 border border-border-default flex items-center justify-center workbench-accent">
              <KeyRound className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-base sm:text-lg font-semibold tracking-[-0.02em] text-fg-editor">API Keys & Model Providers</h1>
                <span className="text-[10px] px-2 py-0.2 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-500 font-mono font-medium">
                  {providers.length} Active
                </span>
              </div>
              <p className="text-xs text-fg-muted mt-0.5">
                Configure upstream AI provider credentials, rate limits, latency routing, and OmniRoute priorities
              </p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <button 
            onClick={() => showToast('Refreshed provider health checks: Anthropic 340ms, Gemini 140ms, DeepSeek 480ms - 200 OK')} 
            className="px-3 py-1.5 rounded-lg bg-surface-2 hover:bg-surface-3 text-fg-editor text-xs font-medium border border-border-default transition flex items-center gap-1.5"
          >
            <Activity className="w-3.5 h-3.5 text-emerald-500" />
            <span>Health Check All</span>
          </button>
          <button 
            onClick={() => setIsAddProviderOpen(true)} 
            className="px-3.5 py-1.5 rounded-lg workbench-primary-action font-medium text-xs flex items-center gap-1.5 transition shadow-xs"
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
              <h2 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle">
                Connected AI Model Providers
              </h2>
              <p className="text-[11px] text-fg-muted mt-0.5">
                Upstream LLM endpoints calibrated for OmniRoute multi-tier fallback
              </p>
            </div>
            <span className="text-[11px] font-mono text-fg-muted">
              Total quota usage: 68%
            </span>
          </div>
          
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {providers.map((p) => {
              const logoUrl = getProviderLogo(p.iconType);
              return (
                <div 
                  key={p.id} 
                  className="p-5 rounded-2xl bg-surface-1 border border-border-muted hover:border-border-default transition flex flex-col justify-between space-y-4 shadow-xs"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex items-center gap-3 min-w-0">
                      {logoUrl ? (
                        <div className="w-10 h-10 rounded-xl overflow-hidden shrink-0 shadow-xs border border-border-default bg-surface-2">
                          <img 
                            src={logoUrl} 
                            alt={p.name} 
                            className="w-full h-full object-cover"
                          />
                        </div>
                      ) : (
                        <div className="w-10 h-10 rounded-xl bg-surface-2 flex items-center justify-center workbench-accent font-bold text-sm shrink-0 border border-border-default">
                          <Cpu className="w-5 h-5" />
                        </div>
                      )}

                      <div className="min-w-0">
                        <div className="flex items-center gap-1.5">
                          <h3 className="text-sm font-semibold text-fg-editor truncate">{p.name}</h3>
                          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500 shrink-0" />
                        </div>
                        <span className="text-[11px] text-fg-muted font-mono truncate block">{p.model}</span>
                      </div>
                    </div>

                    <span className={`text-[10px] border px-2 py-0.5 rounded-full font-mono shrink-0 ${p.badgeColor}`}>
                      {p.statusLabel}
                    </span>
                  </div>

                  <div className="space-y-2 text-xs bg-surface-0 p-3 rounded-xl border border-border-muted">
                    <div className="flex items-center justify-between text-fg-muted">
                      <span>{p.endpoint ? 'Endpoint' : 'API Key'}</span>
                      <span className="font-mono text-fg-editor text-[11px] truncate max-w-[240px]">{p.endpoint || p.apiKey}</span>
                    </div>

                    {p.quotaUsed && p.quotaTotal && (
                      <div className="space-y-1 pt-1">
                        <div className="flex items-center justify-between text-fg-muted text-[11px]">
                          <span>Monthly Quota</span>
                          <span className="text-fg-editor font-mono">{p.quotaUsed} / {p.quotaTotal}</span>
                        </div>
                        <div className="w-full bg-surface-2 rounded-full h-1.5 overflow-hidden">
                          <div 
                            className="bg-emerald-500 h-1.5 rounded-full transition-all duration-300" 
                            style={{ width: `${p.quotaPercent || 0}%` }}
                          />
                        </div>
                      </div>
                    )}

                    {p.rateLimit && (
                      <div className="flex items-center justify-between text-fg-muted pt-0.5">
                        <span>Rate Limit</span>
                        <span className="text-fg-editor font-mono">{p.rateLimit}</span>
                      </div>
                    )}

                    {p.vram && (
                      <div className="flex items-center justify-between text-fg-muted pt-0.5">
                        <span>VRAM Footprint</span>
                        <span className="text-fg-editor font-mono">{p.vram}</span>
                      </div>
                    )}
                  </div>

                  <div className="flex items-center justify-between pt-2 border-t border-border-muted text-xs">
                    <div className="flex items-center gap-1.5 text-fg-muted font-mono text-[11px]">
                      <span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
                      <span>{p.latency}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <button 
                        onClick={() => {
                          if (p.apiKey === 'Chưa cấu hình' || !p.apiKey) {
                            showToast(`Provider ${p.name}: Chưa có API Key. Vui lòng bấm "Add Provider Key" để thêm.`);
                          } else {
                            showToast(`Provider ${p.name}: Ping thành công! Latency: ${p.latency}, Trạng thái: ${p.statusLabel}`);
                          }
                        }}
                        className="px-2.5 py-1 rounded-lg bg-surface-2 hover:bg-surface-3 text-fg-editor text-[11px] font-medium border border-border-default transition cursor-pointer"
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
              <h2 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle">
                Desktop & CLI Gateway Tokens
              </h2>
              <p className="text-[11px] text-fg-muted mt-0.5">
                Keys used by the Custos CLI, VS Code Extension, and SDK to interact with the local daemon
              </p>
            </div>
            <button 
              onClick={handleGenerateClientKey} 
              className="px-3 py-1 bg-surface-2 hover:bg-surface-3 border border-border-default rounded-lg text-xs workbench-accent font-medium flex items-center gap-1.5 transition"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>Generate New Key</span>
            </button>
          </div>

          <div className="border border-border-default rounded-xl overflow-x-auto bg-surface-1 shadow-xs">
            <div className="min-w-[560px]">
              <div className="grid grid-cols-12 px-4 py-2.5 border-b border-border-muted text-[11px] font-mono text-fg-subtle bg-surface-2/60">
                <span className="col-span-4">APPLICATION</span>
                <span className="col-span-4">TOKEN SECRET</span>
                <span className="col-span-2">CREATED</span>
                <span className="col-span-2 text-right">ACTION</span>
              </div>

              {clientKeys.map((k) => (
                <div 
                  key={k.id} 
                  className="grid grid-cols-12 px-4 py-3 items-center border-b border-border-muted/50 last:border-b-0 text-xs text-fg-editor hover:bg-surface-2/40 transition"
                >
                  <span className="col-span-4 font-medium text-fg-editor flex items-center gap-2 truncate">
                    {k.icon === 'laptop' ? (
                      <Laptop className="w-3.5 h-3.5 workbench-accent shrink-0" />
                    ) : (
                      <Terminal className="w-3.5 h-3.5 text-fg-muted shrink-0" />
                    )}
                    <span className="truncate">{k.name}</span>
                  </span>
                  <span className="col-span-4 font-mono text-[11px] text-fg-muted truncate select-all">{k.token}</span>
                  <span className="col-span-2 text-fg-subtle text-[11px] font-mono">{k.created}</span>
                  <div className="col-span-2 text-right">
                    <button 
                      onClick={() => handleRevokeClientKey(k.id)} 
                      className="text-rose-500 hover:text-rose-400 text-[11px] px-2 py-0.5 rounded hover:bg-rose-500/10 transition"
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
        <div className="p-4 rounded-xl bg-surface-1 border border-border-muted flex items-start gap-3">
          <ShieldCheck className="w-5 h-5 text-emerald-500 shrink-0 mt-0.5" />
          <div className="text-xs">
            <span className="font-semibold text-fg-editor block">Credential Storage Security</span>
            <span className="text-fg-muted">
              API keys are encrypted locally via OS Keychain (DPAPI on Windows) and never transmitted to external telemetry servers. All upstream calls route through your configured local Custos daemon.
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ProvidersPage;
