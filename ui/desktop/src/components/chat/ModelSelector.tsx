import React, { useState, useRef, useEffect, useMemo } from 'react';
import {
  ChevronDown,
  Check,
  Search,
  Settings,
  Cpu,
  Ban
} from 'lucide-react';
import {
  ProviderBrandIcon,
  OpenAIIcon,
  ClaudeIcon,
  GeminiIcon,
  DeepSeekIcon
} from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';

export interface ModelOption {
  id: string;
  name: string;
  provider: 'anthropic' | 'openai' | 'gemini' | 'deepseek' | 'local' | 'antigravity' | 'copilot' | 'other';
  providerName: string;
  badge?: string;
  badgeColor?: string;
  description?: string;
  contextWindow?: string;
  isConfigured?: boolean;
}

export function getProviderInfo(modelId: string): {
  provider: ModelOption['provider'];
  label: string;
} {
  const m = (modelId || '').toLowerCase();
  if (m.startsWith('ollama/')) return { provider: 'local', label: 'Ollama' };
  if (m.startsWith('copilot/')) return { provider: 'copilot', label: 'GitHub Copilot' };
  if (m.startsWith('antigravity/')) return { provider: 'antigravity', label: 'Antigravity' };
  if (m.includes('claude') || m.includes('anthropic')) return { provider: 'anthropic', label: 'Anthropic' };
  if (m.includes('gpt') || m.includes('o1') || m.includes('o3') || m.includes('openai')) return { provider: 'openai', label: 'OpenAI' };
  if (m.includes('gemini') || m.includes('google')) return { provider: 'gemini', label: 'Google' };
  if (m.includes('deepseek')) return { provider: 'deepseek', label: 'DeepSeek' };
  if (m.includes('llama') || m.includes('qwen') || m.includes('mistral') || m.includes('local') || m.includes('ollama')) {
    return { provider: 'local', label: 'Local' };
  }
  return { provider: 'other', label: 'Custom' };
}

export function ProviderIcon({
  provider,
  size = 14,
  className
}: {
  provider: ModelOption['provider'];
  size?: number;
  className?: string;
}): React.JSX.Element {
  switch (provider) {
    case 'anthropic':
      return <ClaudeIcon size={size} className={className} />;
    case 'openai':
      return <OpenAIIcon size={size} className={className} />;
    case 'gemini':
      return <GeminiIcon size={size} className={className} />;
    case 'deepseek':
      return <DeepSeekIcon size={size} className={className} />;
    case 'copilot':
      return <ProviderBrandIcon provider="copilot" size={size} className={className} />;
    case 'antigravity':
      return <ProviderBrandIcon provider="antigravity" size={size} className={className} />;
    case 'local':
      return <ProviderBrandIcon provider="local" size={size} className={className || 'text-teal-400'} />;
    default:
      return <Cpu size={size} className={className || 'text-zinc-400'} />;
  }
}

export interface ModelSelectorProps {
  currentModel?: string;
  onSelectModel?: (modelId: string) => void;
  className?: string;
  compact?: boolean;
}

export function getConnectedModelOptions(accounts: import('@/types').ConnectedAccount[]): ModelOption[] {
  const seen = new Set<string>();
  return accounts.filter((a) => a.status === 'active' || a.providerId === 'local').flatMap((account) =>
    account.supportedModels.filter((id) => {
      if (seen.has(id.toLowerCase())) return false;
      seen.add(id.toLowerCase()); return true;
    }).map((id) => {
      const metadata = account.catalogModels?.find((m) => m.id === id);
      return {
        id, name: metadata?.label || id, provider: account.providerId as ModelOption['provider'],
        providerName: account.providerName, badge: account.status === 'offline' ? 'Offline' : account.catalogSource === 'cached' ? 'Cached' : 'Live',
        description: metadata?.description || `Account: ${account.accountName}`,
        contextWindow: metadata?.context_window ? `${Math.round(metadata.context_window / 1000)}k tokens` : undefined,
        isConfigured: account.status === 'active',
      };
    })
  );
}

