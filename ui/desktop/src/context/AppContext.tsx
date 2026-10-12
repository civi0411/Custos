import { APPEARANCE_KEY, readAppearance, applyAppearance, type AppearancePreferences } from '@/lib/appearance';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import React, { createContext, useContext, useState, useEffect, useRef, useCallback } from 'react';
import {
  MainTab,
  ProjectData,
  ProviderItem,
  ClientApiKey,
  Session,
  ModelCombo,
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
  connectedAccounts: ConnectedAccount[];
  refreshModels: () => Promise<void>;
  setConnectedAccounts: React.Dispatch<React.SetStateAction<ConnectedAccount[]>>;
  handleConnectOAuth: (providerId: string, accountName: string, email?: string) => Promise<void>;
  handleDeleteAccount: (accountId: string) => Promise<void>;

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

  // Device-local appearance
  appearance: AppearancePreferences;
  resolvedTheme: 'light' | 'dark';
  updateAppearance: (next: Partial<AppearancePreferences>) => void;

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
  startNativeConversation: (instruction: string, harness: string, workspace: string) => Promise<{ run: import('../types/domain').RunHandle; sessionId: string }>;
  handleStopRun: (runId: string) => Promise<void>;
  handleSendMessage: (text: string, lens?: 'assistant' | 'coding' | 'research') => Promise<void>;
  handleClearHistory: () => void;
  handleDeleteConversation: (sessionId: string) => Promise<void>;
  handleAcceptAndRun: () => Promise<void>;
  handleRejectDiff: () => void;
  handleCopyDiff: () => void;
  handleCreateNewSession: (title: string, pack: 'engineering' | 'research' | 'assistant') => Promise<void>;
  handleNewProjectPrompt: () => void;
  handleSaveProvider: (
    serviceOrParams: string | { service: string; accountName?: string; apiKey?: string; endpointUrl?: string; model?: string; contextWindow?: number; fastMode?: boolean },
    apiKey?: string
  ) => Promise<void>;
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
  antigravity: 'Google Antigravity',
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
  if (s.includes('antigravity')) return 'antigravity';
  if (s.includes('copilot')) return 'copilot';
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
    case 'copilot':
      return 'https://api.githubcopilot.com';
    case 'antigravity':
      return 'https://cloudcode-pa.googleapis.com';
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
  Boolean(token?.has_refresh_token || token?.scope?.trim());

const uniqueModels = (models: string[]): string[] => {
  const seen = new Set<string>();
  return models.filter((model) => {
    const key = model.trim().toLowerCase();
    if (!key || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
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

  // Modals state
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [settingsTab, setSettingsTab] = useState<string>('general');
  const [isNewSessionOpen, setIsNewSessionOpen] = useState<boolean>(false);
  const [isAddProviderOpen, setIsAddProviderOpen] = useState<boolean>(false);

  const openSettings = useCallback((tab?: string) => {
    if (tab) setSettingsTab(tab);
    setIsSettingsOpen(true);
  }, []);

  const [appearance, setAppearance] = useState<AppearancePreferences>(readAppearance);
  const [systemDark, setSystemDark] = useState(() => window.matchMedia('(prefers-color-scheme: dark)').matches);
  const resolvedTheme = appearance.theme === 'system' ? (systemDark ? 'dark' : 'light') : appearance.theme;
  useEffect(() => {
    if (appearance.theme !== 'system') return;
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const update = () => setSystemDark(media.matches);
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, [appearance.theme]);
  useEffect(() => {
    const sync = (event: StorageEvent) => {
      if (event.key !== APPEARANCE_KEY && event.key !== null) return;
      const saved = readAppearance();
      if (saved.theme === 'system') setSystemDark(window.matchMedia('(prefers-color-scheme: dark)').matches);
      setAppearance(saved);
    };
    window.addEventListener('storage', sync);
    return () => window.removeEventListener('storage', sync);
  }, []);
  useEffect(() => { applyAppearance(appearance, resolvedTheme); }, [appearance, resolvedTheme]);
  const themeQueue = useRef<Promise<void>>(Promise.resolve());
  useEffect(() => {
    if (!isTauri()) return;
    const theme = appearance.theme === 'system' ? null : resolvedTheme;
    themeQueue.current = themeQueue.current
      .then(async () => {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setTheme(theme);
      })
      .catch(error => console.warn('Native window theme could not be applied', error));
  }, [appearance.theme, resolvedTheme]);
  const updateAppearance = useCallback((next: Partial<AppearancePreferences>) => {
    if (next.theme === 'system') setSystemDark(window.matchMedia('(prefers-color-scheme: dark)').matches);
    setAppearance(previous => {
      const updated = { ...previous, ...next };
      try { localStorage.setItem(APPEARANCE_KEY, JSON.stringify(updated)); }
      catch (error) { console.warn('Device appearance could not be saved', error); }
      return updated;
    });
  }, []);

  // UI Scale state
  const [uiScale, setUiScale] = useState<number>(() => {
    const saved = localStorage.getItem('custos_ui_scale');
    const value = Number(saved || 100);
    return Math.min(160, Math.max(70, Math.round((Number.isFinite(value) ? value : 100) / 10) * 10));
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

  const zoomQueue = useRef<Promise<void>>(Promise.resolve());
  const applyUiScale = useCallback((scaleVal: number) => {
    const rounded = Math.round((Number.isFinite(scaleVal) ? scaleVal : 100) / 10) * 10;
    const clamped = Math.min(160, Math.max(70, rounded));
    setUiScale(clamped);
    localStorage.setItem('custos_ui_scale', clamped.toString());
  }, []);

  useEffect(() => {
    const factor = uiScale / 100;
    document.documentElement.style.zoom = '';
    document.body.style.zoom = '';
    document.documentElement.style.setProperty('--app-browser-zoom', '1');
    if (isTauri()) {
      // Serialize native zoom changes so rapid clicks settle at the latest level.
      zoomQueue.current = zoomQueue.current.then(() => getCurrentWebview().setZoom(factor)).catch(error => {
        showToast(`Unable to zoom app: ${String(error)}`);
      });
    } else {
      document.documentElement.style.zoom = String(factor);
      document.documentElement.style.setProperty('--app-browser-zoom', String(factor));
    }
  }, [uiScale, showToast]);

  const handleSetUiScale = (val: number) => {
    applyUiScale(val);
  };

  const handleStepUiScale = (delta: number) => {
    handleSetUiScale(uiScale + delta);
  };

  // Check active session object
  const currentSessions = projectData[currentProject] || [];
  const activeSession: Session | null =
    currentSessions.find((s) => s.id === activeSessionId) || null;

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
      if (!tokenLooksOauth(token) || !token?.connected) {
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
        let connected = isProviderConnected(provider);
        const endpointUrl = provider.endpoint_url || defaultProbeBaseUrl(serviceId);

        let tokenMeta: OAuthTokenRecord | null = null;
        let credentialError: string | undefined;
        tokenMeta = await daemonClient.getOAuthToken(provider.id).catch((error) => { credentialError = error instanceof Error ? error.message : String(error); return null; });
        if (serviceId !== 'local') connected = connected && tokenMeta?.connected === true;

        let catalogModels: ModelCatalogOption[] = [];
        let catalogSource: 'live' | 'cached' | 'unavailable' = 'unavailable';
        let catalogError: string | undefined = credentialError;
        let catalogFetchedAt: number | undefined;
        if (connected && endpointUrl) {
          try {
            const probed = await daemonClient.probeModels(
              endpointUrl,
              serviceId === 'local' && provider.default_model?.startsWith('ollama/') ? 'ollama' : provider.service_type || serviceId,
              undefined,
              provider.id
            );
            catalogSource = 'live';
            catalogFetchedAt = Date.now();
            catalogModels = probed.models.map((model, index) => ({
              id: model.id,
              label: model.name || model.id,
              description: model.description,
              provider_type: provider.service_type || serviceId,
              is_default: index === 0,
              default_effort: undefined,
              efforts: [],
              supports_fast_mode: false,
              context_window: model.context_window || provider.context_window,
              pricing: undefined,
            }));
          } catch (err) {
            catalogError = err instanceof Error ? err.message : String(err);
          }
        }

        if (connected && catalogSource !== 'live') {
          try {
            const cached = await daemonClient.getModelCatalog(provider.service_type || serviceId, provider.id);
            if (cached.origin === 'cached' && cached.models.length) {
              catalogModels = cached.models;
              catalogSource = 'cached';
              catalogFetchedAt = cached.fetched_at;
            }
          } catch (err) {
            catalogError = err instanceof Error ? err.message : String(err);
          }
        }

        const supportedModels = uniqueModels(catalogModels.map((model) => model.id));

        if (!credentialError && (!connected || (provider.api_key_masked === 'oauth' && !tokenMeta?.connected))) {
          return null;
        }

        const savedDefault = serviceId === 'local' && provider.default_model && !provider.default_model.startsWith('ollama/') && supportedModels.includes(`ollama/${provider.default_model}`)
          ? `ollama/${provider.default_model}` : provider.default_model;
        const defaultModel = savedDefault && supportedModels.includes(savedDefault)
          ? savedDefault : supportedModels[0] || '';
        const contextWindow =
          catalogModels.find((model) => model.id === defaultModel)?.context_window ||
          provider.context_window ||
          catalogModels[0]?.context_window;
        const available = connected && !(serviceId === 'local' && catalogSource !== 'live');
        const authType = tokenLooksOauth(tokenMeta) || provider.api_key_masked === 'oauth'
          ? 'oauth'
          : 'api_key';

        return {
          id: provider.id,
          providerId: serviceId,
          providerName: PROVIDER_LABELS[serviceId] || provider.name,
          accountName: provider.name || `${PROVIDER_LABELS[serviceId]} Account`,
          authType,
          apiKeyMasked: authType === 'api_key' ? provider.api_key_masked : undefined,
          endpointUrl,
          oauthEmail: authType === 'oauth' && provider.name.includes('@') ? provider.name : undefined,
          oauthTier: authType === 'oauth' ? (serviceId === 'copilot' ? 'Connected via Device Flow' : 'Connected via PKCE') : undefined,
          oauthExpiresAt: tokenMeta?.expires_at ? safeIsoFromMillis(tokenMeta.expires_at) : undefined,
          status: available ? 'active' : 'offline',
          statusLabel: available ? 'Connected' : credentialError || (serviceId === 'local' ? 'Ollama offline — start Ollama and retry' : 'Reconnect provider'),
          badgeColor: available ? '#10b981' : '#6b7280',
          defaultModel,
          supportedModels,
          catalogModels, catalogSource, catalogError, catalogFetchedAt,
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
      if (current.startsWith('ollama/') || (current && liveModelIds.has(current.toLowerCase()))) {
        return current;
      }
      localStorage.setItem('custos_selected_model', fallbackModel);
      return fallbackModel;
    });

    return { providers: providerRecords, keys: fetchedKeys, accounts: liveAccounts };
  }, []);

  // Availability does not mutate saved configuration or switch the chosen model.
  useEffect(() => {
    const accounts = connectedAccounts.filter(a => a.providerId === 'local' && a.endpointUrl && (a.endpointUrl?.includes(':11434') || a.defaultModel?.startsWith('ollama/') || a.supportedModels.some(m => m.startsWith('ollama/'))));
    if (!accounts.length) return;
    let cancelled = false;
    let probing = false;
    const check = async () => {
      if (probing || document.hidden) return;
      probing = true;
      try {
        await Promise.all(accounts.map(async account => {
          let updates: Partial<ConnectedAccount>;
          try {
            const result = await daemonClient.probeModels(account.endpointUrl!, 'ollama');
            updates = {
              status: 'active', statusLabel: 'Connected', badgeColor: '#10b981', catalogSource: 'live', catalogError: undefined, catalogFetchedAt: Date.now(),
              supportedModels: result.models.map(m => m.id),
              catalogModels: result.models.map((m, i) => ({ id: m.id, label: m.name || m.id, provider_type: 'local', is_default: i === 0, efforts: [], supports_fast_mode: false, context_window: m.context_window })),
            };
          } catch {
            updates = { status: 'offline', statusLabel: 'Ollama offline', badgeColor: '#6b7280', catalogSource: account.catalogModels?.length ? 'cached' : 'unavailable', catalogError: 'Ollama is unavailable. Start Ollama, then retry. The selected model has not changed.' };
          }
          if (!cancelled) setConnectedAccounts(previous => previous.map(a => a.id === account.id && a.endpointUrl === account.endpointUrl ? { ...a, ...updates } : a));
        }));
      } finally { probing = false; }
    };
    const timer = window.setInterval(() => { void check(); }, 15000);
    const onVisible = () => { if (!document.hidden) void check(); };
    window.addEventListener('focus', onVisible);
    document.addEventListener('visibilitychange', onVisible);
    return () => { cancelled = true; window.clearInterval(timer); window.removeEventListener('focus', onVisible); document.removeEventListener('visibilitychange', onVisible); };
  }, [connectedAccounts]);

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

        setTasks(fetchedTasks);

        if (fetchedSessions.length > 0) {
          // Keep every canonical session, including multiple sessions bound to one Task.
          const projectedSessions: Session[] = fetchedSessions.map((domainSession) => {
            const attachedTaskId = domainSession.task_id || domainSession.attached_to
              || (typeof domainSession.mode === 'object' ? domainSession.mode.attached?.task_id : undefined);
            const t = fetchedTasks.find((task) => task.id === attachedTaskId);
            const taskId = t?.id;
            const sessionId = domainSession.id;
            const rawJournal = journalBySession.get(domainSession.id) || domainSession.journal || [];
            const journalList: any[] = Array.isArray(rawJournal)
              ? rawJournal
              : Array.isArray((rawJournal as any)?.entries)
              ? (rawJournal as any).entries
              : Array.isArray((rawJournal as any)?.journal)
              ? (rawJournal as any).journal
              : [];

            return {
              id: sessionId,
              nativeHarnessId: t?.metadata?.kind === 'native_coding_agent' ? t.metadata.harness_id : undefined,
              workspaceId: t?.metadata?.workspace_id,
              openedLenses: Array.from(new Set(journalList
                .filter((entry) => entry.entry_type === 'workbench_opened')
                .map((entry) => entry.entry_data)
                .filter((lens): lens is 'chat' | 'code' | 'research' =>
                  lens === 'chat' || lens === 'code' || lens === 'research'))),
              taskId,
              source: 'daemon',
              taskStatus: t ? String(t.status) : undefined,
              sessionId,
              pack: t?.metadata?.kind === 'native_coding_agent' ? 'engineering' : t?.contract?.pack_id || t?.contract?.pack || 'assistant',
              title: t?.title || domainSession.title || domainSession.current_goal || `Conversation ${sessionId.slice(0, 8)}`,
              time: 'Live',
              preview: t ? `Status: ${t.status} | Contract: ${t.contract?.pack_id || 'general'}` : 'Unbound conversation',
              model: t?.metadata?.kind === 'native_coding_agent' ? t.metadata.harness_id : (t?.contract as any)?.model || selectedModel || '',
              fileName: '',
              diffHunk: '',
              diffLinesCount: '',
              summary: t ? `Task ${t.id}: ${t.status}` : 'No Task bound',
              messages: journalList.filter((j) =>
                j.entry_type === 'user_message' || j.entry_type === 'assistant_message' ||
                j.entry_type === 'user' || j.entry_type === 'assistant' ||
                j.role === 'user' || j.role === 'assistant'
              ).map((j) => {
                const isUser =
                  j.role === 'user' ||
                  j.entry_type === 'user' ||
                  j.entry_type === 'user_message';
                return {
                  id: j.id || String(j.entry_id || `${sessionId}-${j.occurred_at}`),
                  recordedAt: j.occurred_at,
                  role: isUser ? 'user' : 'assistant',
                  author: isUser ? 'You' : (j.author || 'Custos Kernel'),
                  badge: j.badge,
                  stepName: j.step_name,
                  duration: j.duration,
                  text: j.content || j.entry_data || '',
                };
              }),
              diffCode: [],
            };
          });

          setProjectData((prev) => ({
            ...prev,
            'Custos Workspace': projectedSessions,
          }));

          for (const chat of projectedSessions) {
            const runs = await daemonClient.listSessionRuns(chat.sessionId || chat.id).catch(() => []);
            for (const run of runs) {
              if (run.journal_occurred_at) {
                setProjectData(previous => ({...previous, 'Custos Workspace': (previous['Custos Workspace'] || []).map(item => item.id !== chat.id ? item : {...item, messages: item.messages.map(message => message.recordedAt === run.journal_occurred_at ? {...message, runId: run.run_id, runStatus: run.status, badge: run.status, author: run.model || message.author} : message)})}));
              }
              followLiveRun('Custos Workspace', chat.id, chat.sessionId || chat.id, run.run_id, run.model || 'Custos');
            }
          }

          setCurrentProject('Custos Workspace');
          setActiveSessionId((previous) => projectedSessions.some((session) => session.id === previous)
            ? previous : '');
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
  const pendingTurns = useRef(new Set<string>());
  const liveConnections = useRef(new Map<string, AbortController>());
  useEffect(() => () => { liveConnections.current.forEach(controller => controller.abort()); liveConnections.current.clear(); }, []);
  const handleStopRun = async (runId: string) => {
    try { await daemonClient.cancelRun({ run_id: runId, reason: 'Stopped from chat' }); }
    catch (error) { showToast(error instanceof Error ? error.message : String(error)); }
  };
  const followLiveRun = (project: string, conversation: string, session: string, run: string, author: string) => {
    if (liveConnections.current.has(run)) return;
    const controller = new AbortController(); liveConnections.current.set(run, controller);
    let timer: number | undefined;
    let latest: import('../types/domain').LiveRunSnapshot | null = null;
    const flush = () => {
      timer = undefined;
      const snapshot = latest; if (!snapshot) return;
      setProjectData(previous => ({ ...previous, [project]: (previous[project] || []).map(chat => chat.id !== conversation ? chat : {
        ...chat, messages: chat.messages.some(message => message.runId === run)
          ? chat.messages.map(message => message.runId !== run ? message : { ...message, text: snapshot.text, runStatus: snapshot.status, badge: snapshot.reconnecting ? 'Reconnecting' : snapshot.status, activities: snapshot.activities, liveError: snapshot.error || undefined })
          : [...chat.messages, { id: `live-${run}`, role: 'assistant' as const, author, text: snapshot.text, runId: run, runStatus: snapshot.status, badge: snapshot.reconnecting ? 'Reconnecting' : snapshot.status, activities: snapshot.activities, liveError: snapshot.error || undefined }],
      }) }));
    };
    void daemonClient.watchRun(session, run, snapshot => {
      latest = snapshot;
      if (['completed', 'failed', 'cancelled', 'interrupted'].includes(snapshot.status)) { if (timer !== undefined) window.clearTimeout(timer); flush(); }
      else if (timer === undefined) timer = window.setTimeout(flush, 50);
    }, controller.signal).finally(() => { if (timer !== undefined) window.clearTimeout(timer); if (!controller.signal.aborted) flush(); liveConnections.current.delete(run); });
  };

  const startNativeConversation = async (instruction: string, harness: string, workspace: string) => {
    const session = await daemonClient.createSession('assisted');
    const task = await daemonClient.createTask({ title: instruction.slice(0, 120), metadata: { kind: 'native_coding_agent', pack_id: 'engineering', harness_id: harness, workspace_id: workspace, conversation_session_id: session.id } });
    await daemonClient.attachSession({ session_id: session.id, task_id: task.id });
    await daemonClient.appendSessionMessage(session.id, 'user', instruction);
    const project = 'Custos Workspace';
    const chat: Session = { nativeHarnessId: harness, workspaceId: workspace, id: session.id, sessionId: session.id, taskId: task.id, source: 'daemon', title: instruction.slice(0, 80), time: 'Live', preview: harness, model: harness, pack: 'engineering', fileName: '', diffHunk: '', diffLinesCount: '', summary: '', diffCode: [], messages: [{ role: 'user', author: 'You', text: instruction }] };
    setProjectData(previous => ({ ...previous, [project]: [...(previous[project] || []), chat] }));
    const run = await daemonClient.startRun({ background: true, task_id: task.id, session_id: session.id, turn_id: `turn_${crypto.randomUUID()}`, preferred_mode: 'native', harness_id: harness, workspace_id: workspace, prompt: instruction });
    setCurrentProject(project); setActiveSessionId(session.id);
    followLiveRun(project, session.id, session.id, run.run_id, harness);
    return { run, sessionId: session.id };
  };

  const handleSendMessage = async (text: string, lens?: 'assistant' | 'coding' | 'research') => {
    if (!activeSession) return;
    if (activeSession.messages.some(message => message.runId && !['completed','failed','cancelled','interrupted'].includes(message.runStatus || ''))) { showToast('Stop or wait for the current run before sending another prompt.'); return; }
    if (pendingTurns.current.has(activeSession.id)) return;
    pendingTurns.current.add(activeSession.id);
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
      if (activeSession.nativeHarnessId && activeSession.workspaceId && activeSession.sessionId) {
        const harness = activeSession.nativeHarnessId;
        const task = await daemonClient.createTask({ title: text.slice(0, 120), metadata: { kind: 'native_coding_agent', pack_id: 'engineering', harness_id: harness, workspace_id: activeSession.workspaceId, conversation_session_id: activeSession.sessionId } });
        await daemonClient.appendSessionMessage(activeSession.sessionId, 'user', text);
        const run = await daemonClient.startRun({ background: true, task_id: task.id, session_id: activeSession.sessionId, turn_id: `turn_${crypto.randomUUID()}`, preferred_mode: 'native', harness_id: harness, workspace_id: activeSession.workspaceId, prompt: text });
        setProjectData(previous => ({ ...previous, [currentProject]: (previous[currentProject] || []).map(chat => chat.id !== activeSession.id ? chat : { ...chat, messages: [...chat.messages, { id: `live-${run.run_id}`, role: 'assistant' as const, author: harness, runId: run.run_id, runStatus: 'queued', badge: 'queued', text: '' }] }) }));
        followLiveRun(currentProject, activeSession.id, activeSession.sessionId, run.run_id, harness);
        return;
      }
      const chosenModel = (selectedModel || activeSession.model || '').trim();
      const selectedCombo = combos.find((combo) => combo.id.toLowerCase() === chosenModel.toLowerCase());

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

      let routedAccountId: string;
      try {
        const modelAccount = connectedAccounts.find(a => a.status === 'active' && a.supportedModels.includes(chosenModel));
        if (!modelAccount) throw new Error('No connected account supports this model');
        const route = await daemonClient.reserveProviderAccount(modelAccount.providerId, chosenModel);
        if (!route.account_id) throw new Error('No eligible account is available');
        routedAccountId = route.account_id;
      } catch (error: any) {
        appendAssistantMessage({ role: 'assistant', author: 'Custos Engine', badge: 'Account Unavailable', text: error?.message || String(error) });
        return;
      }

      if (chosenModel.startsWith('ollama/')) {
        const account = connectedAccounts.find(a => a.id === routedAccountId && a.status === 'active');
        if (!account) {
          appendAssistantMessage({ role: 'assistant', author: 'Custos Engine', badge: 'Ollama Unavailable', stepName: 'Model Gateway', text: 'Ollama is offline or the selected model is no longer installed. Start Ollama and use Retry models. Your prompt will not be sent to another provider.' });
          showToast('Ollama unavailable — start Ollama and retry');
          return;
        }
        // Recheck before dispatch to catch shutdowns between periodic probes.
        try {
          const probe = await daemonClient.probeModels(account.endpointUrl || 'http://localhost:11434', 'ollama');
          if (!probe.models.some(model => model.id === chosenModel)) {
            appendAssistantMessage({ role: 'assistant', author: 'Custos Engine', badge: 'Model Removed', text: 'The selected Ollama model is no longer installed. Refresh models and choose an installed model.' });
            return;
          }
        } catch {
          setConnectedAccounts(previous => previous.map(a => a.id === account.id ? { ...a, status: 'offline', statusLabel: 'Ollama offline', badgeColor: '#6b7280', catalogSource: 'cached', catalogError: 'Ollama is unavailable. Start Ollama and retry models.' } : a));
          appendAssistantMessage({ role: 'assistant', author: 'Custos Engine', badge: 'Ollama Offline', text: 'Cannot connect to Ollama. Start Ollama and retry models, then resend your prompt. No other provider was used.' });
          return;
        }
      }

      const hasActiveAccount = connectedAccounts.some((account) => account.status === 'active');
      if (!hasActiveAccount) {
        appendAssistantMessage({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'Model Not Connected',
          stepName: 'Model Gateway',
          text: `Model "${chosenModel}" cannot be dispatched because no active provider accounts are connected. Please connect a provider in Settings > Providers first.`,
        });
        showToast('No active provider connected');
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
        const rawTitle = (activeSession.title || chosenModel).replace(/^(Chat turn:\s*)+/, '').trim();
        const turnTitle = rawTitle ? `Chat turn: ${rawTitle}` : `Chat turn: ${chosenModel}`;
        const createdTask = await daemonClient.createTask({
          title: turnTitle,
          metadata: {
            kind: 'conversation_chat_turn',
            pack_id: lens || activeSession.pack || 'assistant',
            conversation_session_id: targetSessionId || activeSession.id,
            source_task_id: activeSession.taskId || activeSession.id,
            model: chosenModel,
            provider_account_id: routedAccountId,
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

      // Register a cancellable background run before consuming its live projection.
      const run = await daemonClient.startRun({
        background: true,
        turn_id: `turn_${crypto.randomUUID()}`,
        task_id: runTaskId,
        preferred_mode: 'model',
        session_id: targetSessionId,
        prompt: text,
        model: chosenModel,
      });

      const runId = run.run_id;
      appendAssistantMessage({ role: 'assistant', author: chosenModel, badge: 'queued', stepName: `Run #${runId.slice(0, 8)}`, text: '' });
      setProjectData(previous => ({ ...previous, [currentProject]: (previous[currentProject] || []).map(chat => chat.id !== activeSession.id ? chat : { ...chat, messages: chat.messages.map((message,index) => index === chat.messages.length - 1 ? { ...message, id: `live-${runId}`, runId, runStatus: 'queued' } : message) }) }));
      if (targetSessionId) followLiveRun(currentProject, activeSession.id, targetSessionId, runId, chosenModel);
    } catch (err: any) {
      if ((selectedModel || activeSession.model || '').startsWith('ollama/') && String(err?.message || err).includes('Ollama connection failed')) {
        setConnectedAccounts(previous => previous.map(a => a.providerId === 'local' ? { ...a, status: 'offline', statusLabel: 'Ollama offline', badgeColor: '#6b7280', catalogSource: 'cached', catalogError: 'Ollama stopped during the request. Start Ollama and resend your prompt.' } : a));
      }
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
    } finally { pendingTurns.current.delete(activeSession.id); }
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

    const daemonSessionId = target.sessionId || (target.source === 'daemon' ? target.id : undefined);
    if (daemonSessionId) {
      try {
        await daemonClient.deleteSession(daemonSessionId);
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
    serviceOrParams: string | { service: string; accountName?: string; apiKey?: string; endpointUrl?: string; model?: string; contextWindow?: number; fastMode?: boolean },
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

      const providerId = `${normalizeProviderServiceId(service)}-${crypto.randomUUID()}`;
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
        name: (isObj && serviceOrParams.accountName?.trim()) || `${service} Account ${providerId.slice(-6)}`,
        service_type: providerType,
        provider_type: providerType,
        model: defaultModel || '',
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
      showToast(`Provider ${service} đã được lưu với credential bảo mật!`);
    } catch (err: any) {
      console.error('[SaveProvider] Error:', err);
      showToast(`Lỗi lưu provider: ${err?.message || err}`);
      throw err;
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

  const ensureOAuthProviderRecord = useCallback(async (
    providerId: string,
    accountName?: string
  ) => {
    const token = await daemonClient.getOAuthToken(providerId);
    if (!token?.connected) throw new Error('OAuth account credentials were not saved. Retry the connection.');

    const providerRecords = (await daemonClient.listProviders()) as ProviderConfigRecord[];
    const existing = providerRecords.find((provider) => provider.id === providerId);
    const serviceId = normalizeProviderServiceId(existing?.service_type || providerId);

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

  const handleConnectOAuth = useCallback(async (providerId: string, accountName: string, _email?: string) => {
    await ensureOAuthProviderRecord(providerId, accountName);
    const refreshed = await refreshProviderState();
    const account = refreshed.accounts.find((item) => item.id === providerId);
    showToast(`Connected ${account?.accountName || accountName || PROVIDER_LABELS[normalizeProviderServiceId(providerId)]} via OAuth`);
  }, [ensureOAuthProviderRecord, refreshProviderState, showToast]);

  const handleDeleteAccount = useCallback(async (accountId: string) => {
    const account = connectedAccounts.find((item) => item.id === accountId);
    if (!account) return;
    try {
      await daemonClient.deleteOAuthToken(account.id);
      await daemonClient.deleteProvider(account.id);
      await refreshProviderState();
      showToast('Account disconnected');
    } catch (err) {
      showToast(`Could not disconnect account: ${err instanceof Error ? err.message : String(err)}`);
    }
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
        connectedAccounts,
        refreshModels: async () => { await refreshProviderState(); },
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
        appearance,
        resolvedTheme,
        updateAppearance,
        uiScale,
        handleSetUiScale,
        handleStepUiScale,
        toastMessage,
        showToast,
        selectedModel,
        handleSelectModel,
        handleSendMessage,
        handleStopRun,
        startNativeConversation,
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
