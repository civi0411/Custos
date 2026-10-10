import React, { createContext, useContext, useState, useEffect, useRef, useCallback } from 'react';
import {
  MainTab,
  ProjectData,
  ProviderItem,
  ClientApiKey,
  Session,
  ModelCombo,
  UsageRecord,
  ConnectedAccount,
  ProviderServiceId,
} from '../types';
import { daemonClient } from '../api/daemon_client';
import {
  ModelCatalogOption,
  OAuthTokenRecord,
  ProviderConfigRecord,
  Task,
  SessionJournalEntry,
} from '../types/domain';

interface AppContextType {
  // Projects & Sessions
  projectData: ProjectData;
  setProjectData: React.Dispatch<React.SetStateAction<ProjectData>>;
  currentProject: string;
  setCurrentProject: (name: string) => void;
  activeSessionId: string;
  setActiveSessionId: (id: string) => void;
  activeSession: Session | null;
  currentSessions: Session[];
  tasks: Task[];

  // Navigation & Layout
  currentTab: MainTab;
  setCurrentTab: (tab: MainTab) => void;
  isSessionsCollapsed: boolean;
  setIsSessionsCollapsed: React.Dispatch<React.SetStateAction<boolean>>;
  isRightExplorerOpen: boolean;
  setIsRightExplorerOpen: React.Dispatch<React.SetStateAction<boolean>>;
  toggleRightExplorer: () => void;

  // Providers & Keys
  providers: ProviderItem[];
  setProviders: React.Dispatch<React.SetStateAction<ProviderItem[]>>;
  clientKeys: ClientApiKey[];
  setClientKeys: React.Dispatch<React.SetStateAction<ClientApiKey[]>>;

  // Routing Combos & Usage (OmniRoute / 9Router style)
  combos: ModelCombo[];
  setCombos: React.Dispatch<React.SetStateAction<ModelCombo[]>>;
  handleCreateCombo: (combo: Omit<ModelCombo, 'id' | 'createdAt'>) => void;
  handleUpdateCombo: (id: string, updates: Partial<ModelCombo>) => void;
  handleDeleteCombo: (id: string) => void;
  usageRecords: UsageRecord[];
  handleResetUsage: () => void;
  connectedAccounts: ConnectedAccount[];
  setConnectedAccounts: React.Dispatch<React.SetStateAction<ConnectedAccount[]>>;
  handleConnectOAuth: (providerId: ProviderServiceId, accountName: string, email?: string) => Promise<void>;
  handleDeleteAccount: (accountId: string) => void;

  // Modals
  isSettingsOpen: boolean;
  setIsSettingsOpen: (open: boolean) => void;
  settingsTab: string;
  setSettingsTab: (tab: string) => void;
  openSettings: (tab?: string) => void;
  isNewSessionOpen: boolean;
  setIsNewSessionOpen: (open: boolean) => void;
  isAddProviderOpen: boolean;
  setIsAddProviderOpen: (open: boolean) => void;

  // UI Scale
  uiScale: number;
  handleSetUiScale: (val: number) => void;
  handleStepUiScale: (delta: number) => void;

  // Toast
  toastMessage: string | null;
  showToast: (msg: string) => void;

  // Selected Model
  selectedModel: string;
  handleSelectModel: (model: string) => void;

  // Actions
  handleSendMessage: (text: string) => Promise<void>;
  handleClearHistory: () => void;
  handleDeleteConversation: (sessionId: string) => Promise<void>;
  handleAcceptAndRun: () => Promise<void>;
  handleRejectDiff: () => void;
  handleCopyDiff: () => void;
  handleCreateNewSession: (title: string, pack: 'engineering' | 'research' | 'assistant') => Promise<void>;
  handleNewProjectPrompt: () => void;
  handleSaveProvider: (
    serviceOrParams: string | { service: string; apiKey?: string; endpointUrl?: string; model?: string; contextWindow?: number; fastMode?: boolean },
    apiKey?: string
  ) => Promise<void> | void;
  handleDeleteProvider: (id: string) => Promise<void> | void;
  handleTestConnection: (providerNameOrId: string) => Promise<void> | void;
  handleGenerateClientKey: () => Promise<void> | void;
  handleRevokeClientKey: (id: string) => Promise<void> | void;
}

function mapBackendProvider(p: any): ProviderItem {
  const pType = (p.service_type || p.provider_type || '').toLowerCase();
  const iconType = (
    pType.includes('claude') || pType.includes('anthropic') ? 'anthropic' :
    pType.includes('openai') || pType.includes('gpt') ? 'openai' :
    pType.includes('gemini') ? 'gemini' :
    'deepseek'
  ) as ProviderItem['iconType'];

  const status = (['primary', 'standby', 'failover', 'offline'].includes(p.status)
    ? p.status
    : p.status === 'configured' || p.status === 'active' || p.is_active ? 'primary' : 'offline') as ProviderItem['status'];

  const modelName = p.default_model || p.model || 'Model not reported';

  return {
    id: p.id,
    name: p.name,
    model: modelName,
    status,
    statusLabel: p.status === 'configured' ? 'Configured & Active' : p.status_label || (status === 'primary' ? 'Verified & Active' : 'Offline'),
    badgeColor: status === 'primary' ? '#10b981' : status === 'standby' ? '#3b82f6' : '#6b7280',
    apiKey: p.api_key_masked || (p.api_key && p.api_key.trim().length > 0 ? `${p.api_key.slice(0, 6)}••••••••` : 'Chưa cấu hình'),
    quotaUsed: p.quota_used || undefined,
    quotaTotal: p.quota_total || undefined,
    quotaPercent: typeof p.quota_percent === 'number' ? p.quota_percent : undefined,
    endpoint: p.endpoint_url || p.endpoint || undefined,
    defaultModel: p.default_model || p.model || undefined,
    contextWindow: p.context_window || undefined,
    fastMode: p.fast_mode || undefined,
    latency: p.latency_ms ? `${p.latency_ms}ms` : '32ms',
    iconType,
  };
}

function mapBackendKey(k: any): ClientApiKey {
  let createdDate = new Date().toISOString().split('T')[0];
  if (k.created_at) {
    const epoch = typeof k.created_at === 'number' && k.created_at < 1e11 ? k.created_at * 1000 : k.created_at;
    createdDate = new Date(epoch).toISOString().split('T')[0];
  }
  return {
    id: k.id,
    name: k.name,
    token: k.token,
    created: createdDate,
    icon: (k.name || '').toLowerCase().includes('cli') ? 'terminal' : 'laptop',
  };
}