export const ModelSelector: React.FC<ModelSelectorProps> = ({
  currentModel,
  onSelectModel,
  className = ''
}) => {
  const {
    openSettings,
    connectedAccounts,
    refreshModels,
    selectedModel: contextSelectedModel,
    handleSelectModel: contextHandleSelectModel
  } = useAppContext();

  // If currentModel is provided, use it; otherwise contextSelectedModel; default to empty string
  const activeModelId = currentModel !== undefined ? currentModel : (contextSelectedModel || '');
  const handleSelect = onSelectModel || contextHandleSelectModel;

  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const popoverRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Close when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeyDown);
      setTimeout(() => searchInputRef.current?.focus(), 50);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  const hasModel = Boolean(
    activeModelId &&
    activeModelId.trim().length > 0 &&
    activeModelId !== 'No model selected' &&
    activeModelId !== 'Model not reported'
  );

  const allModels = useMemo(() => getConnectedModelOptions(connectedAccounts), [connectedAccounts]);

  // Find active model details.
  const activeModel = useMemo(() => {
    if (!hasModel) {
      return {
        id: '',
        name: 'No model selected',
        provider: 'other' as const,
        providerName: 'Unassigned',
        badge: 'Select'
      };
    }
    const found = allModels.find((m) => m.id.toLowerCase() === activeModelId.toLowerCase());
    if (found) return found;

    const info = getProviderInfo(activeModelId);
    return {
      id: activeModelId,
      name: activeModelId,
      provider: info.provider,
      providerName: info.label,
      badge: activeModelId.startsWith('ollama/') ? 'Unavailable' : 'Active',
      isConfigured: !activeModelId.startsWith('ollama/')
    };
  }, [allModels, activeModelId, hasModel]);

  // Filtered models
  const filteredModels = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return allModels;

    return allModels.filter(
      (m) =>
        m.name.toLowerCase().includes(q) ||
        m.id.toLowerCase().includes(q) ||
        m.providerName.toLowerCase().includes(q) ||
        (m.description && m.description.toLowerCase().includes(q)) ||
        (m.badge && m.badge.toLowerCase().includes(q))
    );
  }, [allModels, searchQuery]);

  return (
    <div className={`relative inline-block text-left ${className}`} ref={popoverRef}>
      {/* Trigger Button inside Composer */}
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        className={`flex items-center gap-1.5 px-2 py-1 rounded-md text-[11px] font-medium transition select-none ${
          !hasModel
            ? 'bg-amber-500/10 hover:bg-amber-500/20 text-amber-400 border border-amber-500/30'
            : isOpen
            ? 'bg-surface-2 text-fg-editor border border-border-default shadow-xs'
            : 'bg-surface-1/80 hover:bg-surface-2 text-fg-muted hover:text-fg-editor border border-border-muted hover:border-border-default'
        }`}
        title={
          !hasModel
            ? 'No model selected. Click to select a model or combo.'
            : `Current Model: ${activeModel.id}. Click to change model.`
        }
      >
        {!hasModel ? (
          <Cpu size={13} className="shrink-0 text-amber-400" />
        ) : (
          <ProviderIcon provider={activeModel.provider} size={13} className="shrink-0" />
        )}

        <span className="font-mono text-[11px] truncate max-w-[140px] text-fg-editor">
          {!hasModel ? 'No model selected' : activeModel.id}
        </span>
        {hasModel && activeModel.isConfigured === false && <span className="shrink-0 text-[10px] text-fg-muted">Offline</span>}

        <ChevronDown
          size={12}
          className={`shrink-0 text-fg-subtle transition-transform duration-150 ${
            isOpen ? 'rotate-180 text-fg-editor' : ''
          }`}
        />
      </button>

      {/* Floating Popover (Opens Upward Above Composer) */}
      {isOpen && (
        <div
          className="app-dropdown-menu absolute bottom-full mb-2 left-0 z-50 w-72 md:w-72 rounded-xl border border-border-default bg-surface-1/95 backdrop-blur-xl shadow-2xl overflow-hidden animate-in fade-in slide-in-from-bottom-2 duration-150"
          style={{ maxHeight: '420px' }}
        >
      {connectedAccounts.some((a) => a.catalogError && a.providerId !== 'local') && (
        <div className="px-3 py-2 text-xs text-amber-400 break-words border-b border-border-muted" role="status">
          {connectedAccounts.filter((a) => a.catalogError && a.providerId !== 'local').map((a) => (
            <div key={a.id}>{a.providerName}: {a.catalogSource === 'cached' ? `Cached (${a.catalogFetchedAt ? new Date(a.catalogFetchedAt).toLocaleString() : 'unknown time'})` : 'Models unavailable'}: {a.catalogError}</div>
          ))}
          <button type="button" onClick={() => void refreshModels()}>Retry models</button>
        </div>
      )}
          {/* Header & Search */}
          <div className="p-2.5 border-b border-border-muted bg-surface-0/60">
            <div className="flex items-center justify-between mb-2 px-1">
              <span className="text-[11px] font-semibold text-fg-editor flex items-center gap-1.5">
                Select Model
              </span>
              <span className="text-[10px] text-fg-subtle">
                {allModels.length} models
              </span>
            </div>
            <div className="relative">
              <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-subtle" />
              <input
                ref={searchInputRef}
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search model ids..."
                className="w-full bg-surface-1 border border-border-muted focus:border-border-default focus:ring-1 focus:ring-border-default/20 rounded-lg pl-8 pr-3 py-1.5 text-xs text-fg-editor placeholder-fg-subtle outline-none transition"
              />
            </div>
          </div>

          {/* Model Options List */}
          <div className="overflow-y-auto p-1.5 divide-y divide-border-muted/50" style={{ maxHeight: '300px' }}>
            {/* Clear / Unselect Model Option */}
            <div className="py-1 first:pt-0">
              <button
                type="button"
                onClick={() => {
                  handleSelect('');
                  setIsOpen(false);
                }}
                className={`w-full text-left px-2.5 py-1.5 rounded-lg flex items-center justify-between transition group ${
                  !hasModel
                    ? 'bg-amber-500/10 text-amber-300 border border-amber-500/30 shadow-xs'
                    : 'hover:bg-surface-2/50 text-fg-muted hover:text-fg-editor'
                }`}
              >
                <div className="flex items-center gap-2">
                  <Ban size={14} className="text-zinc-400 shrink-0" />
                  <div>
                    <div className="text-xs font-medium text-fg-editor">No model selected</div>
                    <div className="text-[10px] text-fg-subtle">Unset active model; prompt kernel with unassigned model</div>
                  </div>
                </div>
                {!hasModel && <Check size={13} className="text-amber-400 stroke-[2.5]" />}
              </button>
            </div>

            {/* Custom Model Option when search query is entered and not an exact match */}
            {searchQuery.trim().length > 0 &&
              !allModels.some((m) => m.id.toLowerCase() === searchQuery.trim().toLowerCase()) && (
                <div className="py-1">
                  <button
                    type="button"
                    onClick={() => {
                      handleSelect(searchQuery.trim());
                      setIsOpen(false);
                    }}
                    className="w-full text-left px-2.5 py-1.5 rounded-lg flex items-center justify-between transition bg-surface-2 hover:bg-surface-3 text-fg-editor border border-border-muted hover:border-border-default group"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <Cpu size={14} className="text-sky-400 shrink-0" />
                      <div className="min-w-0 truncate">
                        <div className="text-xs font-medium text-fg-editor truncate">
                          Use custom model: <span className="font-mono text-sky-400 font-bold">{searchQuery.trim()}</span>
                        </div>
                        <div className="text-[10px] text-fg-subtle truncate">
                          Send prompt directly to provider with this model ID
                        </div>
                      </div>
                    </div>
                  </button>
                </div>
              )}

            {filteredModels.length === 0 ? (
              <div className="p-4 text-center text-fg-subtle text-xs">
                No configured models matching "{searchQuery}"
              </div>
            ) : (
              <div className="py-1 first:pt-0 last:pb-0">
                <div className="space-y-0.5">
                  {filteredModels.map((m) => {
                    const isSelected = m.id.toLowerCase() === activeModelId.toLowerCase();
                    return (
                      <button
                        key={m.id}
                        type="button"
                        disabled={!m.isConfigured}
                        title={!m.isConfigured ? `${m.id} Offline` : m.name}
                        onClick={() => {
                          handleSelect(m.id);
                          setIsOpen(false);
                        }}
                        className={`w-full text-left px-2.5 py-1.5 rounded-md flex items-center justify-between transition ${
                          isSelected
                            ? 'bg-surface-2 text-fg-editor border border-border-default'
                            : 'hover:bg-surface-2/50 text-fg-muted hover:text-fg-editor'
                        }`}
                      >
                        <span className="font-mono text-xs truncate pr-2">{m.id}</span>
                        {!m.isConfigured && <span className="ml-auto mr-2 shrink-0 text-[10px] text-fg-muted">Offline</span>}
                        {isSelected && <Check size={13} className="text-emerald-400 stroke-[2.5] shrink-0" />}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </div>

          {/* Footer: Jump to Settings */}
          <div className="p-2 border-t border-border-muted bg-surface-0/60 flex items-center justify-between">
            <span className="text-[10px] ml-2 text-fg-subtle">{filteredModels.length} shown</span>
            <button
              type="button"
              onClick={() => {
                setIsOpen(false);
                openSettings('providers');
              }}
              className="text-[10px] text-fg-muted hover:text-fg-editor flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-surface-2 transition"
            >
              <Settings size={11} />
              Manage Providers...
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default ModelSelector;
