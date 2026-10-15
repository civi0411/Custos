import React, { useState, useRef, useEffect, useMemo } from 'react';
import { createPortal } from 'react-dom';
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
import { daemonClient } from '@/api/daemon_client';
import type { LoadedModelInstance, LocalModelDescriptor } from '@/types/domain';

export type ThinkingEffort = string;

export interface ModelOption {
  accountId: string;
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
  familyKey?: string;
  variantEffort?: string;
}

export interface UnifiedModelItem {
  accountId: string;
  routes: ModelOption[];
  efforts: string[];
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
  if (m.startsWith('native:')) return { provider: 'local', label: 'Local (Native Metal)' };
  if (m.startsWith('local/') || m.startsWith('local:')) return { provider: 'local', label: 'Local (Apple Silicon)' };
  if (m.startsWith('ollama/')) return { provider: 'local', label: 'Local (Ollama)' };
  if (m.startsWith('copilot/')) return { provider: 'copilot', label: 'GitHub Copilot' };
  if (m.startsWith('antigravity/')) return { provider: 'antigravity', label: 'Antigravity' };
  if (m.includes('claude') || m.includes('anthropic')) return { provider: 'anthropic', label: 'Anthropic Claude' };
  if (m.includes('gpt') || m.includes('o1') || m.includes('o3') || m.includes('openai')) return { provider: 'openai', label: 'OpenAI' };
  if (m.includes('gemini') || m.includes('google')) return { provider: 'gemini', label: 'Google Gemini' };
  if (m.includes('deepseek')) return { provider: 'deepseek', label: 'DeepSeek' };
  if (m.includes('qwopus') || m.includes('llama') || m.includes('qwen') || m.includes('mistral') || m.includes('local') || m.includes('ollama')) {
    return { provider: 'local', label: 'Local (Apple Silicon)' };
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
  let clean = rawId
    .replace(/^native:/i, '')
    .replace(/^(antigravity|ollama|copilot|local|gemini-oauth)\//i, '')
    .replace(/^local:/i, '');
  clean = clean.replace(/-(high|medium|low|extra-low|tiered)$/i, '');
  if (clean.includes('/') || clean.includes('\\')) {
    clean = clean.split(/[/\\]/).pop() || clean;
  }
  if (clean.toLowerCase().endsWith('.gguf')) {
    clean = clean.slice(0, -5);
  }
  return clean.split(/[-_]/).map(word => {
    if (/^(gpt|oss)$/i.test(word)) return word.toUpperCase();
    return word.charAt(0).toUpperCase() + word.slice(1);
  }).join(' ');
}

/** Provider catalog labels define aliases; transport IDs are never synthesized. */
export function normalizeModelPresentation(id: string, label: string | undefined, provider: string) {
  const advertisedLabel = label && label !== id ? label.trim() : undefined;
  const annotation = advertisedLabel?.match(/\s*\((none|minimal|extra-low|low|medium|high|xhigh|max|ultra)\)\s*$/i);
  const suffix = provider === 'antigravity' && !advertisedLabel
    ? id.match(/-(extra-low|none|minimal|low|medium|high|xhigh|max|ultra)$/i) : null;
  const variantEffort = (annotation?.[1] || suffix?.[1])?.toLowerCase();
  const name = annotation ? advertisedLabel!.slice(0, annotation.index).trim()
    : suffix ? formatModelDisplayName(id.slice(0, suffix.index))
    : advertisedLabel || formatModelDisplayName(id);
  return { family: provider === 'antigravity' || annotation ? name.toLowerCase() : id, name, variantEffort };
}

/** One presentation family retains every account and discovered transport variant. */
export function groupModelOptions(models: ModelOption[]): ModelOption[][] {
  const groups = new Map<string, ModelOption[]>();
  for (const model of models) {
    const key = model.familyKey || JSON.stringify([model.provider, model.id]);
    const group = groups.get(key) || [];
    group.push(model);
    groups.set(key, group);
  }
  return [...groups.values()];
}

export function resolveModelRoute(routes: ModelOption[], accountId: string, effort: string, preferredId?: string) {
  const candidates = routes.filter(route => route.accountId === accountId &&
    (route.variantEffort ? route.variantEffort === effort : !effort || route.efforts?.includes(effort)));
  return candidates.find(route => route.id === preferredId) || candidates.find(route =>
    route.id.split('/').pop() === route.name.toLowerCase().replace(/ /g, '-')) || candidates[0];
}

export function compareModelPresentation(a: Pick<ModelOption, 'name'>, b: Pick<ModelOption, 'name'>): number {
  const rank = (name: string) => /gemini/i.test(name) ? 0 : /claude/i.test(name) ? 1 : /gpt[- ]oss/i.test(name) ? 2 : 3;
  return rank(a.name) - rank(b.name) || b.name.localeCompare(a.name, undefined, { numeric: true });
}

export interface ModelSelectorProps {
  currentModel?: string;
  onSelectModel?: (modelId: string, effort?: string, accountId?: string) => void;
  className?: string;
  compact?: boolean;
}

export function getConnectedModelOptions(accounts: import('@/types').ConnectedAccount[]): ModelOption[] {
  const seen = new Set<string>();
  const options: ModelOption[] = [];

  for (const account of accounts) {
    if (account.status !== 'active') continue;

    for (const id of account.supportedModels) {
      const key = `${account.id}:${id}`;
      if (seen.has(key)) continue;
      seen.add(key);

      const metadata = account.catalogModels?.find((m) => m.id === id);
      const info = getProviderInfo(id);
      // A route namespace identifies the serving connection, regardless of model family.
      const provider = info.provider === 'copilot' ? 'copilot' : (account.providerId as ModelOption['provider']) || info.provider;
      const providerName = provider === 'copilot' ? 'GitHub Copilot' : account.providerName || info.label;

      const idLower = id.toLowerCase();
      const efforts = metadata?.efforts || [];
      const presentation = normalizeModelPresentation(id, metadata?.label, provider);
      const isReasoning = efforts.length > 0 || Boolean(presentation.variantEffort);

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
        accountId: account.id,
        id,
        name: presentation.name,
        familyKey: JSON.stringify([provider, presentation.family]),
        variantEffort: presentation.variantEffort,
        provider,
        providerName,
        badge: account.catalogSource === 'cached' ? 'Cached' : 'Live',
        notice,
        isLeavingSoon,
        description: metadata?.description || `Account: ${account.accountName}`,
        contextWindow: metadata?.context_window ? `${Math.round(metadata.context_window / 1000)}k tokens` : undefined,
        isConfigured: account.status === 'active',
        efforts,
        defaultEffort: metadata?.default_effort,
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
    selectedReasoningEffort,
    handleSelectModel: contextHandleSelectModel
  } = useAppContext();

  const activeModelId = currentModel !== undefined ? currentModel : (contextSelectedModel || '');
  const handleSelect = onSelectModel || contextHandleSelectModel;

  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [effortPreferences, setEffortPreferences] = useState<Partial<Record<string, ThinkingEffort>>>({});
  const [selectedProviderFilter, setSelectedProviderFilter] = useState<string>('all');
  const [hoveredBaseId, setHoveredBaseId] = useState<string | null>(null);
  const [hoveredEffortItem, setHoveredEffortItem] = useState<{
    model: UnifiedModelItem;
    top: number;
    left: number;
  } | null>(null);
  const [loadedInstances, setLoadedInstances] = useState<LoadedModelInstance[]>([]);
  const [localScannedModels, setLocalScannedModels] = useState<LocalModelDescriptor[]>([]);
  const [loadingNativeModel, setLoadingNativeModel] = useState<string | null>(null);
  const [nativeLoadError, setNativeLoadError] = useState<string | null>(null);

  const refreshLoadedInstances = async () => {
    try {
      const [instRes, scanRes] = await Promise.all([
        daemonClient.listLoadedInstances(),
        daemonClient.listLocalModels(),
      ]);
      setLoadedInstances(instRes.instances || []);
      setLocalScannedModels(scanRes.models || []);
    } catch (error) {
      setNativeLoadError(error instanceof Error ? error.message : String(error));
    }
  };

  useEffect(() => {
    void refreshLoadedInstances();
  }, []);

  const popoverRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const submenuRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Close when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node) && !submenuRef.current?.contains(e.target as Node)) {
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
      void refreshLoadedInstances();
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeyDown);
      setTimeout(() => searchInputRef.current?.focus(), 50);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  const rawModels = useMemo(() => {
    const baseOptions = getConnectedModelOptions(connectedAccounts);

    // 1. Instances currently resident in RAM
    const loadedPaths = new Set(loadedInstances.map((inst) => inst.model_path));
    const nativeLoadedOptions: ModelOption[] = loadedInstances.map((inst) => {
      return {
        accountId: 'local-native',
        id: `native:${inst.model_path}`,
        name: inst.model_name || formatModelDisplayName(inst.model_path),
        provider: 'local' as const,
        providerName: 'Local (Native)',
        badge: 'Loaded',
        badgeColor: 'emerald',
        description: 'Loaded in the Custos local inference runtime',
        contextWindow: inst.context_limit > 0 ? `${Math.round(inst.context_limit / 1000)}k tokens` : undefined,
        isConfigured: true,
        efforts: [],
        supportsThinking: false,
      };
    });

    // 2. Scanned GGUF models on disk (ready to load)
    const scannedOptions: ModelOption[] = localScannedModels
      .filter((m) => !loadedPaths.has(m.file_path))
      .map((m) => {
        const sizeGb = (m.file_size_bytes / (1024 * 1024 * 1024)).toFixed(1);
        return {
          accountId: 'local-native',
          id: `native:${m.file_path}`,
          name: m.model_name || formatModelDisplayName(m.file_name),
          provider: 'local' as const,
          providerName: 'Local (Native)',
          badge: `${sizeGb} GB`,
          badgeColor: 'teal',
          description: `GGUF · Arch: ${m.architecture || 'gguf'} · Quant: ${m.quantization || 'Q4'}`,
          contextWindow: `${m.context_length || 8192}`,
          isConfigured: true,
          efforts: [],
          supportsThinking: false,
        };
      });

    return [...nativeLoadedOptions, ...scannedOptions, ...baseOptions];
  }, [connectedAccounts, loadedInstances, localScannedModels]);

  const unifiedModels = useMemo<UnifiedModelItem[]>(() => groupModelOptions(rawModels).map(routes => {
    const activeRoute = routes.find(route => route.id === activeModelId);
    const model = activeRoute || resolveModelRoute(routes, routes[0].accountId, routes[0].variantEffort || '') || routes[0];
    const baseId = model.familyKey!;
    const accountRoutes = routes;
    const effortOrder = ['none', 'minimal', 'extra-low', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'];
    const efforts = [...new Set(accountRoutes.flatMap(route => route.variantEffort ? [route.variantEffort] : route.efforts || []))]
      .sort((a, b) => effortOrder.indexOf(a) - effortOrder.indexOf(b));
    const preferenceKey = baseId;
    let saved = effortPreferences[preferenceKey];
    if (!saved) { try { saved = localStorage.getItem(`custos.model_effort.${preferenceKey}`) || undefined; } catch { /* Device storage is optional. */ } }
    const preferred = activeRoute ? activeRoute.variantEffort || selectedReasoningEffort || saved : saved;
    return {
      ...model, routes, baseId, supportsEffort: efforts.length > 0, efforts,
      selectedEffort: preferred && efforts.includes(preferred) ? preferred : model.defaultEffort || model.variantEffort || '',
      isConfigured: model.isConfigured !== false,
    };
  }), [rawModels, activeModelId, selectedReasoningEffort, effortPreferences]);

  const activeUnifiedItem = useMemo(() => unifiedModels.find(model => model.routes.some(route => route.id === activeModelId)) || null,
    [unifiedModels, activeModelId]);

  useEffect(() => {
    if (hoveredEffortItem && !unifiedModels.some(model => model.baseId === hoveredEffortItem.model.baseId)) {
      setHoveredEffortItem(null);
      setHoveredBaseId(null);
    }
  }, [unifiedModels, hoveredEffortItem]);

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
      'copilot',
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
          return compareModelPresentation(a, b);
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

  const handleSelectWithEffort = async (model: UnifiedModelItem, effort: ThinkingEffort) => {
    if (loadingNativeModel) return;
    setNativeLoadError(null);

    // Native local model loading lifecycle
    if (model.id.startsWith('native:')) {
      const filePath = model.id.slice('native:'.length);
      const isAlreadyLoaded = loadedInstances.some((inst) => inst.model_path === filePath);
      if (!isAlreadyLoaded) {
        setLoadingNativeModel(model.id);
        try {
          const preflight = await daemonClient.preflightLocalModel(filePath, 8192);
          if (!preflight.can_load) {
            throw new Error(preflight.issues.join('. ') || 'Model file may exceed physical memory');
          }
          await daemonClient.loadLocalModel(filePath, preflight.context_limit, preflight.recommended_gpu_layers);
          void refreshLoadedInstances();
        } catch (error) {
          setNativeLoadError(error instanceof Error ? error.message : String(error));
          return;
        } finally {
          setLoadingNativeModel(null);
        }
      }
      handleSelect(model.id);
      setIsOpen(false);
      setHoveredBaseId(null);
      setHoveredEffortItem(null);
      return;
    }

    const route = model.routes.map(candidate => resolveModelRoute(model.routes, candidate.accountId, effort, model.id)).find(Boolean);
    if (!route) return;
    const preferenceKey = model.baseId;
    try {
      localStorage.setItem(`custos.model_effort.${preferenceKey}`, effort);
    } catch { /* empty */ }

    setEffortPreferences(previous => ({ ...previous, [preferenceKey]: effort }));
    handleSelect(route.id, route.variantEffort ? undefined : effort || undefined);
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
      'copilot',
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
        title={loadingNativeModel ? 'Loading local model…' : !hasModel ? 'No model selected' : `Active model: ${activeModelId}`}
      >
        {!hasModel ? (
          <Cpu size={13} className="shrink-0 text-amber-400" />
        ) : (
          <ProviderIcon provider={activeUnifiedItem?.provider || getProviderInfo(activeModelId).provider} size={13} className="shrink-0" />
        )}

        <span className="font-mono text-[11px] truncate max-w-[170px] text-fg-editor">
          {loadingNativeModel ? (
            'Loading local model…'
          ) : !hasModel ? (
            'No model selected'
          ) : (
            <>
              {activeUnifiedItem?.name || formatModelDisplayName(activeModelId)}{!activeUnifiedItem && ' · Unavailable'}
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
          className="app-dropdown-menu absolute bottom-full mb-2 left-0 z-50 flex flex-col w-80 md:w-88 rounded-xl border border-border-default bg-[#1e1e1e]/98 backdrop-blur-2xl shadow-2xl overflow-visible animate-in fade-in slide-in-from-bottom-2 duration-150"
          style={{ maxHeight: 'min(460px, 60vh)' }}
        >
          {/* Header */}
          <div className="shrink-0 rounded-t-[inherit] p-2.5 border-b border-border-muted/60 bg-surface-0/60">
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
                      : prov === 'copilot'
                      ? 'GitHub Copilot'
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
          {nativeLoadError && (
            <div role="alert" className="mx-2 mt-2 rounded-md border border-rose-500/30 bg-rose-500/10 p-2 text-xs text-rose-300">
              {nativeLoadError}
            </div>
          )}
          <div className="min-h-0 overflow-y-auto p-1.5 space-y-2" style={{ maxHeight: '320px' }}>
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
                      activeUnifiedItem?.baseId === m.baseId;
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
                              const targetTop = Math.max(8, Math.min(rowRect.top - 28, window.innerHeight - (m.efforts.length * 32 + 16) - 8));
                              const targetLeft = menuRect.right + 134 < window.innerWidth ? menuRect.right + 6 : Math.max(8, menuRect.left - 134);
                              setHoveredEffortItem({
                                model: m,
                                top: targetTop,
                                left: targetLeft,
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
                          disabled={loadingNativeModel !== null}
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

                            {/* In RAM / Custom Badge */}
                            {m.badge && (
                              <span
                                className={`px-1.5 py-0.5 rounded text-[9px] font-sans border shrink-0 ${
                                  m.badge === 'Loaded'
                                    ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-400 font-medium'
                                    : 'border-zinc-700/60 bg-zinc-800/80 text-zinc-400'
                                }`}
                              >
                                {m.badge}
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
                            {(m.supportsEffort) && (
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
          {hoveredEffortItem && createPortal(
            <div
              ref={submenuRef}
              style={{
                position: 'fixed',
                top: `${hoveredEffortItem.top}px`,
                left: `${hoveredEffortItem.left}px`,
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
              <div className="space-y-0.5 max-h-[60vh] overflow-y-auto">

                {hoveredEffortItem.model.efforts.map((effort) => {
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
            </div>, document.body
          )}

          {/* Footer */}
          <div className="shrink-0 rounded-b-[inherit] p-2 border-t border-border-muted bg-surface-0/60 flex items-center justify-between text-[10px]">
            <button
              type="button"
              onClick={() => {
                handleSelect('', undefined, undefined);
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
                openSettings('runtime');
              }}
              className="text-teal-400 hover:text-teal-300 flex items-center gap-1 px-2 py-0.5 rounded hover:bg-surface-2 transition text-xs font-medium"
            >
              <Cpu size={11} />
              Local Models
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

