import React, { useState, useRef, useEffect, useMemo } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Check,
  Search,
  Settings,
  Cpu,
  Ban,
  Info,
  AlertTriangle,
} from 'lucide-react';
import {
  ProviderBrandIcon,
  OpenAIIcon,
  ClaudeIcon,
  GeminiIcon,
  DeepSeekIcon
} from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';

export type ThinkingEffort = 'low' | 'medium' | 'high';

export interface ModelOption {
  id: string;
  name: string;
  provider: 'anthropic' | 'openai' | 'gemini' | 'deepseek' | 'local' | 'antigravity' | 'copilot' | 'other';
  providerName: string;
  badge?: string;
  badgeColor?: string;
  notice?: string;
  isLeavingSoon?: boolean;
  description?: string;
  contextWindow?: string;
  isConfigured?: boolean;
  efforts?: ThinkingEffort[];
  defaultEffort?: ThinkingEffort;
  selectedEffort?: ThinkingEffort;
  supportsThinking?: boolean;
}

export interface UnifiedModelItem {
  id: string; // The active ID to pass to backend
  baseId: string;
  name: string;
  provider: ModelOption['provider'];
  providerName: string;
  supportsEffort: boolean;
  selectedEffort: ThinkingEffort;
  effortModelMap?: Partial<Record<ThinkingEffort, string>>;
  badge?: string;
  notice?: string;
  isLeavingSoon?: boolean;
  isConfigured: boolean;
  description?: string;
  contextWindow?: string;
}

