import { useCallback } from 'react';
import type { ProviderItem, ClientApiKey, ModelCombo, ConnectedAccount } from '@/types';
import type { ProviderConfigRecord } from '@/types/domain';
import { daemonClient } from '@/api/daemon_client';
import {
  mapBackendKey,
  PROVIDER_LABELS,
  normalizeProviderServiceId,
  defaultProbeBaseUrl,
  isProviderConnected,
} from './providers_utils';

export interface ProvidersActionsDeps {
  providers: ProviderItem[];
  setProviders: React.Dispatch<React.SetStateAction<ProviderItem[]>>;
  clientKeys: ClientApiKey[];
  setClientKeys: React.Dispatch<React.SetStateAction<ClientApiKey[]>>;
  combos: ModelCombo[];
  setCombos: React.Dispatch<React.SetStateAction<ModelCombo[]>>;
  connectedAccounts: ConnectedAccount[];
  setConnectedAccounts: React.Dispatch<React.SetStateAction<ConnectedAccount[]>>;
  selectedModel: string;
  setSelectedModel: React.Dispatch<React.SetStateAction<string>>;
  refreshProviderState: () => Promise<{ providers: ProviderConfigRecord[]; keys: any[]; accounts: ConnectedAccount[] }>;
  setIsAddProviderOpen: (open: boolean) => void;
  showToast: (msg: string) => void;
}

export function useProvidersActions({
  providers,
  setClientKeys,
  setCombos,
  connectedAccounts,
  selectedModel,
  setSelectedModel,
  refreshProviderState,
  setIsAddProviderOpen,
  showToast,
}: ProvidersActionsDeps) {
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
      showToast(`Provider ${service} saved with secure credentials.`);
    } catch (err: any) {
      console.error('[SaveProvider] Error:', err);
      showToast(`Failed to save provider: ${err?.message || err}`);
      throw err;
    }
  };

  const handleDeleteProvider = async (id: string) => {
    try {
      await daemonClient.deleteProvider(id);
      await refreshProviderState();
      showToast(`Removed provider ${id} from system.`);
    } catch (err: any) {
      console.error('[DeleteProvider] Error:', err);
      showToast(`Failed to remove provider: ${err?.message || err}`);
    }
  };

  const handleTestConnection = async (providerNameOrId: string) => {
    try {
      const provider = providers.find((p) => p.id === providerNameOrId || p.name === providerNameOrId);
      if (!provider) {
        showToast(`Provider ${providerNameOrId} not found.`);
        return;
      }
      const probeRes = await daemonClient.probeModels(provider.endpoint || '', undefined, undefined, provider.id);
      await refreshProviderState();
      showToast(`✓ Successfully probed ${provider.name}; daemon detected ${probeRes.count} models.`);
    } catch (err: any) {
      showToast(`Provider connection error: ${err?.message || err}`);
    }
  };

  const handleGenerateClientKey = async () => {
    try {
      const name = window.prompt('Enter new Client Token name (e.g. CLI Integration, Microservice):', 'Custom API Key');
      if (!name) return;
      await daemonClient.generateClientKey(name);
      const updated = await daemonClient.listClientKeys();
      if (updated) {
        setClientKeys(updated.map(mapBackendKey));
      }
      showToast('New Client Key generated and persisted to SQLite DB.');
    } catch (err: any) {
      console.error('[GenerateKey] Error:', err);
      showToast(`Failed to create key: ${err?.message || err}`);
    }
  };

  const handleRevokeClientKey = async (id: string) => {
    try {
      await daemonClient.revokeClientKey(id);
      const updated = await daemonClient.listClientKeys();
      if (updated) {
        setClientKeys(updated.map(mapBackendKey));
      }
      showToast(`Revoked Client Key ${id.slice(0, 8)}`);
    } catch (err: any) {
      console.error('[RevokeKey] Error:', err);
      showToast(`Failed to revoke key: ${err?.message || err}`);
    }
  };

  const handleCreateCombo = useCallback((comboData: Omit<ModelCombo, 'id' | 'createdAt'>) => {
    const newCombo: ModelCombo = {
      ...comboData,
      id: `combo-${Date.now()}`,
      createdAt: new Date().toISOString(),
    };
    setCombos((prev) => [newCombo, ...prev]);
    showToast(`Routing Combo "${newCombo.name}" created!`);
  }, [setCombos, showToast]);

  const handleUpdateCombo = useCallback((id: string, updates: Partial<ModelCombo>) => {
    setCombos((prev) =>
      prev.map((c) => (c.id === id ? { ...c, ...updates, updatedAt: new Date().toISOString() } : c))
    );
    showToast('Routing Combo updated');
  }, [setCombos, showToast]);

  const handleDeleteCombo = useCallback((id: string) => {
    setCombos((prev) => prev.filter((c) => c.id !== id));
    if (selectedModel === id) {
      setSelectedModel('');
      localStorage.setItem('custos_selected_model', '');
    }
    showToast('Routing Combo deleted');
  }, [selectedModel, setCombos, setSelectedModel, showToast]);

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
      showToast(`Failed to disconnect: ${err}`);
    }
  }, [connectedAccounts, refreshProviderState, showToast]);

  return {
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
  };
}
