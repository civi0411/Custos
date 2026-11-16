import { useState, useCallback, useEffect, useRef } from 'react';
import type {
  ProviderItem,
  ClientApiKey,
  ModelCombo,
  ConnectedAccount,
  ProviderServiceId,
  ProjectData,
  Session,
} from '@/types';
import type {
  ProviderConfigRecord,
  OAuthTokenRecord,
} from '@/types/domain';
import { daemonClient } from '@/api/daemon_client';
import {
  mapBackendProvider,
  mapBackendKey,
  PROVIDER_LABELS,
  normalizeProviderServiceId,
  defaultProbeBaseUrl,
  isProviderConnected,
  safeIsoFromMillis,
  tokenLooksOauth,
  uniqueModels,
  AI_LOCAL_ACCOUNT_ID,
  AI_LOCAL_ENDPOINT,
  isTransientAiLocalAccount,
} from './providers_utils';
import { useLocalAccountsPoll } from './providers_local';
import { useProvidersActions } from './providers_actions';

export {
  mapBackendProvider,
  mapBackendKey,
  PROVIDER_LABELS,
  normalizeProviderServiceId,
  defaultProbeBaseUrl,
  isProviderConnected,
  safeIsoFromMillis,
  tokenLooksOauth,
  uniqueModels,
  AI_LOCAL_ACCOUNT_ID,
  AI_LOCAL_ENDPOINT,
  isTransientAiLocalAccount,
};

export interface ProvidersSliceDeps {
  currentProject: string;
  activeSessionId: string;
  activeSession: Session | null;
  setProjectData: React.Dispatch<React.SetStateAction<ProjectData>>;
  showToast: (msg: string) => void;
  setIsAddProviderOpen: (open: boolean) => void;
}

