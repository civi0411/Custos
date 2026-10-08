import React from 'react';
import { Plus, Cpu, Laptop, Terminal, Trash2, Zap, RefreshCw } from 'lucide-react';
import { ProviderItem, ClientApiKey } from '@/types';

interface ProvidersViewProps {
  providers: ProviderItem[];
  clientKeys: ClientApiKey[];
  onOpenAddProviderModal: () => void;
  onGenerateClientKey: () => void;
  onRevokeClientKey: (id: string) => void;
  onTestConnection: (providerName: string) => void;
  onDeleteProvider?: (id: string) => void;
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
  onTestConnection,
  onDeleteProvider,
}) => {
  return (
    <main className="flex-1 bg-canvas flex flex-col overflow-y-auto min-w-0 text-fg-editor font-sans">
      {/* Providers Header */}
      <div className="px-4 sm:px-8 pt-6 sm:pt-8 pb-4 border-b border-border-muted flex items-center justify-between min-w-0 bg-surface-1">
        <div className="min-w-0">
          <h1 className="text-lg sm:text-xl font-semibold tracking-[-0.02em] text-fg-editor flex items-center gap-2 truncate">
            <span>Model Providers & Gateway Keys</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-500 font-mono font-normal shrink-0">
              {providers.length} Configured
            </span>
          </h1>
          <p className="text-xs text-fg-muted mt-1 truncate">
            Deterministic inference routing, live model probing, token context windows, and fail-closed credentials
          </p>
        </div>

        <div className="flex items-center gap-3 shrink-0 ml-2">
          <button 
            onClick={onOpenAddProviderModal} 
            className="px-3 sm:px-3.5 py-1.5 rounded-lg workbench-primary-action font-medium text-xs flex items-center gap-1.5 transition shadow-xs cursor-pointer"
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
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle">
              Configured AI Model Providers
            </h3>
            <span className="text-[11px] text-fg-subtle font-mono">
              Live Probe Enabled · SQLite Persisted
            </span>
          </div>
          
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {providers.map((p) => {
              const logoUrl = getProviderLogo(p.iconType);
              const ctxK = p.contextWindow ? `${Math.round(p.contextWindow / 1000)}k ctx` : null;

              return (
                <div 
                  key={p.id} 
                  className="p-4 sm:p-5 rounded-2xl bg-surface-1 border border-border-muted hover:border-border-default transition flex flex-col justify-between space-y-4 shadow-xs"
                >
                  <div className="flex items-center justify-between">
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
                        <h4 className="text-sm font-semibold text-fg-editor truncate">{p.name}</h4>
                        <div className="flex items-center gap-2 mt-0.5">
                          <span className="text-[11px] text-fg-muted font-mono truncate">{p.model}</span>
                          {ctxK && (
                            <span className="text-[10px] px-1.5 py-0.2 rounded bg-surface-2 text-fg-subtle font-mono shrink-0">
                              {ctxK}
                            </span>
                          )}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-1.5 shrink-0">
                      {p.fastMode && (
                        <span className="text-[10px] bg-amber-500/10 text-amber-400 border border-amber-500/20 px-1.5 py-0.5 rounded-full font-mono flex items-center gap-1">
                          <Zap className="w-2.5 h-2.5" /> Fast
                        </span>
                      )}
                      <span className={`text-[10px] border px-2 py-0.5 rounded-full font-mono shrink-0 ${p.badgeColor}`}>
                        {p.statusLabel}
                      </span>
                    </div>
                  </div>

                  <div className="space-y-2 text-xs bg-surface-0 p-3 rounded-xl border border-border-muted">
                    <div className="flex items-center justify-between text-fg-muted">
                      <span>{p.endpoint ? 'Endpoint URL' : 'API Secret'}</span>
                      <span className="font-mono text-fg-editor text-[11px] truncate max-w-[280px]">
                        {p.endpoint || p.apiKey}
                      </span>
                    </div>

                    {p.contextWindow && (
                      <div className="flex items-center justify-between text-fg-muted">
                        <span>Max Context Window</span>
                        <span className="text-fg-editor font-mono text-[11px]">
                          {p.contextWindow.toLocaleString()} tokens
                        </span>
                      </div>
                    )}

                    {p.quotaUsed && p.quotaTotal && (
                      <>
                        <div className="flex items-center justify-between text-fg-muted">
                          <span>Monthly Quota</span>
                          <span className="text-fg-editor font-mono">{p.quotaUsed} / {p.quotaTotal}</span>
                        </div>
                        <div className="w-full bg-surface-2 rounded-full h-1.5 overflow-hidden">
                          <div 
                            className="bg-emerald-500 h-1.5 rounded-full" 
                            style={{ width: `${p.quotaPercent || 0}%` }}
                          />
                        </div>
                      </>
                    )}

                    {p.rateLimit && (
                      <div className="flex items-center justify-between text-fg-muted">
                        <span>Rate Limit</span>
                        <span className="text-fg-editor font-mono">{p.rateLimit}</span>
                      </div>
                    )}

                    {p.vram && (
                      <div className="flex items-center justify-between text-fg-muted">
                        <span>VRAM Footprint</span>
                        <span className="text-fg-editor font-mono">{p.vram}</span>
                      </div>
                    )}
                  </div>

                  <div className="flex items-center justify-between pt-3 border-t border-border-muted text-xs">
                    <div className="flex items-center gap-2">
                      <span className="text-[11px] text-fg-subtle">{p.latency}</span>
                      {onDeleteProvider && (
                        <button
                          onClick={() => {
                            if (window.confirm(`Xóa cấu hình provider "${p.name}"?`)) {
                              onDeleteProvider(p.id);
                            }
                          }}
                          className="text-fg-subtle hover:text-rose-400 p-1 rounded transition cursor-pointer"
                          title="Delete provider"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>

                    <button 
                      onClick={() => onTestConnection(p.id || p.name)} 
                      className="px-2.5 py-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-editor text-[11px] border border-border-default hover:border-border-emphasis transition flex items-center gap-1.5 cursor-pointer"
                    >
                      <RefreshCw className="w-3 h-3 text-fg-muted" />
                      <span>{p.endpoint ? 'Probe & Verify' : 'Test Connection'}</span>
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
            <h3 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle">
              Desktop & CLI Gateway Keys
            </h3>
            <button 
              onClick={onGenerateClientKey} 
              className="text-xs workbench-accent hover:underline flex items-center gap-1 font-medium cursor-pointer"
            >
              <Plus className="w-3 h-3" />
              <span>Generate new key</span>
            </button>
          </div>

          <div className="border border-border-default rounded-xl overflow-x-auto bg-surface-1 shadow-xs">
            <div className="min-w-[500px]">
              <div className="grid grid-cols-12 px-4 py-2.5 border-b border-border-muted text-[11px] font-mono text-fg-subtle bg-surface-2/60">
                <span className="col-span-4">KEY NAME</span>
                <span className="col-span-4">KEY TOKEN</span>
                <span className="col-span-2">CREATED</span>
                <span className="col-span-2 text-right">ACTION</span>
              </div>

              {clientKeys.map((k) => (
                <div 
                  key={k.id} 
                  className="grid grid-cols-12 px-4 py-3 items-center border-b border-border-muted/50 last:border-b-0 text-xs text-fg-editor"
                >
                  <span className="col-span-4 font-medium text-fg-editor flex items-center gap-2 truncate">
                    {k.icon === 'laptop' ? (
                      <Laptop className="w-3.5 h-3.5 workbench-accent shrink-0" />
                    ) : (
                      <Terminal className="w-3.5 h-3.5 text-fg-muted shrink-0" />
                    )}
                    <span className="truncate">{k.name}</span>
                  </span>
                  <span className="col-span-4 font-mono text-[11px] text-fg-muted truncate">{k.token}</span>
                  <span className="col-span-2 text-fg-subtle text-[11px] font-mono">{k.created}</span>
                  <div className="col-span-2 text-right">
                    <button 
                      onClick={() => onRevokeClientKey(k.id)} 
                      className="text-rose-500 hover:text-rose-400 text-[11px] cursor-pointer"
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
