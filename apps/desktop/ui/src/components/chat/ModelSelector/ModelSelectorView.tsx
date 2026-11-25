import React, { useState, useRef, useEffect, useMemo } from 'react';
import { ChevronDown, Cpu, Search } from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import type { LoadedModelInstance, LocalModelDescriptor } from '@/types/domain';
import {
  type ModelSelectorProps,
  type ModelOption,
  type UnifiedModelItem,
  type ThinkingEffort,
  getProviderInfo,
  formatModelDisplayName,
  groupModelOptions,
  resolveModelRoute,
  compareModelPresentation,
} from './types';
import { ProviderIcon } from './ProviderIcon';
import { getConnectedModelOptions } from './catalog';
import { EffortSubmenu } from './EffortSubmenu';
import { ModelRow } from './ModelRow';
import {
  ModelSelectorDropdownFooter,
  ModelSelectorProviderPills,
} from './ModelSelectorDropdownControls';

export const ModelSelectorView: React.FC<ModelSelectorProps> = ({
  currentModel,
  onSelectModel,
  className = '',
}) => {
  const {
    openSettings,
    connectedAccounts,
    selectedModel: contextSelectedModel,
    selectedReasoningEffort,
    handleSelectModel: contextHandleSelectModel,
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
            <ModelSelectorProviderPills
              availableProviders={availableProviders}
              selectedProviderFilter={selectedProviderFilter}
              onSelectFilter={setSelectedProviderFilter}
            />

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
                  {group.models.map((m, mIdx) => (
                    <ModelRow
                      key={`${group.providerKey}-${m.baseId || m.id}-${mIdx}`}
                      model={m}
                      providerKey={group.providerKey}
                      index={mIdx}
                      isSelected={activeUnifiedItem?.baseId === m.baseId}
                      isHovered={hoveredBaseId === m.baseId}
                      loadingNativeModel={loadingNativeModel}
                      onHover={(e, model) => {
                        setHoveredBaseId(model.baseId);
                        if (model.supportsEffort) {
                          const rowRect = e.currentTarget.getBoundingClientRect();
                          const menuRect = menuRef.current?.getBoundingClientRect();
                          if (menuRect) {
                            const targetTop = Math.max(8, Math.min(rowRect.top - 28, window.innerHeight - (model.efforts.length * 32 + 16) - 8));
                            const targetLeft = menuRect.right + 134 < window.innerWidth ? menuRect.right + 6 : Math.max(8, menuRect.left - 134);
                            setHoveredEffortItem({
                              model,
                              top: targetTop,
                              left: targetLeft,
                            });
                          }
                        } else {
                          setHoveredEffortItem(null);
                        }
                      }}
                      onClick={handleDirectModelClick}
                    />
                  ))}
                </div>
              ))
            )}
          </div>

          {/* FLYOUT SUBMENU */}
          {hoveredEffortItem && (
            <EffortSubmenu
              hoveredEffortItem={hoveredEffortItem}
              onSelectEffort={handleSelectWithEffort}
              onMouseEnter={() => setHoveredBaseId(hoveredEffortItem.model.baseId)}
              onMouseLeave={() => {
                setHoveredEffortItem(null);
                setHoveredBaseId(null);
              }}
              submenuRef={submenuRef}
            />
          )}

          {/* Footer */}
          <ModelSelectorDropdownFooter
            onUnsetModel={() => {
              handleSelect('', undefined, undefined);
              setIsOpen(false);
              setHoveredBaseId(null);
              setHoveredEffortItem(null);
            }}
            onOpenLocalModels={() => {
              setIsOpen(false);
              setHoveredBaseId(null);
              setHoveredEffortItem(null);
              openSettings('runtime');
            }}
            onManageProviders={() => {
              setIsOpen(false);
              setHoveredBaseId(null);
              setHoveredEffortItem(null);
              openSettings('providers');
            }}
          />
        </div>
      )}
    </div>
  );
};
