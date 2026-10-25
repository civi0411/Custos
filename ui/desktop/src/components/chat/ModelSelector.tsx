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
} from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import type { LoadedModelInstance, LocalModelDescriptor } from '@/types/domain';
import type { ProviderServiceId } from '@/types/provider_types';

export type ThinkingEffort = string;

export interface ModelOption {
  accountId: string;
  id: string;
  name: string;
  provider: ProviderServiceId;
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
  provider: ProviderServiceId;
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
  provider: ProviderServiceId;
  label: string;
} {
  const m = (modelId || '').toLowerCase();
  if (m.startsWith('native:')) return { provider: 'local', label: 'Local (Native Metal)' };
  if (m.startsWith('local/') || m.startsWith('local:')) return { provider: 'local', label: 'Local (Apple Silicon)' };
  if (m.startsWith('ollama/')) return { provider: 'local', label: 'Local (Ollama)' };
  if (m.startsWith('lmstudio/') || m.includes('lmstudio')) return { provider: 'lmstudio', label: 'LM Studio' };
  if (m.startsWith('copilot/')) return { provider: 'copilot', label: 'GitHub Copilot' };
  if (m.startsWith('antigravity/')) return { provider: 'antigravity', label: 'Google Antigravity' };
  if (m.startsWith('groq/') || m.includes('groq')) return { provider: 'groq', label: 'Groq LPU' };
  if (m.startsWith('cerebras/') || m.includes('cerebras')) return { provider: 'cerebras', label: 'Cerebras' };
  if (m.startsWith('mistral/') || m.includes('codestral') || m.includes('mistral')) return { provider: 'mistral', label: 'Mistral AI' };
  if (m.startsWith('moonshot/') || m.includes('kimi')) return { provider: 'moonshot', label: 'Moonshot Kimi' };
  if (m.startsWith('zhipu/') || m.includes('glm')) return { provider: 'zhipu', label: 'Zhipu GLM' };
  if (m.startsWith('minimax/')) return { provider: 'minimax', label: 'MiniMax' };
  if (m.startsWith('alibaba/') || m.startsWith('qwen/') || m.includes('qwen')) return { provider: 'alibaba', label: 'Alibaba Qwen' };
  if (m.startsWith('nvidia/') || m.includes('nemotron')) return { provider: 'nvidia', label: 'NVIDIA NIM' };
  if (m.startsWith('together/')) return { provider: 'together', label: 'Together AI' };
  if (m.startsWith('xai/') || m.includes('grok')) return { provider: 'xai', label: 'xAI (Grok)' };
  if (m.startsWith('openrouter/')) return { provider: 'openrouter', label: 'OpenRouter' };
  if (m.startsWith('perplexity/') || m.includes('sonar')) return { provider: 'perplexity', label: 'Perplexity' };
  if (m.startsWith('cohere/') || m.includes('command-r')) return { provider: 'cohere', label: 'Cohere' };
  if (m.startsWith('sambanova/')) return { provider: 'sambanova', label: 'SambaNova' };
  if (m.startsWith('fireworks/')) return { provider: 'fireworks', label: 'Fireworks AI' };
  if (m.startsWith('siliconflow/')) return { provider: 'siliconflow', label: 'SiliconFlow' };
  if (m.includes('claude') || m.includes('anthropic')) return { provider: 'anthropic', label: 'Anthropic Claude' };
  if (m.includes('gpt') || m.includes('o1') || m.includes('o3') || m.includes('openai') || m.includes('codex')) return { provider: 'openai', label: 'OpenAI / Codex' };
  if (m.includes('gemini') || m.includes('google')) return { provider: 'gemini', label: 'Google Gemini' };
  if (m.includes('deepseek')) return { provider: 'deepseek', label: 'DeepSeek' };
  if (m.includes('qwopus') || m.includes('llama') || m.includes('local') || m.includes('ollama')) {
    return { provider: 'local', label: 'Local (Apple Silicon)' };
  }
  return { provider: 'custom', label: 'Custom Gateway' };
}