export function getProviderInfo(modelId: string): {
  provider: ModelOption['provider'];
  label: string;
} {
  const m = (modelId || '').toLowerCase();
  if (m.startsWith('ollama/')) return { provider: 'local', label: 'Local (Ollama)' };
  if (m.startsWith('copilot/')) return { provider: 'copilot', label: 'GitHub Copilot' };
  if (m.startsWith('antigravity/')) return { provider: 'antigravity', label: 'Antigravity' };
  if (m.includes('claude') || m.includes('anthropic')) return { provider: 'anthropic', label: 'Anthropic Claude' };
  if (m.includes('gpt') || m.includes('o1') || m.includes('o3') || m.includes('openai')) return { provider: 'openai', label: 'OpenAI' };
  if (m.includes('gemini') || m.includes('google')) return { provider: 'gemini', label: 'Google Gemini' };
  if (m.includes('deepseek')) return { provider: 'deepseek', label: 'DeepSeek' };
  if (m.includes('llama') || m.includes('qwen') || m.includes('mistral') || m.includes('local') || m.includes('ollama')) {
    return { provider: 'local', label: 'Local (Ollama)' };
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

export function formatModelDisplayName(rawId: string): string {
  let clean = rawId.replace(/^antigravity\//i, '').replace(/^ollama\//i, '');
  clean = clean.replace(/-(high|medium|low|extra-low|tiered)$/i, '');

  if (clean.toLowerCase().includes('gemini-3.8-flash')) return 'Gemini 3.8 Flash';
  if (clean.toLowerCase().includes('gemini-3.7-flash')) return 'Gemini 3.7 Flash';
  if (clean.toLowerCase().includes('gemini-3.6-flash')) return 'Gemini 3.6 Flash';
  if (clean.toLowerCase().includes('gemini-3.5-flash-lite')) return 'Gemini 3.5 Flash Lite';
  if (clean.toLowerCase().includes('gemini-3.5-flash')) return 'Gemini 3.5 Flash';
  if (clean.toLowerCase().includes('gemini-3.1-pro')) return 'Gemini 3.1 Pro';
  if (clean.toLowerCase().includes('gemini-2.5-flash-thinking')) return 'Gemini 2.5 Flash Thinking';
  if (clean.toLowerCase().includes('gemini-2.5-flash')) return 'Gemini 2.5 Flash';
  if (clean.toLowerCase().includes('gemini-2.5-pro')) return 'Gemini 2.5 Pro';
  if (clean.toLowerCase().includes('gpt-oss-120b')) return 'GPT-OSS 120B (Medium)';
  if (
    clean.toLowerCase().includes('claude-3-7-sonnet') ||
    clean.toLowerCase().includes('sonnet-4-6') ||
    clean.toLowerCase().includes('sonnet-5-5')
  ) {
    return 'Claude Sonnet 4.6 (Thinking)';
  }
  if (
    clean.toLowerCase().includes('claude-opus-4-6') ||
    clean.toLowerCase().includes('claude-opus-5-5')
  ) {
    return 'Claude Opus 4.6 (Thinking)';
  }
  if (clean.toLowerCase().includes('claude-3-5-sonnet')) return 'Claude 3.5 Sonnet';
  if (clean.toLowerCase().includes('claude-3-5-haiku')) return 'Claude 3.5 Haiku';
  if (clean.toLowerCase().includes('o3-mini')) return 'o3-mini';
  if (clean.toLowerCase().includes('o1')) return 'o1 (Reasoning)';
  if (clean.toLowerCase().includes('gpt-4o-mini')) return 'GPT-4o Mini';
  if (clean.toLowerCase().includes('gpt-4o')) return 'GPT-4o';
  if (clean.toLowerCase().includes('deepseek-r1')) return 'DeepSeek-R1 (Thinking)';
  if (clean.toLowerCase().includes('deepseek-v3')) return 'DeepSeek-V3';

  // Capitalize hyphenated words
  return clean
    .split(/[-_]/)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}

export interface ModelSelectorProps {
  currentModel?: string;
  onSelectModel?: (modelId: string) => void;
  className?: string;
  compact?: boolean;
}

export function getConnectedModelOptions(accounts: import('@/types').ConnectedAccount[]): ModelOption[] {
  const seen = new Set<string>();
  const options: ModelOption[] = [];

  for (const account of accounts) {
    if (account.status !== 'active' && account.providerId !== 'local') continue;

    for (const id of account.supportedModels) {
      if (seen.has(id.toLowerCase())) continue;
      seen.add(id.toLowerCase());

      const metadata = account.catalogModels?.find((m) => m.id === id);
      const info = getProviderInfo(id);
      const provider = (account.providerId as ModelOption['provider']) || info.provider;
      const providerName = account.providerName || info.label;

      const idLower = id.toLowerCase();
      const isReasoning =
        idLower.includes('o1') ||
        idLower.includes('o3') ||
        idLower.includes('reasoner') ||
        idLower.includes('r1') ||
        idLower.includes('thinking') ||
        idLower.includes('flash') ||
        idLower.includes('pro') ||
        idLower.includes('sonnet') ||
        idLower.includes('opus') ||
        idLower.includes('tiered') ||
        idLower.includes('antigravity') ||
        Boolean(metadata?.efforts && metadata.efforts.length > 0);

      const efforts: ThinkingEffort[] = isReasoning ? ['low', 'medium', 'high'] : [];

      let isLeavingSoon = false;
      let notice: string | undefined = undefined;

      if (idLower.includes('3.6') || idLower.includes('3.7') || idLower.includes('3.1-pro')) {
        isLeavingSoon = true;
      }
      if (
        idLower.includes('sonnet-4') ||
        idLower.includes('sonnet-5') ||
        idLower.includes('opus-4') ||
        idLower.includes('opus-5') ||
        idLower.includes('gpt-oss-120b')
      ) {
        notice = 'Notice ⓘ';
      }

      options.push({
        id,
        name: formatModelDisplayName(id),
        provider,
        providerName,
        badge: account.status === 'offline' ? 'Offline' : account.catalogSource === 'cached' ? 'Cached' : 'Live',
        notice,
        isLeavingSoon,
        description: metadata?.description || `Account: ${account.accountName}`,
        contextWindow: metadata?.context_window ? `${Math.round(metadata.context_window / 1000)}k tokens` : undefined,
        isConfigured: account.status === 'active',
        efforts,
        defaultEffort: 'medium',
        supportsThinking: isReasoning,
      });
    }
  }

  return options;
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

  const activeModelId = currentModel !== undefined ? currentModel : (contextSelectedModel || '');
  const handleSelect = onSelectModel || contextHandleSelectModel;

  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedProviderFilter, setSelectedProviderFilter] = useState<string>('all');
  const [hoveredBaseId, setHoveredBaseId] = useState<string | null>(null);
  const [hoveredEffortItem, setHoveredEffortItem] = useState<{
    model: UnifiedModelItem;
    top: number;
  } | null>(null);

  const popoverRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Close when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setIsOpen(false);
        setHoveredBaseId(null);
        setHoveredEffortItem(null);
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsOpen(false);
        setHoveredBaseId(null);
        setHoveredEffortItem(null);
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

  const rawModels = useMemo(() => getConnectedModelOptions(connectedAccounts), [connectedAccounts]);

  const getDefaultEffortForModel = (baseId: string): ThinkingEffort => {
    const lower = baseId.toLowerCase();
    if (lower.includes('3.8')) return 'high';
    if (lower.includes('3.1-pro')) return 'low';
    return 'medium';
  };

  // Collapse tiered / variant models into clean UnifiedModelItems
  const unifiedModels = useMemo<UnifiedModelItem[]>(() => {
    const map = new Map<string, UnifiedModelItem>();

    // Helper to get stored effort for a base model
    const getStoredEffort = (baseKey: string, fallback: ThinkingEffort): ThinkingEffort => {
      try {
        const stored = localStorage.getItem(`custos.model_effort.${baseKey}`);
        if (stored === 'low' || stored === 'medium' || stored === 'high') return stored;
      } catch { /* empty */ }
      return fallback;
    };

    for (const m of rawModels) {
      const idLower = m.id.toLowerCase();

      // Detect effort suffix (-high, -medium, -low, -extra-low, -tiered)
      let effortFromSuffix: ThinkingEffort | null = null;
      let baseKey = m.id;

      if (idLower.endsWith('-extra-low')) {
        effortFromSuffix = 'low';
        baseKey = m.id.replace(/-extra-low$/i, '');
      } else {
        const suffixMatch = idLower.match(/-(high|medium|low)$/);
        if (suffixMatch) {
          effortFromSuffix = suffixMatch[1] as ThinkingEffort;
          baseKey = m.id.replace(/-(high|medium|low)$/i, '');
        } else if (idLower.endsWith('-tiered')) {
          baseKey = m.id.replace(/-tiered$/i, '');
        }
      }

      const existing = map.get(baseKey);
      if (existing) {
        if (effortFromSuffix && existing.effortModelMap) {
          existing.effortModelMap[effortFromSuffix] = m.id;
          // If activeModelId is this variant, update selectedEffort
          if (activeModelId.toLowerCase() === m.id.toLowerCase()) {
            existing.selectedEffort = effortFromSuffix;
            existing.id = m.id;
          }
        }
        continue;
      }

      // New unified item
      const isReasoning =
        m.supportsThinking ||
        Boolean(effortFromSuffix) ||
        idLower.includes('tiered') ||
        idLower.includes('thinking') ||
        idLower.includes('flash') ||
        idLower.includes('pro') ||
        idLower.includes('sonnet') ||
        idLower.includes('opus') ||
        idLower.includes('o1') ||
        idLower.includes('o3') ||
        idLower.includes('r1');

      const displayName = formatModelDisplayName(m.id);
      const isFixedMode =
        displayName.includes('(Thinking)') ||
        displayName.includes('(Medium)');
      const supportsEffort = isReasoning && !isFixedMode;

      let currentEffort: ThinkingEffort =
        getStoredEffort(baseKey, getDefaultEffortForModel(baseKey));

      // If activeModelId in session matches this variant, sync selected effort
      if (activeModelId.toLowerCase().includes(baseKey.toLowerCase())) {
        if (activeModelId.toLowerCase().endsWith('-high')) currentEffort = 'high';
        else if (activeModelId.toLowerCase().endsWith('-low') || activeModelId.toLowerCase().endsWith('-extra-low')) currentEffort = 'low';
        else if (activeModelId.toLowerCase().endsWith('-medium')) currentEffort = 'medium';
      }

      const effortMap: Partial<Record<ThinkingEffort, string>> = {};
      if (effortFromSuffix) {
        effortMap[effortFromSuffix] = m.id;
      } else if (idLower.endsWith('-tiered')) {
        effortMap.low = m.id;
        effortMap.medium = m.id;
        effortMap.high = m.id;
      }

      let isLeavingSoon = m.isLeavingSoon;
      let notice = m.notice;
      if (
        idLower.includes('3.6') ||
        idLower.includes('3.7') ||
        idLower.includes('3.1-pro')
      ) {
        isLeavingSoon = true;
      }
      if (
        idLower.includes('sonnet-4') ||
        idLower.includes('sonnet-5') ||
        idLower.includes('opus-4') ||
        idLower.includes('opus-5') ||
        idLower.includes('gpt-oss-120b')
      ) {
        notice = 'Notice ⓘ';
      }

      map.set(baseKey, {
        id: m.id,
        baseId: baseKey,
        name: displayName,
        provider: m.provider,
        providerName: m.providerName,
        supportsEffort,
        selectedEffort: currentEffort,
        effortModelMap: effortMap,
        badge: m.badge,
        notice,
        isLeavingSoon,
        isConfigured: m.isConfigured !== false,
        description: m.description,
        contextWindow: m.contextWindow,
      });
    }

    return Array.from(map.values());
  }, [rawModels, activeModelId]);

  // Find currently active item
  const activeUnifiedItem = useMemo(() => {
    if (!activeModelId) return null;
    const lower = activeModelId.toLowerCase();
    return unifiedModels.find(
      (m) =>
        m.id.toLowerCase() === lower ||
        m.baseId.toLowerCase() === lower ||
        (m.effortModelMap && Object.values(m.effortModelMap).some((id) => id?.toLowerCase() === lower))
    ) || null;
  }, [unifiedModels, activeModelId]);

  // Priority sorting helper to match Antigravity model order
  const getModelPriority = (id: string, name: string): number => {
    const s = (id + ' ' + name).toLowerCase();
    if (s.includes('3.8-flash')) return 10;
    if (s.includes('3.7-flash')) return 20;
    if (s.includes('3.6-flash')) return 30;
    if (s.includes('3.1-pro')) return 40;
    if (s.includes('3.5-flash-lite')) return 48;
    if (s.includes('3.5-flash')) return 45;
    if (s.includes('sonnet')) return 50;
    if (s.includes('opus')) return 60;
    if (s.includes('gpt-oss-120b')) return 70;
    if (s.includes('o3-mini')) return 80;
    if (s.includes('o1')) return 85;
    if (s.includes('gpt-4o')) return 90;
    if (s.includes('gemini-2.5-pro')) return 100;
    if (s.includes('gemini-2.5-flash')) return 110;
    if (s.includes('deepseek-r1')) return 120;
    if (s.includes('deepseek-v3')) return 130;
    return 200;
  };

  // Filter models by provider and search query
  const filteredModels = useMemo(() => {
    let result = unifiedModels;

    if (selectedProviderFilter !== 'all') {
      result = result.filter((m) => m.provider === selectedProviderFilter);
    }

    const q = searchQuery.trim().toLowerCase();
    if (q) {
      result = result.filter(
        (m) =>
          m.name.toLowerCase().includes(q) ||
          m.id.toLowerCase().includes(q) ||
          m.baseId.toLowerCase().includes(q) ||
          m.providerName.toLowerCase().includes(q)
      );
    }

    return result;
  }, [unifiedModels, selectedProviderFilter, searchQuery]);

  // Group models by Provider
  const groupedByProvider = useMemo(() => {
    const groups: Array<{
      providerKey: ModelOption['provider'];
      providerName: string;
      models: UnifiedModelItem[];
    }> = [];

    const order: Array<ModelOption['provider']> = [
      'antigravity',
      'gemini',
      'anthropic',
      'openai',
      'deepseek',
      'local',
      'other',
    ];

    for (const key of order) {
      const items = filteredModels.filter((m) => m.provider === key);
      if (items.length > 0) {
        const sortedItems = [...items].sort((a, b) => {
          return getModelPriority(a.baseId, a.name) - getModelPriority(b.baseId, b.name);
        });
        groups.push({
          providerKey: key,
          providerName: items[0].providerName,
          models: sortedItems,
        });
      }
    }

    return groups;
  }, [filteredModels]);

  const handleSelectWithEffort = (model: UnifiedModelItem, effort: ThinkingEffort) => {
    try {
      localStorage.setItem(`custos.model_effort.${model.baseId}`, effort);
    } catch { /* empty */ }

    // Resolve target model ID
    let targetId = model.id;
    if (model.effortModelMap && model.effortModelMap[effort]) {
      targetId = model.effortModelMap[effort]!;
    } else if (model.id.includes('-high') || model.id.includes('-medium') || model.id.includes('-low')) {
      targetId = `${model.baseId}-${effort}`;
    }

    handleSelect(targetId);
    setIsOpen(false);
    setHoveredBaseId(null);
    setHoveredEffortItem(null);
  };

  const handleDirectModelClick = (model: UnifiedModelItem) => {
    handleSelectWithEffort(model, model.selectedEffort);
  };

  const availableProviders = useMemo(() => {
    const set = new Set<ModelOption['provider']>();
    unifiedModels.forEach((m) => set.add(m.provider));
    const list = Array.from(set);
    const order: Array<ModelOption['provider']> = [
      'antigravity',
      'openai',
      'gemini',
      'anthropic',
      'deepseek',
      'local',
      'other',
    ];
    return list.sort((a, b) => order.indexOf(a) - order.indexOf(b));
  }, [unifiedModels]);

  const hasModel = Boolean(activeModelId && activeModelId.trim().length > 0 && activeModelId !== 'No model selected');

  return (
    <div className={`relative inline-block text-left ${className}`} ref={popoverRef}>
      {/* Trigger Button inside Composer */}
      <button
        type="button"
        onClick={() => {
          setIsOpen((prev) => !prev);
          setHoveredBaseId(null);
          setHoveredEffortItem(null);
        }}
        className={`flex items-center gap-1.5 px-2 py-1 rounded-md text-[11px] font-medium transition select-none ${
          !hasModel
            ? 'bg-amber-500/10 hover:bg-amber-500/20 text-amber-400 border border-amber-500/30'
            : isOpen
            ? 'bg-surface-2 text-fg-editor border border-border-default shadow-xs'
            : 'bg-surface-1/80 hover:bg-surface-2 text-fg-muted hover:text-fg-editor border border-border-muted hover:border-border-default'
        }`}
        title={!hasModel ? 'No model selected' : `Active model: ${activeModelId}`}
      >
        {!hasModel ? (
          <Cpu size={13} className="shrink-0 text-amber-400" />
        ) : (
          <ProviderIcon provider={activeUnifiedItem?.provider || getProviderInfo(activeModelId).provider} size={13} className="shrink-0" />
        )}

        <span className="font-mono text-[11px] truncate max-w-[170px] text-fg-editor">
          {!hasModel ? (
            'No model selected'
          ) : (
            <>
              {activeUnifiedItem?.name || activeModelId}
              {activeUnifiedItem?.supportsEffort && (
                <span className="ml-1 text-[10px] text-zinc-400 capitalize opacity-90 font-sans">
                  {activeUnifiedItem.selectedEffort}
                </span>
              )}
            </>
          )}
        </span>

        <ChevronDown
          size={12}
          className={`shrink-0 text-fg-subtle transition-transform duration-150 ${
            isOpen ? 'rotate-180 text-fg-editor' : ''
          }`}
        />
      </button>

      {/* Floating Popover Container (Opens Upward Above Composer) */}
      {isOpen && (
        <div
          ref={menuRef}
          className="app-dropdown-menu absolute bottom-full mb-2 left-0 z-50 w-80 md:w-88 rounded-xl border border-border-default bg-[#1e1e1e]/98 backdrop-blur-2xl shadow-2xl overflow-visible animate-in fade-in slide-in-from-bottom-2 duration-150"
          style={{ maxHeight: '460px' }}
        >
          {/* Header */}
          <div className="p-2.5 border-b border-border-muted/60 bg-surface-0/60">
            <div className="flex items-center justify-between mb-2 px-1">
              <span className="text-[11px] font-semibold tracking-wide text-zinc-400 uppercase">
                Model
              </span>
              <span className="text-[10px] text-fg-subtle font-mono">
                {unifiedModels.length} models
              </span>
            </div>

            {/* Provider Filter Tabs (Định hình của ai rõ ràng) */}
            {availableProviders.length > 1 && (
              <div className="flex items-center gap-1 mb-2 overflow-x-auto no-scrollbar pb-0.5">
                <button
                  type="button"
                  onClick={() => setSelectedProviderFilter('all')}
                  className={`px-2 py-0.5 rounded text-[10px] font-medium transition shrink-0 ${
                    selectedProviderFilter === 'all'
                      ? 'bg-surface-3 text-fg-editor border border-border-default'
                      : 'text-fg-subtle hover:text-fg-editor hover:bg-surface-2'
                  }`}
                >
                  All
                </button>
                {availableProviders.map((prov) => {
                  const label =
                    prov === 'antigravity'
                      ? 'Antigravity'
                      : prov === 'openai'
                      ? 'OpenAI'
                      : prov === 'anthropic'
                      ? 'Anthropic'
                      : prov === 'gemini'
                      ? 'Gemini'
                      : prov === 'local'
                      ? 'Local'
                      : prov === 'deepseek'
                      ? 'DeepSeek'
                      : prov;
                  return (
                    <button
                      key={prov}
                      type="button"
                      onClick={() => setSelectedProviderFilter(prov)}
                      className={`flex items-center gap-1.5 px-2 py-0.5 rounded text-[10px] font-medium transition shrink-0 ${
                        selectedProviderFilter === prov
                          ? 'bg-surface-3 text-fg-editor border border-border-default'
                          : 'text-fg-subtle hover:text-fg-editor hover:bg-surface-2'
                      }`}
                    >
                      <ProviderIcon provider={prov} size={11} />
                      <span>{label}</span>
                    </button>
                  );
                })}
              </div>
            )}

            {/* Search Input */}
            <div className="relative">
              <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-subtle" />
              <input
                ref={searchInputRef}
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search models..."
                className="w-full bg-[#2a2a2a] border border-border-muted/70 focus:border-blue-500 focus:ring-1 focus:ring-blue-500/20 rounded-lg pl-8 pr-3 py-1.5 text-xs text-fg-editor placeholder-fg-subtle outline-none transition"
              />
            </div>
          </div>

          {/* Model Options List */}
          <div className="overflow-y-auto p-1.5 space-y-2" style={{ maxHeight: '320px' }}>
            {groupedByProvider.length === 0 ? (
              <div className="p-4 text-center text-fg-subtle text-xs">
                No models matching "{searchQuery}"
              </div>
            ) : (
              groupedByProvider.map((group) => (
                <div key={group.providerKey} className="space-y-0.5">
                  {/* Provider Section Header */}
                  <div className="px-2 pt-1.5 pb-1 flex items-center justify-between text-[10px] font-semibold text-fg-subtle uppercase tracking-wider">
                    <span className="flex items-center gap-1.5 text-zinc-300">
                      <ProviderIcon provider={group.providerKey} size={12} />
                      <span>{group.providerName}</span>
                    </span>
                    <span className="font-mono text-[9px] opacity-70">
                      {group.models.length}
                    </span>
                  </div>

                  {/* Provider Models */}
                  {group.models.map((m) => {
                    const isSelected =
                      activeUnifiedItem?.baseId === m.baseId ||
                      activeModelId.toLowerCase() === m.id.toLowerCase();
                    const isHovered = hoveredBaseId === m.baseId;

                    return (
                      <div
                        key={m.baseId}
                        className="relative"
                        onMouseEnter={(e) => {
                          setHoveredBaseId(m.baseId);
                          if (m.supportsEffort) {
                            const rowRect = e.currentTarget.getBoundingClientRect();
                            const menuRect = menuRef.current?.getBoundingClientRect();
                            if (menuRect) {
                              const relativeTop = rowRect.top - menuRect.top;
                              const targetTop = Math.max(8, Math.min(relativeTop - 28, menuRect.height - 110));
                              setHoveredEffortItem({
                                model: m,
                                top: targetTop,
                              });
                            }
                          } else {
                            setHoveredEffortItem(null);
                          }
                        }}
                      >
                        <button
                          type="button"
                          onClick={() => handleDirectModelClick(m)}
                          className={`w-full text-left px-2.5 py-1.5 rounded-lg flex items-center justify-between transition group select-none ${
                            isSelected
                              ? 'border border-blue-500 bg-[#252528] text-fg-editor shadow-sm'
                              : isHovered
                              ? 'bg-surface-2/80 text-fg-editor border border-transparent'
                              : 'hover:bg-surface-2/50 text-fg-muted hover:text-fg-editor border border-transparent'
                          }`}
                        >
                          <div className="flex items-center gap-2 min-w-0 flex-1">
                            <span className="font-medium text-xs text-fg-editor truncate">
                              {m.name}
                            </span>

                            {/* Current Effort Label */}
                            {m.supportsEffort && (
                              <span className="text-[11px] text-zinc-400 capitalize font-sans shrink-0">
                                {m.selectedEffort}
                              </span>
                            )}

                            {/* Leaving Soon Badge */}
                            {m.isLeavingSoon && (
                              <span className="flex items-center gap-1 px-1.5 py-0.2 rounded-full text-[9px] font-sans border border-zinc-700/60 bg-zinc-800/80 text-zinc-400 shrink-0">
                                Leaving Soon
                                <Info size={9} />
                              </span>
                            )}

                            {/* Notice Badge */}
                            {m.notice && (
                              <span className="flex items-center gap-1 px-1.5 py-0.2 rounded-full text-[9px] font-sans border border-amber-500/30 bg-amber-500/10 text-amber-400 shrink-0">
                                <AlertTriangle size={9} />
                                Notice
                                <Info size={9} />
                              </span>
                            )}
                          </div>

                          <div className="flex items-center gap-1 shrink-0 ml-1">
                            {/* Chevron Right for Effort Submenu (3 Chế độ chọn) */}
                            {m.supportsEffort && (
                              <ChevronRight
                                size={13}
                                className={`transition text-zinc-500 group-hover:text-fg-editor ${
                                  isHovered ? 'translate-x-0.5 text-blue-400' : ''
                                }`}
                              />
                            )}
                          </div>
                        </button>
                      </div>
                    );
                  })}
                </div>
              ))
            )}
          </div>

          {/* FLYOUT SUBMENU (Positioned Next to Hovered Row: Low / Medium / High) */}
          {hoveredEffortItem && hoveredEffortItem.model.supportsEffort && (
            <div
              style={{
                position: 'absolute',
                top: `${hoveredEffortItem.top}px`,
                left: 'calc(100% + 6px)',
                zIndex: 100,
              }}
              className="w-32 rounded-xl border border-zinc-700/60 bg-[#252528]/98 backdrop-blur-2xl p-1 shadow-2xl animate-in fade-in-50 duration-75 select-none"
              onMouseEnter={() => setHoveredBaseId(hoveredEffortItem.model.baseId)}
              onMouseLeave={() => {
                setHoveredEffortItem(null);
                setHoveredBaseId(null);
              }}
            >
              {/* 12px hover bridge to prevent mouseleave across 6px gap */}
              <div className="absolute -left-3 top-0 w-3 h-full pointer-events-auto" />
              <div className="space-y-0.5">
                {(['low', 'medium', 'high'] as ThinkingEffort[]).map((effort) => {
                  const isEffortActive = hoveredEffortItem.model.selectedEffort === effort;
                  return (
                    <button
                      key={effort}
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleSelectWithEffort(hoveredEffortItem.model, effort);
                      }}
                      className={`w-full text-left px-3 py-1.5 rounded-lg text-xs capitalize transition flex items-center justify-between select-none ${
                        isEffortActive
                          ? 'bg-zinc-700/70 text-white font-medium border border-zinc-600/40 shadow-xs'
                          : 'text-zinc-300 hover:bg-zinc-800/80 hover:text-white'
                      }`}
                    >
                      <span>{effort}</span>
                      {isEffortActive && (
                        <Check size={12} className="text-blue-400 stroke-[2.5]" />
                      )}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Footer */}
          <div className="p-2 border-t border-border-muted bg-surface-0/60 flex items-center justify-between text-[10px]">
            <button
              type="button"
              onClick={() => {
                handleSelect('');
                setIsOpen(false);
                setHoveredBaseId(null);
                setHoveredEffortItem(null);
              }}
              className="text-zinc-400 hover:text-amber-400 flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-surface-2 transition"
            >
              <Ban size={11} />
              Unset Model
            </button>

            <button
              type="button"
              onClick={() => {
                setIsOpen(false);
                setHoveredBaseId(null);
                setHoveredEffortItem(null);
                openSettings('providers');
              }}
              className="text-fg-muted hover:text-fg-editor flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-surface-2 transition"
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