const PROVIDER_LABELS: Record<ProviderServiceId, string> = {
  copilot: 'GitHub Copilot',
  gemini: 'Google Gemini',
  openai: 'OpenAI Platform',
  anthropic: 'Anthropic Claude',
  deepseek: 'DeepSeek',
  openrouter: 'OpenRouter Gateway',
  groq: 'Groq Cloud',
  local: 'Local Inference',
  custom: 'Custom Gateway',
};

const normalizeProviderServiceId = (service?: string): ProviderServiceId => {
  const s = (service || '').toLowerCase();
  if (s.includes('anthropic') || s.includes('claude')) return 'anthropic';
  if (s.includes('openai') || s.includes('gpt') || s.includes('codex')) return 'openai';
  if (s.includes('gemini') || s.includes('google')) return 'gemini';
  if (s.includes('deepseek')) return 'deepseek';
  if (s.includes('openrouter')) return 'openrouter';
  if (s.includes('groq')) return 'groq';
  if (s.includes('local') || s.includes('ollama')) return 'local';
  return 'custom';
};

const defaultProbeBaseUrl = (service: ProviderServiceId): string | undefined => {
  switch (service) {
    case 'openai':
      return 'https://api.openai.com/v1';
    case 'anthropic':
      return 'https://api.anthropic.com/v1';
    case 'gemini':
      return 'https://generativelanguage.googleapis.com';
    case 'deepseek':
      return 'https://api.deepseek.com/v1';
    case 'local':
      return 'http://localhost:11434';
    default:
      return undefined;
  }
};

const isProviderConnected = (p: ProviderConfigRecord): boolean => {
  const status = (p.status || '').toLowerCase();
  const statusConnected =
    status === 'configured' ||
    status === 'active' ||
    status === 'primary' ||
    status === 'standby';
  const localProvider = normalizeProviderServiceId(p.service_type) === 'local';
  return statusConnected && (localProvider || p.api_key_masked !== 'none');
};

const safeIsoFromMillis = (value?: number): string => {
  if (!value || value > 253402300799000) {
    return new Date().toISOString();
  }
  return new Date(value).toISOString();
};

const tokenLooksOauth = (token?: OAuthTokenRecord | null): boolean =>
  Boolean(token?.has_refresh_token || token?.refresh_token || token?.scope?.includes('openid'));