export function ProviderIcon({
  provider,
  size = 14,
  className
}: {
  provider: ProviderServiceId;
  size?: number;
  className?: string;
}): React.JSX.Element {
  return <ProviderBrandIcon provider={provider} size={size} className={className} />;
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
  const isGoogle = provider === 'antigravity' || provider === 'gemini';
  const suffix = isGoogle && !advertisedLabel
    ? id.match(/-(extra-low|none|minimal|low|medium|high|xhigh|max|ultra)$/i) : null;
  const variantEffort = (annotation?.[1] || suffix?.[1])?.toLowerCase();
  const name = annotation ? advertisedLabel!.slice(0, annotation.index).trim()
    : suffix ? formatModelDisplayName(id.slice(0, suffix.index))
    : advertisedLabel || formatModelDisplayName(id);
  return { family: isGoogle || annotation ? name.toLowerCase() : id, name, variantEffort };
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
    if (account.status !== 'active' && account.statusLabel !== 'Connected') continue;

    const modelIds = (account.supportedModels && account.supportedModels.length > 0)
      ? account.supportedModels
      : account.catalogModels?.map((m) => m.id) || [];

    for (const id of modelIds) {
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
        familyKey: JSON.stringify([(provider === 'antigravity' || provider === 'gemini') ? 'antigravity' : provider, presentation.family]),
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
          contextWindow: m.context_length ? `${m.context_length}` : undefined,
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

  const activeUnifiedItem = useMemo(() => {
    if (!activeModelId) return unifiedModels[0] || null;
    return (
      unifiedModels.find((model) => model.routes.some((route) => route.id === activeModelId)) ||
      unifiedModels.find((model) => model.id === activeModelId || model.name.toLowerCase() === activeModelId.toLowerCase()) ||
      unifiedModels[0] ||
      null
    );
  }, [unifiedModels, activeModelId]);

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
      setLoadingNativeModel(model.id);
      try {
        await daemonClient.ensureLocalModel(filePath);
        void refreshLoadedInstances();
      } catch (error) {
        setNativeLoadError(error instanceof Error ? error.message : String(error));
        return;
      } finally {
        setLoadingNativeModel(null);
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
        title={
          loadingNativeModel
            ? 'Loading local model…'
            : !hasModel
            ? 'No model selected'
            : activeUnifiedItem && new Set(activeUnifiedItem.routes.map((r) => r.accountId)).size > 1
            ? `Active model: ${activeModelId} (${new Set(activeUnifiedItem.routes.map((r) => r.accountId)).size} accounts pooled with 429 failover)`
            : `Active model: ${activeModelId}`
        }
      >
        {!hasModel && !activeUnifiedItem ? (
          <Cpu size={13} className="shrink-0 text-amber-400" />
        ) : (
          <ProviderIcon provider={activeUnifiedItem?.provider || getProviderInfo(activeModelId).provider} size={13} className="shrink-0" />
        )}

        <span className="font-mono text-[11px] truncate max-w-[170px] text-fg-editor flex items-center gap-1">
          {loadingNativeModel ? (
            'Loading local model…'
          ) : activeUnifiedItem ? (
            <>
              <span className="truncate">{activeUnifiedItem.name}</span>
              {activeUnifiedItem.supportsEffort && (
                <span className="text-[10px] text-zinc-400 capitalize opacity-90 font-sans shrink-0">
                  {activeUnifiedItem.selectedEffort}
                </span>
              )}
              {new Set(activeUnifiedItem.routes.map((r) => r.accountId)).size > 1 && (
                <span
                  className="px-1 py-0.2 rounded text-[9px] bg-purple-500/20 text-purple-300 border border-purple-500/40 font-sans font-medium shrink-0"
                  title={`${new Set(activeUnifiedItem.routes.map((r) => r.accountId)).size} accounts pooled with 429 failover`}
                >
                  {new Set(activeUnifiedItem.routes.map((r) => r.accountId)).size}x
                </span>
              )}
            </>
          ) : activeModelId ? (
            formatModelDisplayName(activeModelId)
          ) : (
            'Select Model'
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
          className="app-dropdown-menu absolute bottom-full mb-2 right-0 z-50 flex flex-col w-80 md:w-88 rounded-xl border border-border-default bg-[#1e1e1e]/98 backdrop-blur-2xl shadow-2xl overflow-visible animate-in fade-in slide-in-from-bottom-2 duration-150"
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

            {/* Provider Filter Tabs */}
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
                  {group.models.map((m, mIdx) => {
                    const isSelected =
                      activeUnifiedItem?.baseId === m.baseId;
                    const isHovered = hoveredBaseId === m.baseId;

                    return (
                      <div
                        key={`${group.providerKey}-${m.baseId || m.id}-${mIdx}`}
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

                            {/* Multi-account Pool Badge */}
                            {new Set(m.routes.map((r) => r.accountId)).size > 1 && (
                              <span
                                className="px-1.5 py-0.5 rounded text-[9px] font-sans border border-purple-500/40 bg-purple-500/15 text-purple-300 font-medium shrink-0 flex items-center gap-1"
                                title={`${new Set(m.routes.map((r) => r.accountId)).size} accounts pooled with automatic 429 failover`}
                              >
                                <span className="w-1.5 h-1.5 rounded-full bg-purple-400 animate-pulse" />
                                {new Set(m.routes.map((r) => r.accountId)).size}x Pool
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
                            {/* Chevron Right for Effort Submenu (3 effort levels) */}
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

