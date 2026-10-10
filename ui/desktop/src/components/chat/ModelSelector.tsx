import React, { useState, useRef, useEffect, useMemo } from 'react';
import {
  ChevronDown,
  Check,
  Search,
  Settings,
  Server,
  Cpu,
  Ban
} from 'lucide-react';
import {
  OpenAIIcon,
  ClaudeIcon,
  GeminiIcon,
  DeepSeekIcon
} from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';

export interface ModelOption {
  id: string;
  name: string;
  provider: 'anthropic' | 'openai' | 'gemini' | 'deepseek' | 'local' | 'other';
  providerName: string;
  badge?: string;
  badgeColor?: string;
  description?: string;
  contextWindow?: string;
  isConfigured?: boolean;
}

export const CANONICAL_MODELS: ModelOption[] = [
  // Anthropic
  {
    id: 'claude-3-7-sonnet',
    name: 'Claude 3.7 Sonnet',
    provider: 'anthropic',
    providerName: 'Anthropic',
    badge: 'Hybrid Thinking',
    badgeColor: 'text-amber-400 bg-amber-500/10 border-amber-500/20',
    description: 'State-of-the-art reasoning & agentic coding',
    contextWindow: '200k tokens'
  },
  {
    id: 'claude-3-5-sonnet',
    name: 'Claude 3.5 Sonnet',
    provider: 'anthropic',
    providerName: 'Anthropic',
    badge: 'Standard',
    badgeColor: 'text-amber-400 bg-amber-500/10 border-amber-500/20',
    description: 'Industry benchmark for software engineering',
    contextWindow: '200k tokens'
  },
  {
    id: 'claude-3-5-haiku',
    name: 'Claude 3.5 Haiku',
    provider: 'anthropic',
    providerName: 'Anthropic',
    badge: 'Fast',
    badgeColor: 'text-emerald-400 bg-emerald-500/10 border-emerald-500/20',
    description: 'Ultra-fast inference & responsive interactions',
    contextWindow: '200k tokens'
  },

  // OpenAI
  {
    id: 'gpt-4o',
    name: 'GPT-4o',
    provider: 'openai',
    providerName: 'OpenAI',
    badge: 'Omni',
    badgeColor: 'text-emerald-400 bg-emerald-500/10 border-emerald-500/20',
    description: 'Versatile multimodal intelligence & analysis',
    contextWindow: '128k tokens'
  },
  {
    id: 'o3-mini',
    name: 'o3-mini',
    provider: 'openai',
    providerName: 'OpenAI',
    badge: 'Reasoning',
    badgeColor: 'text-sky-400 bg-sky-500/10 border-sky-500/20',
    description: 'Deep mathematical & algorithmic reasoning',
    contextWindow: '200k tokens'
  },
  {
    id: 'gpt-4o-mini',
    name: 'GPT-4o mini',
    provider: 'openai',
    providerName: 'OpenAI',
    badge: 'Efficient',
    badgeColor: 'text-zinc-400 bg-zinc-500/10 border-zinc-500/20',
    description: 'Affordable fast coding and quick lookups',
    contextWindow: '128k tokens'
  },

  // Google Gemini
  {
    id: 'gemini-2.5-pro',
    name: 'Gemini 2.5 Pro',
    provider: 'gemini',
    providerName: 'Google',
    badge: '2M Context',
    badgeColor: 'text-indigo-400 bg-indigo-500/10 border-indigo-500/20',
    description: 'Extensive codebase understanding & deep reasoning',
    contextWindow: '2M tokens'
  },
  {
    id: 'gemini-2.5-flash',
    name: 'Gemini 2.5 Flash',
    provider: 'gemini',
    providerName: 'Google',
    badge: '1M Context',
    badgeColor: 'text-blue-400 bg-blue-500/10 border-blue-500/20',
    description: 'Sub-second latency with massive context capacity',
    contextWindow: '1M tokens'
  },

  // DeepSeek
  {
    id: 'deepseek-chat',
    name: 'DeepSeek V3',
    provider: 'deepseek',
    providerName: 'DeepSeek',
    badge: 'V3',
    badgeColor: 'text-cyan-400 bg-cyan-500/10 border-cyan-500/20',
    description: 'High-capability 671B MoE model',
    contextWindow: '64k tokens'
  },
  {
    id: 'deepseek-reasoner',
    name: 'DeepSeek R1',
    provider: 'deepseek',
    providerName: 'DeepSeek',
    badge: 'R1 Chain',
    badgeColor: 'text-cyan-400 bg-cyan-500/10 border-cyan-500/20',
    description: 'Open verified chain-of-thought reasoner',
    contextWindow: '64k tokens'
  },

  // Local / Sovereign
  {
    id: 'llama3.3:70b',
    name: 'Llama 3.3 70B',
    provider: 'local',
    providerName: 'Local (Ollama)',
    badge: 'Local Offline',
    badgeColor: 'text-teal-400 bg-teal-500/10 border-teal-500/20',
    description: 'Meta flagship sovereign local inference',
    contextWindow: '128k tokens'
  },
  {
    id: 'qwen2.5-coder:32b',
    name: 'Qwen 2.5 Coder 32B',
    provider: 'local',
    providerName: 'Local (Ollama)',
    badge: 'Coding Specialist',
    badgeColor: 'text-teal-400 bg-teal-500/10 border-teal-500/20',
    description: 'Specialized local code architecture engine',
    contextWindow: '32k tokens'
  },
  {
    id: 'deepseek-r1:14b',
    name: 'DeepSeek R1 14B Distill',
    provider: 'local',
    providerName: 'Local (Ollama)',
    badge: 'Local Reasoning',
    badgeColor: 'text-teal-400 bg-teal-500/10 border-teal-500/20',
    description: 'Lightweight local reasoning model',
    contextWindow: '32k tokens'
  }
];