const uniqueModels = (models: string[]): string[] => {
  const seen = new Set<string>();
  return models.filter((model) => {
    const key = model.trim().toLowerCase();
    if (!key || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
};

const mergeCatalogModels = (
  primary: ModelCatalogOption[],
  fallback: ModelCatalogOption[]
): ModelCatalogOption[] => {
  const seen = new Set<string>();
  const merged: ModelCatalogOption[] = [];
  for (const model of [...primary, ...fallback]) {
    const key = model.id.trim().toLowerCase();
    if (!key || seen.has(key)) continue;
    seen.add(key);
    merged.push(model);
  }
  return merged;
};

const DEFAULT_COMBOS: ModelCombo[] = [
  {
    id: 'combo-fast-coding',
    name: 'Primary Coding + Fallback',
    description: 'Claude 3.7 Sonnet as primary, failing over to GPT-4o if rate limited or offline.',
    strategy: 'fallback',
    enabled: true,
    createdAt: new Date().toISOString(),
    targets: [
      {
        modelId: 'claude-3-7-sonnet',
        modelName: 'Claude 3.7 Sonnet',
        providerId: 'anthropic',
        providerName: 'Anthropic',
        priority: 1,
      },
      {
        modelId: 'gpt-4o',
        modelName: 'GPT-4o',
        providerId: 'openai',
        providerName: 'OpenAI',
        priority: 2,
      },
      {
        modelId: 'deepseek-chat',
        modelName: 'DeepSeek Chat',
        providerId: 'deepseek',
        providerName: 'DeepSeek',
        priority: 3,
      },
    ],
  },
  {
    id: 'combo-round-robin',
    name: 'Multi-Model Round Robin',
    description: 'Distributes load evenly across Gemini 2.5 Flash and GPT-4o-mini.',
    strategy: 'round_robin',
    enabled: true,
    createdAt: new Date().toISOString(),
    targets: [
      {
        modelId: 'gemini-2.5-flash',
        modelName: 'Gemini 2.5 Flash',
        providerId: 'gemini',
        providerName: 'Google Gemini',
        priority: 1,
        weight: 50,
      },
      {
        modelId: 'gpt-4o-mini',
        modelName: 'GPT-4o Mini',
        providerId: 'openai',
        providerName: 'OpenAI',
        priority: 2,
        weight: 50,
      },
    ],
  },
];

const DEFAULT_USAGE_RECORDS: UsageRecord[] = [
  {
    id: 'usage-1',
    modelId: 'claude-3-7-sonnet',
    modelName: 'Claude 3.7 Sonnet',
    providerId: 'anthropic',
    providerName: 'Anthropic',
    accountName: 'Production Org',
    inputTokens: 142500,
    outputTokens: 38200,
    totalTokens: 180700,
    estimatedCostUsd: 1.02,
    requestCount: 48,
    quotaLimitTokens: 1000000,
    quotaUsedTokens: 180700,
    quotaPercent: 18,
    rateLimitStatus: 'normal',
    resetPeriod: 'Resets daily at 00:00 UTC',
    lastUsedAt: 'Just now',
  },
  {
    id: 'usage-2',
    modelId: 'gpt-4o',
    modelName: 'GPT-4o',
    providerId: 'openai',
    providerName: 'OpenAI',
    accountName: 'Personal Key',
    inputTokens: 215400,
    outputTokens: 41600,
    totalTokens: 257000,
    estimatedCostUsd: 0.95,
    requestCount: 62,
    quotaLimitTokens: 500000,
    quotaUsedTokens: 257000,
    quotaPercent: 51,
    rateLimitStatus: 'normal',
    resetPeriod: 'Resets daily at 00:00 UTC',
    lastUsedAt: '12m ago',
  },
  {
    id: 'usage-3',
    modelId: 'gemini-2.5-flash',
    modelName: 'Gemini 2.5 Flash',
    providerId: 'gemini',
    providerName: 'Google Gemini',
    accountName: 'Free Tier OAuth',
    inputTokens: 820000,
    outputTokens: 120000,
    totalTokens: 940000,
    estimatedCostUsd: 0.14,
    requestCount: 155,
    quotaLimitTokens: 1000000,
    quotaUsedTokens: 940000,
    quotaPercent: 94,
    rateLimitStatus: 'throttled',
    resetPeriod: 'Resets daily at 00:00 UTC',
    lastUsedAt: '4m ago',
  },
  {
    id: 'usage-4',
    modelId: 'deepseek-chat',
    modelName: 'DeepSeek Chat',
    providerId: 'deepseek',
    providerName: 'DeepSeek',
    accountName: 'DeepSeek Dev',
    inputTokens: 64000,
    outputTokens: 18000,
    totalTokens: 82000,
    estimatedCostUsd: 0.03,
    requestCount: 19,
    quotaLimitTokens: 10000000,
    quotaUsedTokens: 82000,
    quotaPercent: 1,
    rateLimitStatus: 'normal',
    resetPeriod: 'Prepaid Balance',
    lastUsedAt: '2h ago',
  },
];

const AppContext = createContext<AppContextType | null>(null);

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // Projects & Sessions state
  const [projectData, setProjectData] = useState<ProjectData>({ 'Custos Workspace': [] });
  const [currentProject, setCurrentProject] = useState<string>('Custos Workspace');
  const [activeSessionId, setActiveSessionId] = useState<string>('');
  const [tasks, setTasks] = useState<Task[]>([]);

  // Navigation & Layout state
  const [currentTab, setCurrentTab] = useState<MainTab>('studio');
  const [isSessionsCollapsed, setIsSessionsCollapsed] = useState<boolean>(false);
  const [isRightExplorerOpen, setIsRightExplorerOpen] = useState<boolean>(true);

  const toggleRightExplorer = useCallback(() => {
    setIsRightExplorerOpen((prev) => !prev);
  }, []);

  // Providers & Keys state
  const [providers, setProviders] = useState<ProviderItem[]>([]);
  const [clientKeys, setClientKeys] = useState<ClientApiKey[]>([]);

  // Combos & Usage state (OmniRoute / 9Router style)
  const [combos, setCombos] = useState<ModelCombo[]>(() => {
    try {
      const saved = localStorage.getItem('custos_combos');
      return saved ? JSON.parse(saved) : DEFAULT_COMBOS;
    } catch {
      return DEFAULT_COMBOS;
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_combos', JSON.stringify(combos));
    } catch (e) {
      console.warn('Failed to save combos to localStorage', e);
    }
  }, [combos]);

  const [connectedAccounts, setConnectedAccounts] = useState<ConnectedAccount[]>(() => {
    return [];
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_connected_accounts', JSON.stringify(connectedAccounts));
    } catch (e) {
      console.warn('Failed to save connected accounts to localStorage', e);
    }
  }, [connectedAccounts]);

  const [usageRecords, setUsageRecords] = useState<UsageRecord[]>(() => {
    try {
      const saved = localStorage.getItem('custos_usage_records');
      return saved ? JSON.parse(saved) : DEFAULT_USAGE_RECORDS;
    } catch {
      return DEFAULT_USAGE_RECORDS;
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_usage_records', JSON.stringify(usageRecords));
    } catch (e) {
      console.warn('Failed to save usage records to localStorage', e);
    }
  }, [usageRecords]);

  // Modals state
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [settingsTab, setSettingsTab] = useState<string>('general');
  const [isNewSessionOpen, setIsNewSessionOpen] = useState<boolean>(false);
  const [isAddProviderOpen, setIsAddProviderOpen] = useState<boolean>(false);

  const openSettings = useCallback((tab?: string) => {
    if (tab) setSettingsTab(tab);
    setIsSettingsOpen(true);
  }, []);

  // UI Scale state
  const [uiScale, setUiScale] = useState<number>(() => {
    const saved = localStorage.getItem('custos_ui_scale');
    return saved ? parseInt(saved, 10) : 100;
  });

  // Toast state
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const toastTimeoutRef = useRef<number | null>(null);

  const showToast = useCallback((msg: string) => {
    if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current);
    setToastMessage(msg);
    toastTimeoutRef.current = window.setTimeout(() => {
      setToastMessage(null);
    }, 2500);
  }, []);

  // UI Scale applicator - zoom the entire document root so the full screen/window scales uniformly
  const applyUiScale = useCallback((scaleVal: number) => {
    const rounded = Math.round(scaleVal / 10) * 10;
    const clamped = Math.min(150, Math.max(70, rounded));
    setUiScale(clamped);
    localStorage.setItem('custos_ui_scale', clamped.toString());
    document.documentElement.style.zoom = `${clamped / 100}`;
    document.body.style.zoom = '';
  }, []);

  const handleSetUiScale = (val: number) => {
    applyUiScale(val);
    showToast(`UI Scale: ${val}%`);
  };

  const handleStepUiScale = (delta: number) => {
    handleSetUiScale(uiScale + delta);
  };

  // Check active session object
  const currentSessions = projectData[currentProject] || [];
  const activeSession: Session | null =
    currentSessions.find((s) => s.id === activeSessionId) || currentSessions[0] || null;

  // Selected Model state: default empty string means "No model selected"
  const [selectedModel, setSelectedModel] = useState<string>(() => {
    return localStorage.getItem('custos_selected_model') || '';
  });

  const refreshProviderState = useCallback(async () => {
    const [fetchedProviders, fetchedKeys] = await Promise.all([
      daemonClient.listProviders().catch(() => []),
      daemonClient.listClientKeys().catch(() => []),
    ]);

    let providerRecords = fetchedProviders as ProviderConfigRecord[];
    let repairedOAuthProvider = false;

    const recoverableOAuthProviders: ProviderServiceId[] = ['openai', 'anthropic', 'gemini', 'deepseek'];
    for (const providerId of recoverableOAuthProviders) {
      const hasProvider = providerRecords.some((provider) =>
        provider.id === providerId ||
        normalizeProviderServiceId(provider.service_type || provider.id) === providerId
      );
      if (hasProvider) {
        continue;
      }

      const token = await daemonClient.getOAuthToken(providerId).catch(() => null);
      if (!token?.connected && !token?.access_token) {
        continue;
      }

      await daemonClient.saveProvider({
        id: providerId,
        name: `${PROVIDER_LABELS[providerId]} (OAuth)`,
        service_type: providerId,
        provider_type: providerId,
        api_key_masked: 'oauth',
        endpoint_url: defaultProbeBaseUrl(providerId),
        status: 'active',
        status_label: 'OAuth Active',
        fast_mode: false,
        is_active: true,
      }).catch(() => undefined);
      repairedOAuthProvider = true;
    }

    if (repairedOAuthProvider) {
      providerRecords = (await daemonClient.listProviders().catch(() => providerRecords)) as ProviderConfigRecord[];
    }

    setProviders(providerRecords.map(mapBackendProvider));
    setClientKeys(fetchedKeys.map(mapBackendKey));

    const accounts = await Promise.all(
      providerRecords.map(async (provider): Promise<ConnectedAccount | null> => {
        const serviceId = normalizeProviderServiceId(provider.service_type || provider.id);
        const connected = isProviderConnected(provider);
        const endpointUrl = provider.endpoint_url || defaultProbeBaseUrl(serviceId);

        let tokenMeta: OAuthTokenRecord | null = null;
        tokenMeta = await daemonClient.getOAuthToken(provider.id).catch(() => null);
        if (!tokenMeta && provider.id !== serviceId) {
          tokenMeta = await daemonClient.getOAuthToken(serviceId).catch(() => null);
        }

        let catalogModels: ModelCatalogOption[] = [];
        if (connected && endpointUrl) {
          try {
            const probed = await daemonClient.probeModels(
              endpointUrl,
              provider.service_type || serviceId,
              undefined,
              provider.id
            );
            catalogModels = probed.models.map((model, index) => ({
              id: model.id,
              label: model.name || model.id,
              description: model.description,
              provider_type: provider.service_type || serviceId,
              is_default: index === 0,
              default_effort: undefined,
              efforts: [],
              supports_fast_mode: true,
              context_window: model.context_window || provider.context_window,
              pricing: undefined,
            }));
          } catch (err) {
            console.warn(`[ProviderHydrate] Live model probe failed for ${provider.id}:`, err);
          }
        }

        try {
          const providerCatalog = await daemonClient.getModelCatalog(provider.service_type || serviceId, provider.id);
          const canonicalCatalog =
            providerCatalog.origin === 'catalog'
              ? providerCatalog
              : await daemonClient.getModelCatalog(provider.service_type || serviceId);
          catalogModels = mergeCatalogModels(catalogModels, [
            ...(providerCatalog.models || []),
            ...(canonicalCatalog.models || []),
          ]);
        } catch {
          catalogModels = catalogModels;
        }

        const supportedModels = uniqueModels(catalogModels.map((model) => model.id));

        if (!connected) {
          return null;
        }

        const defaultModel =
          provider.default_model && supportedModels.includes(provider.default_model)
            ? provider.default_model
            : supportedModels[0] || '';
        const contextWindow =
          catalogModels.find((model) => model.id === defaultModel)?.context_window ||
          provider.context_window ||
          catalogModels[0]?.context_window;
        const authType = tokenLooksOauth(tokenMeta) || provider.name.toLowerCase().includes('oauth')
          ? 'oauth'
          : 'api_key';

        return {
          id: provider.id,
          providerId: serviceId,
          providerName: PROVIDER_LABELS[serviceId] || provider.name,
          accountName: provider.name || PROVIDER_LABELS[serviceId] || provider.id,
          authType,
          apiKeyMasked: authType === 'api_key' ? provider.api_key_masked : undefined,
          endpointUrl,
          oauthTier: authType === 'oauth' ? 'Connected via PKCE' : undefined,
          oauthExpiresAt: tokenMeta?.expires_at ? safeIsoFromMillis(tokenMeta.expires_at) : undefined,
          status: connected ? 'active' : 'offline',
          statusLabel: connected ? 'Connected' : 'Offline',
          badgeColor: connected ? '#10b981' : '#6b7280',
          defaultModel,
          supportedModels,
          contextWindow,
          fastMode: provider.fast_mode,
          latencyMs: 0,
          createdAt: safeIsoFromMillis(provider.created_at),
          lastUsedAt: connected ? 'Connected' : undefined,
        };
      })
    );

    const liveAccounts = accounts.filter((account): account is ConnectedAccount => account !== null);
    setConnectedAccounts(liveAccounts);

    const liveModelIds = new Set(
      liveAccounts.flatMap((account) => account.supportedModels).map((model) => model.toLowerCase())
    );
    const fallbackModel =
      liveAccounts.find((account) => account.status === 'active' && account.defaultModel)?.defaultModel ||
      liveAccounts.flatMap((account) => account.supportedModels)[0] ||
      '';

    setSelectedModel((previous) => {
      const current = previous || localStorage.getItem('custos_selected_model') || '';
      if (current && liveModelIds.has(current.toLowerCase())) {
        return current;
      }
      localStorage.setItem('custos_selected_model', fallbackModel);
      return fallbackModel;
    });

    return { providers: providerRecords, keys: fetchedKeys, accounts: liveAccounts };
  }, []);

  const handleSelectModel = useCallback((modelName: string) => {
    setSelectedModel(modelName);
    localStorage.setItem('custos_selected_model', modelName);
    if (activeSessionId) {
      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        const updatedList = list.map((s) => {
          if (s.id === activeSessionId) {
            return {
              ...s,
              model: modelName,
            };
          }
          return s;
        });
        return {
          ...prev,
          [currentProject]: updatedList,
        };
      });
    }
    showToast(modelName ? `Active Model: ${modelName}` : 'No model selected');
  }, [activeSessionId, currentProject, showToast]);

  // Synchronize activeSession model only when it still belongs to a live account catalog.
  useEffect(() => {
    const sessionModel = activeSession?.model || '';
    const sessionModelIsLive = connectedAccounts.some((account) =>
      account.status === 'active' &&
      account.supportedModels.some((model) => model.toLowerCase() === sessionModel.toLowerCase())
    );
    if (
      sessionModel &&
      sessionModelIsLive &&
      sessionModel !== 'Model not reported' &&
      sessionModel !== 'No model selected' &&
      sessionModel !== selectedModel
    ) {
      setSelectedModel(sessionModel);
    }
  }, [activeSessionId, activeSession?.model, connectedAccounts, selectedModel]);

  // Hydrate state from Custos Daemon on Mount
  useEffect(() => {
    applyUiScale(uiScale);

    async function hydrateFromDaemon() {
      try {
        const isOnline = await daemonClient.checkHealth();
        if (!isOnline) {
          showToast('Custos daemon is offline. No simulated data was loaded.');
          return;
        }

        const [fetchedTasks, fetchedSessions] = await Promise.all([
          daemonClient.listTasks().catch(() => []),
          daemonClient.listSessions().catch(() => []),
        ]);

        await refreshProviderState();

        const journals = await Promise.all(
          fetchedSessions.map(async (session) => {
            try {
              return [session.id, await daemonClient.getSessionJournal(session.id)] as const;
            } catch {
              return [session.id, [] as SessionJournalEntry[]] as const;
            }
          })
        );
        const journalBySession = new Map(journals);

        if (fetchedTasks.length > 0) {
          setTasks(fetchedTasks);

          // Project daemon tasks and sessions into UI sessions
          const projectedSessions: Session[] = fetchedTasks.flatMap((t) => {
            const domainSession = fetchedSessions.find((s) => (s.task_id || s.attached_to) === t.id);
            if (!domainSession) {
              return [];
            }
            const taskId = t.id;
            const sessionId = domainSession.id;
            const rawJournal = journalBySession.get(domainSession.id) || domainSession.journal || [];
            const journalList: any[] = Array.isArray(rawJournal)
              ? rawJournal
              : Array.isArray((rawJournal as any)?.entries)
              ? (rawJournal as any).entries
              : Array.isArray((rawJournal as any)?.journal)
              ? (rawJournal as any).journal
              : [];

            return [{
              id: sessionId,
              taskId,
              source: 'daemon',
              taskStatus: String(t.status),
              sessionId,
              pack: t.contract?.pack_id || t.contract?.pack,
              title: t.title,
              time: 'Live',
              preview: `Status: ${t.status} | Contract: ${t.contract?.pack || 'general'}`,
              model: (t.contract as any)?.model || selectedModel || '',
              fileName: '',
              diffHunk: '',
              diffLinesCount: '',
              summary: `Task ${t.id}: ${t.status}`,
              messages: journalList.map((j) => ({
                id: j.id || String(j.entry_id || `${sessionId}-${j.occurred_at}`),
                role: (j.role || j.entry_type) === 'user' ? 'user' : 'assistant',
                author: (j.role || j.entry_type) === 'user' ? 'You' : 'Custos Kernel',
                badge: j.badge,
                stepName: j.step_name,
                duration: j.duration,
                text: j.content || j.entry_data || '',
              })),
              diffCode: [],
            }];
          });

          setProjectData((prev) => ({
            ...prev,
            'Custos Workspace': projectedSessions,
          }));

          if (projectedSessions.length > 0) {
            setCurrentProject('Custos Workspace');
            setActiveSessionId(projectedSessions[0].id);
          }
        }
      } catch (err) {
        console.warn('[DaemonClient] Could not load data from daemon:', err);
      }
    }

    hydrateFromDaemon();
  }, []);

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Toggle settings (Ctrl+,)
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault();
        setIsSettingsOpen((prev) => !prev);
      }
      // Toggle Sessions Sidebar (Ctrl+B)
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        setIsSessionsCollapsed((prev) => !prev);
      }
      // UI Zoom In (Ctrl++ or Ctrl+=)
      if ((e.metaKey || e.ctrlKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault();
        handleStepUiScale(10);
      }
      // UI Zoom Out (Ctrl+-)
      if ((e.metaKey || e.ctrlKey) && (e.key === '-' || e.key === '_')) {
        e.preventDefault();
        handleStepUiScale(-10);
      }
      // Reset UI Scale (Ctrl+0)
      if ((e.metaKey || e.ctrlKey) && e.key === '0') {
        e.preventDefault();
        handleSetUiScale(100);
      }
      // Close open modals on Escape
      if (e.key === 'Escape') {
        setIsSettingsOpen(false);
        setIsNewSessionOpen(false);
        setIsAddProviderOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [uiScale]);

  // Actions
  const handleSendMessage = async (text: string) => {
    if (!activeSession) return;
    const userMsg = {
      role: 'user' as const,
      author: 'You',
      text,
    };

    // Keep the local draft visible while the request is pending.
    setProjectData((prev) => {
      const list = prev[currentProject] || [];
      const updatedList = list.map((s) => {
        if (s.id === activeSession.id) {
          return {
            ...s,
            messages: [...s.messages, userMsg],
          };
        }
        return s;
      });
      return {
        ...prev,
        [currentProject]: updatedList,
      };
    });

    const appendAssistantMessage = (assistantMsg: {
      role: 'assistant';
      author: string;
      badge?: string;
      stepName?: string;
      text: string;
    }) => {
      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        const updatedList = list.map((s) => {
          if (s.id === activeSession.id) {
            return {
              ...s,
              messages: [...s.messages, assistantMsg],
            };
          }
          return s;
        });
        return {
          ...prev,
          [currentProject]: updatedList,
        };
      });
    };

    try {
      const chosenModel = (selectedModel || activeSession.model || '').trim();
      const selectedCombo = combos.find((combo) => combo.id.toLowerCase() === chosenModel.toLowerCase());
      const liveModelIds = new Set(
        connectedAccounts
          .filter((account) => account.status === 'active')
          .flatMap((account) => account.supportedModels)
          .map((model) => model.toLowerCase())
      );

      if (!chosenModel) {
        appendAssistantMessage({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'Model Required',
          stepName: 'Model Gateway',
          text: 'Select a connected model before sending chat. Add an API key or complete PKCE in Providers, then choose one of the discovered models.',
        });
        showToast('Select a connected model first');
        return;
      }

      if (selectedCombo) {
        appendAssistantMessage({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'Concrete Model Required',
          stepName: 'Model Gateway',
          text: 'Routing combos are not dispatched as a single model turn yet. Select one concrete live model from a connected account.',
        });
        showToast('Select a concrete model, not a combo');
        return;
      }

      if (liveModelIds.size === 0 || !liveModelIds.has(chosenModel.toLowerCase())) {
        appendAssistantMessage({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'Model Not Connected',
          stepName: 'Model Gateway',
          text: `Model "${chosenModel}" is not in the live model catalog for any connected PKCE/API key account. Refresh or reconnect the provider, then select a discovered model.`,
        });
        showToast('Selected model is not connected');
        return;
      }

      // 1. Check if an LLM is currently available / configured
      const llmStatus = await daemonClient.checkLlmStatus();
      if (!llmStatus.configured) {
        const noLlmMsg = {
          role: 'assistant' as const,
          author: 'Custos Engine',
          badge: 'No LLM Available',
          stepName: 'LLM Gateway',
          text: `⚠️ Chưa có LLM nào được cấu hình hiện tại (No LLMs currently available).\n\n${llmStatus.message || 'Hệ thống đang hoạt động ở chế độ Sovereign & Offline Kernel. Vui lòng thêm API Key (Anthropic Claude, OpenAI Codex, Google Gemini...) hoặc chọn Local Model trong Cài đặt > Providers để kích hoạt AI reasoning.'}`,
        };

        setProjectData((prev) => {
          const list = prev[currentProject] || [];
          const updatedList = list.map((s) => {
            if (s.id === activeSession.id) {
              return {
                ...s,
                messages: [...s.messages, noLlmMsg],
              };
            }
            return s;
          });
          return {
            ...prev,
            [currentProject]: updatedList,
          };
        });

        showToast('Chưa có LLM nào được cấu hình hiện tại');
        return;
      }

      showToast('Dispatching run to Custos Spine...');

      // 2. Ensure a daemon session exists, but run each chat turn on its own task
      // so active worker claims on the source task do not fence conversation turns.
      let targetSessionId = activeSession.sessionId;
      let runTaskId = '';
      let createTaskError: unknown = null;

      if (!targetSessionId) {
        try {
          const createdSession = await daemonClient.createSession('assisted');
          targetSessionId = createdSession.id;
        } catch (e) {
          console.warn('[DaemonClient] Could not create session on daemon:', e);
        }
      }

      try {
        const createdTask = await daemonClient.createTask({
          title: `Chat turn: ${activeSession.title || chosenModel}`,
          metadata: {
            kind: 'conversation_chat_turn',
            pack_id: activeSession.pack || 'assistant',
            conversation_session_id: targetSessionId || activeSession.id,
            source_task_id: activeSession.taskId || activeSession.id,
            model: chosenModel,
          },
        });
        runTaskId = createdTask.id;
      } catch (e) {
        createTaskError = e;
        console.warn('[DaemonClient] Could not create chat turn task on daemon:', e);
      }

      if (!runTaskId) {
        appendAssistantMessage({
          role: 'assistant',
          author: 'Custos Kernel',
          badge: 'Task Required',
          stepName: 'Conversation Dispatch',
          text: `Could not create an isolated chat task for this turn: ${
            createTaskError instanceof Error ? createTaskError.message : String(createTaskError || 'unknown error')
          }`,
        });
        return;
      }

      // Append message to backend session journal
      if (targetSessionId) {
        await daemonClient.appendSessionMessage(targetSessionId, 'user', text);
      }

      // 3. Start workflow run with target model
      const run = await daemonClient.startRun({
        task_id: runTaskId,
        preferred_mode: 'model',
        session_id: targetSessionId,
        prompt: text,
        model: chosenModel,
      });

      const runId = (run as any).run_id || run.id || 'unknown';
      const outputText = (run as any).output || run.metadata?.output;
      const isCompleted = run.status === 'completed';

      const assistantMsg = {
        role: 'assistant' as const,
        author: chosenModel,
        badge: isCompleted ? 'Live Inference' : `Run ${run.status}`,
        stepName: `Run #${runId.slice(0, 8)}`,
        text: outputText
          ? outputText
          : `Run ${runId} was accepted with status ${run.status}. Awaiting background completion.`,
      };

      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        const updatedList = list.map((s) => {
          if (s.id === activeSession.id) {
            return {
              ...s,
              taskId: s.taskId,
              sessionId: targetSessionId || s.sessionId,
              messages: [...s.messages, assistantMsg],
            };
          }
          return s;
        });
        return {
          ...prev,
          [currentProject]: updatedList,
        };
      });

      showToast(`Run ${runId.slice(0, 8)}: ${run.status}`);
    } catch (err: any) {
      const errMsg = {
        role: 'assistant' as const,
        author: 'Custos Kernel',
        badge: 'Gate Fenced',
        text: `Execution notice: ${err?.message || err}`,
      };

      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        const updatedList = list.map((s) => {
          if (s.id === activeSession.id) {
            return {
              ...s,
              messages: [...s.messages, errMsg],
            };
          }
          return s;
        });
        return {
          ...prev,
          [currentProject]: updatedList,
        };
      });
    }
  };

  const handleClearHistory = () => {
    if (!activeSession) return;
    if (activeSession.source === 'daemon') {
      showToast('Clearing saved conversation history is not available yet.');
      return;
    }
    setProjectData((prev) => {
      const list = prev[currentProject] || [];
      const updatedList = list.map((s) => {
        if (s.id === activeSession.id) {
          return { ...s, messages: [] };
        }
        return s;
      });
      return { ...prev, [currentProject]: updatedList };
    });
    showToast('Demo conversation cleared in this window');
  };

  const handleDeleteConversation = useCallback(async (sessionId: string) => {
    const list = projectData[currentProject] || [];
    const targetIndex = list.findIndex((session) => session.id === sessionId);
    if (targetIndex < 0) {
      showToast('Conversation not found');
      return;
    }

    const target = list[targetIndex];
    const nextList = list.filter((session) => session.id !== sessionId);
    const fallbackSession = nextList[targetIndex] || nextList[targetIndex - 1] || nextList[0];

    if (target.source === 'daemon' && target.sessionId) {
      try {
        await daemonClient.deleteSession(target.sessionId);
      } catch (error) {
        showToast(`Could not delete session: ${error instanceof Error ? error.message : String(error)}`);
        return;
      }
    }

    setProjectData((prev) => ({
      ...prev,
      [currentProject]: nextList,
    }));

    if (activeSessionId === sessionId) {
      setActiveSessionId(fallbackSession?.id || '');
    }

    showToast(
      target.source === 'daemon'
        ? 'Session deleted from daemon'
        : 'Conversation deleted'
    );
  }, [activeSessionId, currentProject, projectData, showToast]);

  const handleAcceptAndRun = async () => {
    if (!activeSession) return;
    try {
      await daemonClient.advanceTask(activeSession.taskId || activeSession.id, 'queued');
      showToast('Task queued. The daemon will report the run state when it starts.');
    } catch (error) {
      showToast(`Could not advance task: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleRejectDiff = () => {
    showToast('No patch was changed. Rejection is not connected to the daemon yet.');
  };

  const handleCopyDiff = () => {
    if (!activeSession) return;
    const diffText = activeSession.diffCode.map((d) => d.text).join('\n');
    navigator.clipboard.writeText(diffText);
    showToast('Code diff copied to clipboard');
  };

  const handleCreateNewSession = async (title: string, pack: 'engineering' | 'research' | 'assistant') => {
    try {
      // 1. Create real Task in Custos Daemon
      const createdTask = await daemonClient.createTask({
        title: title || 'New Supervised Task',
        contract: {
          pack_id: pack,
          name: title || 'New Supervised Task',
          description: `${pack} conversation task`,
          required_capabilities: [],
          evidence_requirements: [],
        },
      });

      // 2. Create Session in Custos Daemon
      const createdSession = await daemonClient.createSession('assisted');
      await daemonClient.attachSession({ session_id: createdSession.id, task_id: createdTask.id });

      const newSessionItem: Session = {
        id: createdSession.id,
        taskId: createdTask.id,
        source: 'daemon',
        taskStatus: createdTask.status,
        sessionId: createdSession.id,
        pack,
        title: createdTask.title,
        time: 'Just now',
        preview: `${pack} task · ${createdTask.status}`,
        model: selectedModel || '',
        fileName: '',
        diffHunk: '',
        diffLinesCount: '',
        summary: `Created task ${createdTask.id}`,
        messages: [],
        diffCode: [],
      };

      setProjectData((prev) => {
        const currentList = prev[currentProject] || [];
        return {
          ...prev,
          [currentProject]: [newSessionItem, ...currentList],
        };
      });

      setActiveSessionId(createdSession.id);
      setIsNewSessionOpen(false);
      showToast(`Created Task ${createdTask.id.slice(0, 8)}`);

    } catch (err: any) {
      console.error('[CreateSession] Error:', err);
      showToast(`Error creating session: ${err?.message || err}`);
    }
  };

  const handleNewProjectPrompt = () => {
    const name = window.prompt('Enter new Project Name:');
    if (name && name.trim()) {
      const cleanName = name.trim();
      setProjectData((prev) => {
        if (prev[cleanName]) return prev;
        return {
          ...prev,
          [cleanName]: [],
        };
      });
      setCurrentProject(cleanName);
      showToast(`Created project: ${cleanName}`);
    }
  };

  const handleSaveProvider = async (
    serviceOrParams: string | { service: string; apiKey?: string; endpointUrl?: string; model?: string; contextWindow?: number; fastMode?: boolean },
    maybeApiKey?: string
  ) => {
    try {
      const isObj = typeof serviceOrParams === 'object' && serviceOrParams !== null;
      const service = isObj ? serviceOrParams.service : serviceOrParams;
      const apiKey = isObj ? (serviceOrParams.apiKey || '') : (maybeApiKey || '');
      const endpointUrl = isObj ? serviceOrParams.endpointUrl : undefined;
      const defaultModel = isObj ? serviceOrParams.model : undefined;
      const contextWindow = isObj ? serviceOrParams.contextWindow : undefined;
      const fastMode = isObj ? serviceOrParams.fastMode : undefined;

      const existing = providers.find((p) => p.name.toLowerCase().includes(service.toLowerCase()) || p.id === service);
      const providerId = existing?.id || service.toLowerCase().replace(/[^a-z0-9]/g, '_');
      const providerType =
        service.toLowerCase().includes('claude') || service.toLowerCase().includes('anthropic') ? 'anthropic' :
        service.toLowerCase().includes('openai') || service.toLowerCase().includes('gpt') ? 'openai' :
        service.toLowerCase().includes('gemini') ? 'gemini' :
        service.toLowerCase().includes('deepseek') ? 'deepseek' : 'local';
      const normalizedService = normalizeProviderServiceId(providerType);
      const resolvedEndpoint = endpointUrl?.trim() || undefined;
      const configured = Boolean(apiKey.trim() || resolvedEndpoint || normalizedService === 'local');

      await daemonClient.saveProvider({
        id: providerId,
        name: existing?.name || service,
        service_type: providerType,
        provider_type: providerType,
        model: defaultModel || existing?.model || '',
        default_model: defaultModel,
        api_key: apiKey,
        endpoint_url: resolvedEndpoint,
        context_window: contextWindow,
        fast_mode: fastMode,
        status: configured ? 'primary' : 'offline',
        status_label: configured ? 'Configured & Active' : 'Offline',
        latency_ms: 28,
        is_active: configured,
      });

      await refreshProviderState();
      setIsAddProviderOpen(false);
      showToast(`Provider ${service} đã được cập nhật thành công vào Backend SQLite!`);
    } catch (err: any) {
      console.error('[SaveProvider] Error:', err);
      showToast(`Lỗi lưu provider: ${err?.message || err}`);
    }
  };

  const handleDeleteProvider = async (id: string) => {
    try {
      await daemonClient.deleteProvider(id);
      await refreshProviderState();
      showToast(`Đã xóa provider ${id} khỏi hệ thống!`);
    } catch (err: any) {
      console.error('[DeleteProvider] Error:', err);
      showToast(`Lỗi xóa provider: ${err?.message || err}`);
    }
  };

  const handleTestConnection = async (providerNameOrId: string) => {
    try {
      const provider = providers.find((p) => p.id === providerNameOrId || p.name === providerNameOrId);
      if (!provider) {
        showToast(`Không tìm thấy provider ${providerNameOrId}`);
        return;
      }
      if (provider.endpoint) {
        const probeRes = await daemonClient.probeModels(provider.endpoint, provider.iconType, undefined, provider.id);
        await refreshProviderState();
        showToast(`✓ Đã kết nối endpoint ${provider.name}! Tìm thấy ${probeRes.count} models.`);
      } else {
        showToast(`✓ Đã xác minh kết nối provider ${provider.name} qua Daemon protocol.`);
      }
    } catch (err: any) {
      showToast(`Lỗi kết nối provider: ${err?.message || err}`);
    }
  };

  const handleGenerateClientKey = async () => {
    try {
      const name = window.prompt('Nhập tên Client Token mới (vd: CLI Integration, Microservice):', 'Custom API Key');
      if (!name) return;
      await daemonClient.generateClientKey(name);
      const updated = await daemonClient.listClientKeys();
      if (updated) {
        setClientKeys(updated.map(mapBackendKey));
      }
      showToast('Client Key mới đã được tạo và lưu vào SQLite DB!');
    } catch (err: any) {
      console.error('[GenerateKey] Error:', err);
      showToast(`Lỗi tạo key: ${err?.message || err}`);
    }
  };

  const handleRevokeClientKey = async (id: string) => {
    try {
      await daemonClient.revokeClientKey(id);
      const updated = await daemonClient.listClientKeys();
      if (updated) {
        setClientKeys(updated.map(mapBackendKey));
      }
      showToast(`Đã thu hồi Client Key ${id.slice(0, 8)}`);
    } catch (err: any) {
      console.error('[RevokeKey] Error:', err);
      showToast(`Lỗi thu hồi key: ${err?.message || err}`);
    }
  };

  // Combos & Usage Handlers (OmniRoute / 9Router style)
  const handleCreateCombo = useCallback((comboData: Omit<ModelCombo, 'id' | 'createdAt'>) => {
    const newCombo: ModelCombo = {
      ...comboData,
      id: `combo-${Date.now()}`,
      createdAt: new Date().toISOString(),
    };
    setCombos((prev) => [newCombo, ...prev]);
    showToast(`Routing Combo "${newCombo.name}" created!`);
  }, [showToast]);

  const handleUpdateCombo = useCallback((id: string, updates: Partial<ModelCombo>) => {
    setCombos((prev) =>
      prev.map((c) => (c.id === id ? { ...c, ...updates, updatedAt: new Date().toISOString() } : c))
    );
    showToast('Routing Combo updated');
  }, [showToast]);

  const handleDeleteCombo = useCallback((id: string) => {
    setCombos((prev) => prev.filter((c) => c.id !== id));
    if (selectedModel === id) {
      setSelectedModel('');
      localStorage.setItem('custos_selected_model', '');
    }
    showToast('Routing Combo deleted');
  }, [selectedModel, showToast]);

  const handleResetUsage = useCallback(() => {
    setUsageRecords((prev) =>
      prev.map((r) => ({
        ...r,
        inputTokens: 0,
        outputTokens: 0,
        totalTokens: 0,
        estimatedCostUsd: 0,
        requestCount: 0,
        quotaUsedTokens: 0,
        quotaPercent: 0,
        rateLimitStatus: 'normal' as const,
        lastUsedAt: 'Reset just now',
      }))
    );
    showToast('Usage metrics reset successfully');
  }, [showToast]);

  const ensureOAuthProviderRecord = useCallback(async (
    providerId: ProviderServiceId,
    accountName?: string
  ) => {
    const token = await daemonClient.getOAuthToken(providerId).catch(() => null);
    if (!token?.connected && !token?.access_token) {
      return;
    }

    const providerRecords = (await daemonClient.listProviders().catch(() => [])) as ProviderConfigRecord[];
    const serviceId = normalizeProviderServiceId(providerId);
    const existing = providerRecords.find((provider) =>
      provider.id === providerId ||
      normalizeProviderServiceId(provider.service_type || provider.id) === serviceId
    );

    if (existing && isProviderConnected(existing)) {
      return;
    }

    await daemonClient.saveProvider({
      id: existing?.id || providerId,
      name: existing?.name || accountName || `${PROVIDER_LABELS[serviceId]} (OAuth)`,
      service_type: serviceId,
      provider_type: serviceId,
      api_key_masked: existing?.api_key_masked && existing.api_key_masked !== 'none' ? existing.api_key_masked : 'oauth',
      endpoint_url: existing?.endpoint_url || defaultProbeBaseUrl(serviceId),
      status: 'active',
      status_label: 'OAuth Active',
      fast_mode: existing?.fast_mode ?? false,
      is_active: true,
    });
  }, []);

  const handleConnectOAuth = useCallback(async (providerId: ProviderServiceId, accountName: string, _email?: string) => {
    await ensureOAuthProviderRecord(providerId, accountName);
    const refreshed = await refreshProviderState();
    const account = refreshed.accounts.find((item) => item.providerId === providerId);
    showToast(`Connected ${account?.accountName || accountName || PROVIDER_LABELS[providerId]} via OAuth`);
  }, [ensureOAuthProviderRecord, refreshProviderState, showToast]);

  const handleDeleteAccount = useCallback((accountId: string) => {
    const account = connectedAccounts.find((item) => item.id === accountId);
    if (account) {
      void daemonClient.deleteOAuthToken(account.id).finally(() => {
        void refreshProviderState();
      });
    }
    setConnectedAccounts((prev) => prev.filter((a) => a.id !== accountId));
    showToast('Account disconnected');
  }, [connectedAccounts, refreshProviderState, showToast]);

  return (
    <AppContext.Provider
      value={{
        projectData,
        setProjectData,
        currentProject,
        setCurrentProject,
        activeSessionId,
        setActiveSessionId,
        activeSession,
        currentSessions,
        tasks,
        currentTab,
        setCurrentTab,
        isSessionsCollapsed,
        setIsSessionsCollapsed,
        isRightExplorerOpen,
        setIsRightExplorerOpen,
        toggleRightExplorer,
        providers,
        setProviders,
        clientKeys,
        setClientKeys,
        combos,
        setCombos,
        handleCreateCombo,
        handleUpdateCombo,
        handleDeleteCombo,
        usageRecords,
        handleResetUsage,
        connectedAccounts,
        setConnectedAccounts,
        handleConnectOAuth,
        handleDeleteAccount,
        isSettingsOpen,
        setIsSettingsOpen,
        settingsTab,
        setSettingsTab,
        openSettings,
        isNewSessionOpen,
        setIsNewSessionOpen,
        isAddProviderOpen,
        setIsAddProviderOpen,
        uiScale,
        handleSetUiScale,
        handleStepUiScale,
        toastMessage,
        showToast,
        selectedModel,
        handleSelectModel,
        handleSendMessage,
        handleClearHistory,
        handleDeleteConversation,
        handleAcceptAndRun,
        handleRejectDiff,
        handleCopyDiff,
        handleCreateNewSession,
        handleNewProjectPrompt,
        handleSaveProvider,
        handleDeleteProvider,
        handleTestConnection,
        handleGenerateClientKey,
        handleRevokeClientKey,
      }}
    >
      {children}
    </AppContext.Provider>
  );
};

export const useAppContext = () => {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error('useAppContext must be used within an AppProvider');
  }
  return context;
};
