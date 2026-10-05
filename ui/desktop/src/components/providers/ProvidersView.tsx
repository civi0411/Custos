import React from 'react';
import { Plus, Cpu, Laptop, Terminal } from 'lucide-react';
import { ProviderItem, ClientApiKey } from '@/types';

interface ProvidersViewProps {
  providers: ProviderItem[];
  clientKeys: ClientApiKey[];
  onOpenAddProviderModal: () => void;
  onGenerateClientKey: () => void;
  onRevokeClientKey: (id: string) => void;
  onTestConnection: (providerName: string) => void;
}

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

export const ProvidersView: React.FC<ProvidersViewProps> = ({
  providers,
  clientKeys,
  onOpenAddProviderModal,
  onGenerateClientKey,
  onRevokeClientKey,
  onTestConnection
}) => {
  return (
    <main className="flex-1 bg-surface flex flex-col overflow-y-auto min-w-0">
      {/* Providers Header */}
      <div className="px-4 sm:px-8 pt-6 sm:pt-8 pb-4 border-b border-surface-border flex items-center justify-between min-w-0">
        <div className="min-w-0">
          <h1 className="text-lg sm:text-xl font-bold tracking-tight text-white flex items-center gap-2 truncate">
            <span>API Keys & Model Providers</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 font-mono font-normal shrink-0">
              {providers.length} Active
            </span>
          </h1>
          <p className="text-xs text-neutral-500 mt-1 truncate">
            Configure credentials, rate limits, and fallback weights for OmniRoute
          </p>
        </div>

        <div className="flex items-center gap-3 shrink-0 ml-2">
          <button 
            onClick={onOpenAddProviderModal} 
            className="px-3 sm:px-3.5 py-1.5 rounded-lg bg-white text-black hover:bg-neutral-200 font-medium text-xs flex items-center gap-1.5 transition"
          >
            <Plus className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Add Provider Key</span>
            <span className="sm:hidden">Add</span>
          </button>
        </div>
      </div>

      {/* Providers Cards Grid */}
      <div className="px-4 sm:px-8 py-6 sm:py-8 max-w-6xl w-full mx-auto space-y-8 min-w-0">
        
        {/* SECTION 1: Active AI Providers */}
        <div className="space-y-4 min-w-0">
          <h3 className="text-xs font-semibold uppercase tracking-wider text-neutral-500">
            Connected AI Model Providers
          </h3>
          
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {providers.map((p) => {
              const logoUrl = getProviderLogo(p.iconType);
              return (
                <div 
                  key={p.id} 
                  className="p-4 sm:p-5 rounded-2xl bg-surface-card border border-surface-border hover:border-surface-borderHover transition flex flex-col justify-between space-y-4"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      {logoUrl ? (
                        <div className="w-10 h-10 rounded-xl overflow-hidden shrink-0 shadow-sm">
                          <img 
                            src={logoUrl} 
                            alt={p.name} 
                            className="w-full h-full object-cover"
                          />
                        </div>
                      ) : (
                        <div className="w-10 h-10 rounded-xl bg-surface-elevated flex items-center justify-center text-neutral-400 font-bold text-sm shrink-0">
                          <Cpu className="w-5 h-5" />
                        </div>
                      )}

                      <div>
                        <h4 className="text-sm font-semibold text-white">{p.name}</h4>
                        <span className="text-[11px] text-neutral-500 font-mono">{p.model}</span>
                      </div>
                    </div>

                    <span className={`text-[10px] border px-2 py-0.5 rounded-full font-mono shrink-0 ${p.badgeColor}`}>
                      {p.statusLabel}
                    </span>
                  </div>

                  <div className="space-y-2 text-xs">
                    <div className="flex items-center justify-between text-neutral-400">
                      <span>{p.endpoint ? 'Endpoint' : 'API Key'}</span>
                      <span className="font-mono text-neutral-300">{p.endpoint || p.apiKey}</span>
                    </div>

                    {p.quotaUsed && p.quotaTotal && (
                      <>
                        <div className="flex items-center justify-between text-neutral-400">
                          <span>Monthly Quota</span>
                          <span className="text-neutral-300 font-mono">{p.quotaUsed} / {p.quotaTotal}</span>
                        </div>
                        <div className="w-full bg-neutral-800 rounded-full h-1.5 overflow-hidden">
                          <div 
                            className="bg-brand-blue h-1.5 rounded-full" 
                            style={{ width: `${p.quotaPercent || 0}%` }}
                          />
                        </div>
                      </>
                    )}

                    {p.rateLimit && (
                      <div className="flex items-center justify-between text-neutral-400">
                        <span>Rate Limit</span>
                        <span className="text-neutral-300 font-mono">{p.rateLimit}</span>
                      </div>
                    )}

                    {p.vram && (
                      <div className="flex items-center justify-between text-neutral-400">
                        <span>VRAM Footprint</span>
                        <span className="text-neutral-300 font-mono">{p.vram}</span>
                      </div>
                    )}
                  </div>

                  <div className="flex items-center justify-between pt-3 border-t border-surface-border/70 text-xs">
                    <span className="text-[11px] text-neutral-500">{p.latency}</span>
                    <button 
                      onClick={() => onTestConnection(p.name)} 
                      className="px-2.5 py-1 rounded bg-surface-elevated hover:bg-surface-hover text-neutral-300 text-[11px] border border-surface-border transition"
                    >
                      {p.endpoint ? 'Check Health' : 'Test Connection'}
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* SECTION 2: Custos Gateway Client API Keys */}
        <div className="space-y-4 min-w-0">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-semibold uppercase tracking-wider text-neutral-500">
              Desktop & CLI Access Keys
            </h3>
            <button 
              onClick={onGenerateClientKey} 
              className="text-xs text-brand-blue hover:underline flex items-center gap-1"
            >
              <Plus className="w-3 h-3" />
              <span>Generate new key</span>
            </button>
          </div>

          <div className="border border-surface-border rounded-xl overflow-x-auto bg-surface-card">
            <div className="min-w-[500px]">
              <div className="grid grid-cols-12 px-4 py-2.5 border-b border-surface-border text-[11px] font-mono text-neutral-500 bg-surface">
                <span className="col-span-4">KEY NAME</span>
                <span className="col-span-4">KEY TOKEN</span>
                <span className="col-span-2">CREATED</span>
                <span className="col-span-2 text-right">ACTION</span>
              </div>

              {clientKeys.map((k) => (
                <div 
                  key={k.id} 
                  className="grid grid-cols-12 px-4 py-3 items-center border-b border-surface-border/50 last:border-b-0 text-xs text-neutral-300"
                >
                  <span className="col-span-4 font-medium text-white flex items-center gap-2 truncate">
                    {k.icon === 'laptop' ? (
                      <Laptop className="w-3.5 h-3.5 text-neutral-500 shrink-0" />
                    ) : (
                      <Terminal className="w-3.5 h-3.5 text-neutral-500 shrink-0" />
                    )}
                    <span className="truncate">{k.name}</span>
                  </span>
                  <span className="col-span-4 font-mono text-[11px] text-neutral-400 truncate">{k.token}</span>
                  <span className="col-span-2 text-neutral-500 text-[11px] font-mono">{k.created}</span>
                  <div className="col-span-2 text-right">
                    <button 
                      onClick={() => onRevokeClientKey(k.id)} 
                      className="text-red-400 hover:text-red-300 text-[11px]"
                    >
                      Revoke
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

      </div>
    </main>
  );
};