export function getProviderInfo(modelId: string): {
  provider: ModelOption['provider'];
  label: string;
} {
  const m = (modelId || '').toLowerCase();
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
    case 'local':
      return <Server size={size} className={className || 'text-teal-400'} />;
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

export const ModelSelector: React.FC<ModelSelectorProps> = ({
  currentModel,
  onSelectModel,
  className = ''
}) => {
  const {
    openSettings,
    connectedAccounts,
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

  // Build the selectable list only from connected account catalogs.
  const allModels: ModelOption[] = useMemo(() => {
    const list = CANONICAL_MODELS.map((m) => ({ ...m }));

    connectedAccounts?.forEach((acc) => {
      if (acc.status === 'active') {
        acc.supportedModels?.forEach((sm) => {
          const match = list.find((m) => m.id.toLowerCase() === sm.toLowerCase());
          if (match) {
            match.isConfigured = true;
          } else {
            const info = getProviderInfo(sm);
            list.unshift({
              id: sm,
              name: sm,
              provider: info.provider,
              providerName: acc.providerName || info.label,
              badge: acc.authType === 'oauth' ? 'OAuth PKCE' : 'API key',
              badgeColor:
                acc.authType === 'oauth'
                  ? 'text-sky-400 bg-sky-500/10 border-sky-500/20'
                  : 'text-emerald-400 bg-emerald-500/10 border-emerald-500/20',
              description: `Live account: ${acc.accountName}`,
              contextWindow: acc.contextWindow ? `${Math.round(acc.contextWindow / 1000)}k tokens` : undefined,
              isConfigured: true,
            });
          }
        });
      }
    });

    return list.filter((m) => m.isConfigured);
  }, [connectedAccounts]);

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
      badge: 'Active'
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
          className="absolute bottom-full mb-2 left-0 z-50 w-72 md:w-72 rounded-xl border border-border-default bg-surface-1/95 backdrop-blur-xl shadow-2xl overflow-hidden animate-in fade-in slide-in-from-bottom-2 duration-150"
          style={{ maxHeight: '420px' }}
        >
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

            {filteredModels.length === 0 ? (
              <div className="p-6 text-center text-fg-subtle text-xs">
                No models matching "{searchQuery}"
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