export function useProvidersSlice({
  currentProject,
  activeSessionId,
  activeSession,
  setProjectData,
  showToast,
  setIsAddProviderOpen,
}: ProvidersSliceDeps) {
  const [providers, setProviders] = useState<ProviderItem[]>([]);
  const [clientKeys, setClientKeys] = useState<ClientApiKey[]>([]);

  const [combos, setCombos] = useState<ModelCombo[]>(() => {
    try {
      const saved = localStorage.getItem('custos_combos');
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
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
    try {
      const saved = localStorage.getItem('custos_connected_accounts');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) return parsed;
      }
    } catch {}
    return [];
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_connected_accounts', JSON.stringify(connectedAccounts));
    } catch (e) {
      console.warn('Failed to save connected accounts to localStorage', e);
    }
  }, [connectedAccounts]);

  const [selectedModel, setSelectedModel] = useState<string>(() => {
    return localStorage.getItem('custos_selected_model') || '';
  });
  const [selectedModelAccountId, setSelectedModelAccountId] = useState(() => localStorage.getItem('custos_selected_model_account') || '');
  const [selectedReasoningEffort, setSelectedReasoningEffort] = useState(() => localStorage.getItem('custos_selected_reasoning_effort') || '');

  const selectedModelAvailable = (() => {
    if (!selectedModel) return false;
    if (selectedModel.startsWith('native:')) return true;
    return connectedAccounts.some(account => account.status === 'active' && account.supportedModels.includes(selectedModel) &&
      (!selectedReasoningEffort || account.catalogModels?.some(model => model.id === selectedModel && model.efforts.includes(selectedReasoningEffort))));
  })();

  const localAccountsRef = useRef(connectedAccounts);
  useEffect(() => {
    localAccountsRef.current = connectedAccounts;
  }, [connectedAccounts]);

  const refreshCatalogAvailability = useCallback(async () => {
    const updates = await Promise.all(localAccountsRef.current.filter(account => !isTransientAiLocalAccount(account)).map(async account => {
      try { return { id: account.id, endpoint: account.endpointUrl, catalog: await daemonClient.getModelCatalog(account.providerId, account.id) }; } catch { return null; }
    }));
    setConnectedAccounts(previous => previous.map(account => {
      const update = updates.find(value => value?.id === account.id && value.endpoint === account.endpointUrl);
      if (!update) return account;
      const hasModels = update.catalog.models && update.catalog.models.length > 0;
      const status = hasModels ? 'active' : account.status;
      return {
        ...account,
        status,
        statusLabel: status === 'active' ? 'Connected' : account.statusLabel,
        supportedModels: update.catalog.models.map(model => model.id),
        catalogModels: update.catalog.models,
        unavailableModels: update.catalog.unavailable_models,
        catalogError: Object.values(update.catalog.unavailable_models || {}).map(value => value.reason).join('; ') || (status !== 'active' ? account.catalogError : undefined)
      };
    }));
  }, []);

  const refreshProviderState = useCallback(async () => {
    const [fetchedProviders, fetchedKeys] = await Promise.all([
      daemonClient.listProviders(),
      daemonClient.listClientKeys(),
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
      providerRecords = (await daemonClient.listProviders()) as ProviderConfigRecord[];
    }

    setProviders(providerRecords.map(mapBackendProvider));
    setClientKeys(fetchedKeys.map(mapBackendKey));

    const accounts = await Promise.all(
      providerRecords.map(async (provider): Promise<ConnectedAccount | null> => {
        const serviceId = normalizeProviderServiceId(provider.service_type || provider.id);
        const endpointUrl = provider.endpoint_url || defaultProbeBaseUrl(serviceId);
        const connected = isProviderConnected(provider);
        let catalogModels = [] as any[];
        let catalogSource = 'unavailable' as ConnectedAccount['catalogSource'];
        let catalogFetchedAt = undefined as number | undefined;
        let unavailableModels: ConnectedAccount['unavailableModels'];
        let catalogError: string | undefined;
        let tokenMeta: OAuthTokenRecord | null = null;
        let credentialError: string | undefined;

        if (serviceId === 'copilot' || provider.api_key_masked === 'oauth') {
          tokenMeta = await daemonClient.getOAuthToken(serviceId).catch(() => null);
          if (tokenMeta && !tokenMeta.connected) {
            credentialError = 'OAuth token expired or revoked. Reconnect in Settings.';
          }
        }

        if (connected && endpointUrl) {
          try {
            const verified = await daemonClient.probeModels(endpointUrl, serviceId, undefined, provider.id);
            if (verified.models && verified.models.length > 0) {
              catalogSource = 'live';
              catalogFetchedAt = Date.now();
              catalogModels = verified.models;
            }
          } catch (err) {
            catalogError = err instanceof Error ? err.message : String(err);
          }
        }

        if (connected && catalogSource !== 'live') {
          try {
            const cached = await daemonClient.getModelCatalog(provider.service_type || serviceId, provider.id);
            if (cached.models && cached.models.length > 0) {
              catalogModels = cached.models;
              catalogSource = 'cached';
              catalogFetchedAt = cached.fetched_at;
              unavailableModels = cached.unavailable_models;
              catalogError = Object.values(unavailableModels || {}).map(value => value.reason).join('; ') || undefined;
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
        const available = connected && (catalogSource === 'live' || catalogSource === 'cached' || catalogModels.length > 0);
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
          catalogModels,
          catalogSource,
          catalogError,
          catalogFetchedAt,
          unavailableModels,
          contextWindow,
          fastMode: provider.fast_mode,
          latencyMs: 0,
          createdAt: safeIsoFromMillis(provider.created_at),
          lastUsedAt: 'Active',
        };
      })
    );

    const liveAccounts = accounts.filter((account): account is ConnectedAccount => account !== null);
    const hasSavedLocalAccount = providerRecords.some(
      (provider) => normalizeProviderServiceId(provider.service_type || provider.id) === 'local'
    );

    if (!hasSavedLocalAccount) {
      try {
        const discovered = await daemonClient.probeModels(AI_LOCAL_ENDPOINT, 'ollama');
        const localModels = uniqueModels(discovered.models.map((model) => model.id));
        if (localModels.length > 0) {
          liveAccounts.unshift({
            id: AI_LOCAL_ACCOUNT_ID,
            providerId: 'local',
            providerName: 'AI Local',
            accountName: 'AI Local (Ollama)',
            authType: 'api_key',
            endpointUrl: AI_LOCAL_ENDPOINT,
            status: 'active',
            statusLabel: 'Ready without sign-in',
            badgeColor: '#10b981',
            defaultModel: localModels[0],
            supportedModels: localModels,
            catalogModels: discovered.models.map((model, index) => ({
              id: model.id,
              label: model.name || model.id,
              description: model.description,
              provider_type: 'local',
              is_default: index === 0,
              efforts: [],
              supports_fast_mode: false,
              context_window: model.context_window,
            })),
            catalogSource: 'live',
            catalogFetchedAt: Date.now(),
            contextWindow: discovered.models[0]?.context_window,
            latencyMs: 0,
            createdAt: new Date().toISOString(),
            lastUsedAt: 'Available locally',
          });
        }
      } catch {
        // AI Local is optional.
      }
    }
    setConnectedAccounts(liveAccounts);

    const fallbackModel =
      liveAccounts.find((account) => account.status === 'active' && account.defaultModel)?.defaultModel ||
      liveAccounts.flatMap((account) => account.supportedModels)[0] ||
      '';

    setSelectedModel((previous) => {
      const current = previous || localStorage.getItem('custos_selected_model') || '';
      if (current) return current;
      localStorage.setItem('custos_selected_model', fallbackModel);
      return fallbackModel;
    });

    return { providers: providerRecords, keys: fetchedKeys, accounts: liveAccounts };
  }, []);

  useLocalAccountsPoll(connectedAccounts, setConnectedAccounts);

  const handleSelectModel = useCallback((model: string, effort?: string, accountId?: string) => {
    setSelectedModel(model);
    localStorage.setItem('custos_selected_model', model);
    if (effort !== undefined) {
      setSelectedReasoningEffort(effort);
      localStorage.setItem('custos_selected_reasoning_effort', effort);
    }
    if (accountId !== undefined) {
      setSelectedModelAccountId(accountId);
      localStorage.setItem('custos_selected_model_account', accountId);
    }
    if (activeSessionId) {
      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        return {
          ...prev,
          [currentProject]: list.map((s) =>
            s.id === activeSessionId
              ? {
                  ...s,
                  model,
                  modelAccountId: accountId ?? s.modelAccountId,
                  reasoningEffort: effort ?? s.reasoningEffort,
                }
              : s
          ),
        };
      });
      try {
        const stored = JSON.parse(localStorage.getItem(`custos.conversation_model.${activeSessionId}`) || '{}');
        localStorage.setItem(`custos.conversation_model.${activeSessionId}`, JSON.stringify({
          ...stored,
          model,
          accountId: accountId ?? stored.accountId,
          effort: effort ?? stored.effort,
        }));
      } catch {}
    }
  }, [activeSessionId, currentProject, setProjectData]);

  useEffect(() => {
    if (!activeSessionId) return;
    try {
      const saved = JSON.parse(localStorage.getItem(`custos.conversation_model.${activeSessionId}`) || '{}');
      if (saved && typeof saved.model === 'string' && saved.model.length > 0) {
        setSelectedModel(saved.model);
        if (typeof saved.accountId === 'string') setSelectedModelAccountId(saved.accountId);
        if (typeof saved.effort === 'string') setSelectedReasoningEffort(saved.effort);
        return;
      }
    } catch {}
    if (activeSession?.model) {
      setSelectedModel(activeSession.model);
      if (activeSession.modelAccountId) setSelectedModelAccountId(activeSession.modelAccountId);
      if (activeSession.reasoningEffort) setSelectedReasoningEffort(activeSession.reasoningEffort);
    }
  }, [activeSessionId, activeSession?.model, activeSession?.modelAccountId, activeSession?.reasoningEffort]);

  useEffect(() => {
    const timer = window.setInterval(() => { if (!document.hidden) void refreshCatalogAvailability(); }, 5000);
    return () => window.clearInterval(timer);
  }, [refreshCatalogAvailability]);

  const {
    handleSaveProvider,
    handleDeleteProvider,
    handleTestConnection,
    handleGenerateClientKey,
    handleRevokeClientKey,
    handleCreateCombo,
    handleUpdateCombo,
    handleDeleteCombo,
    handleConnectOAuth,
    handleDeleteAccount,
  } = useProvidersActions({
    providers,
    setProviders,
    clientKeys,
    setClientKeys,
    combos,
    setCombos,
    connectedAccounts,
    setConnectedAccounts,
    selectedModel,
    setSelectedModel,
    refreshProviderState,
    setIsAddProviderOpen,
    showToast,
  });

  const refreshModels = useCallback(async () => {
    await refreshProviderState();
  }, [refreshProviderState]);

  return {
    providers,
    setProviders,
    clientKeys,
    setClientKeys,
    combos,
    setCombos,
    connectedAccounts,
    setConnectedAccounts,
    selectedModel,
    setSelectedModel,
    selectedModelAccountId,
    setSelectedModelAccountId,
    selectedReasoningEffort,
    setSelectedReasoningEffort,
    selectedModelAvailable,
    refreshProviderState,
    refreshCatalogAvailability,
    refreshModels,
    handleSelectModel,
    handleCreateCombo,
    handleUpdateCombo,
    handleDeleteCombo,
    handleConnectOAuth,
    handleDeleteAccount,
    handleSaveProvider,
    handleDeleteProvider,
    handleTestConnection,
    handleGenerateClientKey,
    handleRevokeClientKey,
  };
}
