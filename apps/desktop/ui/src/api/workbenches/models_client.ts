import type {
  ProbedModel,
  ModelPricing,
  ModelCatalogResult,
  ProviderConfigRecord,
  LocalModelFolder,
  LocalModelDescriptor,
  LocalHardwareProfile,
  LocalModelPreflightResponse,
  LoadedModelInstance,
  LocalModelLoadProfile,
  OAuthAuthorizeResult,
  OAuthConnectionMetadata,
  OAuthStatusResult,
  TerminalSession,
  GitHubAuthStatusResponse,
} from '@/types/domain';
import { fromDaemon } from '../normalizers';
import type { DaemonTransport } from './quant_client';

export class ModelsClient {
  constructor(private transport: DaemonTransport) {}

  async getProviderRouting(
    service: string
  ): Promise<{ service_type: string; round_robin: boolean; account_order: string[] }> {
    return this.transport.request('v1.providers.routing.get', { service_type: service });
  }

  async saveProviderRouting(
    service: string,
    roundRobin: boolean,
    accountOrder: string[]
  ): Promise<{ service_type: string; round_robin: boolean; account_order: string[] }> {
    return this.transport.request('v1.providers.routing.save', {
      service_type: service,
      round_robin: roundRobin,
      account_order: accountOrder,
    });
  }

  async reserveProviderAccount(
    service: string,
    model: string,
    reasoningEffort?: string
  ): Promise<{ account_id: string | null }> {
    return this.transport.request('v1.providers.route', {
      service_type: service,
      model,
      reasoning_effort: reasoningEffort,
    });
  }

  async listProviders(): Promise<any[]> {
    return this.transport.request<any[]>('v1.providers.list', {});
  }

  async getProvider(providerId: string): Promise<ProviderConfigRecord> {
    return this.transport.request<ProviderConfigRecord>('v1.providers.get', { id: providerId });
  }

  async saveProvider(provider: any): Promise<{ saved: boolean; id: string }> {
    return this.transport.request<{ saved: boolean; id: string }>('v1.providers.save', provider);
  }

  async deleteProvider(providerId: string): Promise<{ deleted: boolean; id: string }> {
    return this.transport.request<{ deleted: boolean; id: string }>('v1.providers.delete', {
      id: providerId,
    });
  }

  async getOAuthAuthorizeUrl(params?: {
    provider_id?: string;
    service_type?: string;
    client_id?: string;
    redirect_uri?: string;
    scope?: string;
    auth_endpoint?: string;
    token_endpoint?: string;
  }): Promise<OAuthAuthorizeResult> {
    return this.transport.request<OAuthAuthorizeResult>('v1.oauth.authorize', params || {});
  }

  async exchangeOAuthCode(params: {
    code_or_url: string;
    code_verifier: string;
    provider_id?: string;
    service_type?: string;
    client_id?: string;
    redirect_uri?: string;
    token_endpoint?: string;
  }): Promise<OAuthConnectionMetadata> {
    return this.transport.request<OAuthConnectionMetadata>('v1.oauth.exchange', params);
  }

  async refreshOAuthToken(provider_id: string = 'openai'): Promise<OAuthConnectionMetadata> {
    return this.transport.request<OAuthConnectionMetadata>('v1.oauth.refresh', { provider_id });
  }

  async getOAuthToken(provider_id: string = 'openai'): Promise<OAuthConnectionMetadata | null> {
    return this.transport.request<OAuthConnectionMetadata | null>('v1.oauth.get', { provider_id });
  }

  async deleteOAuthToken(
    provider_id: string = 'openai'
  ): Promise<{ deleted: boolean; provider_id: string }> {
    return this.transport.request<{ deleted: boolean; provider_id: string }>('v1.oauth.delete', {
      provider_id,
    });
  }

  async cancelOAuth(): Promise<{ cancelled: boolean }> {
    return this.transport.request('v1.oauth.cancel', {});
  }

  async getOAuthStatus(): Promise<OAuthStatusResult> {
    return this.transport.request<OAuthStatusResult>('v1.oauth.status', {});
  }

  async startGitHubLogin(): Promise<TerminalSession> {
    return fromDaemon<TerminalSession>(await this.transport.request<unknown>('v1.github.login', {}));
  }

  async getGitHubAuthStatus(): Promise<GitHubAuthStatusResponse> {
    return this.transport.request<GitHubAuthStatusResponse>('v1.github.auth_status', {});
  }

  async getGitHubViewer(): Promise<any> {
    return this.transport.request('v1.github.viewer', {});
  }

  async probeModels(
    baseUrl: string,
    providerType?: string,
    apiKey?: string,
    providerId?: string
  ): Promise<{ models: ProbedModel[]; count: number; base_url: string }> {
    const probePromise = this.transport.request<{
      models: ProbedModel[];
      count: number;
      base_url: string;
    }>('v1.models.probe', {
      base_url: baseUrl,
      provider_type: providerType,
      api_key: apiKey,
      provider_id: providerId,
    });
    let timer: ReturnType<typeof setTimeout> | undefined;
    const timeoutPromise = new Promise<never>((_, reject) => {
      timer = setTimeout(() => reject(new Error('Model probe timed out')), 30000);
    });
    try {
      return await Promise.race([probePromise, timeoutPromise]);
    } finally {
      clearTimeout(timer);
    }
  }

  async getModelCatalog(providerType?: string, providerId?: string): Promise<ModelCatalogResult> {
    return this.transport.request<ModelCatalogResult>('v1.models.catalog', {
      provider_type: providerType,
      provider_id: providerId,
    });
  }

  async getModelPricing(
    modelId: string,
    tokens?: {
      input_tokens?: number;
      output_tokens?: number;
      cache_read_tokens?: number;
      cache_write_tokens?: number;
    }
  ): Promise<{
    model_id: string;
    pricing: ModelPricing | null;
    estimated_cost_usd: number | null;
  }> {
    return this.transport.request<{
      model_id: string;
      pricing: ModelPricing | null;
      estimated_cost_usd: number | null;
    }>('v1.models.pricing', {
      model_id: modelId,
      ...(tokens || {}),
    });
  }

  async listLocalFolders(): Promise<LocalModelFolder[]> {
    return this.transport.request<LocalModelFolder[]>('v1.local_models.folders.list', {});
  }

  async addLocalFolder(
    folderPath: string,
    label?: string
  ): Promise<{
    folder: LocalModelFolder;
    models_discovered: number;
    models: LocalModelDescriptor[];
  }> {
    return this.transport.request('v1.local_models.folders.add', {
      folder_path: folderPath,
      label,
    });
  }

  async removeLocalFolder(id: string): Promise<{ deleted: boolean; id: string }> {
    return this.transport.request('v1.local_models.folders.remove', { id });
  }

  async scanLocalModels(
    folderId?: string
  ): Promise<{
    scanned_folders: number;
    models_count: number;
    models: LocalModelDescriptor[];
    hardware: LocalHardwareProfile;
    scanned_at: number;
  }> {
    return this.transport.request('v1.local_models.scan', {
      folder_id: folderId,
    });
  }

  async listLocalModels(folderId?: string): Promise<{ models: LocalModelDescriptor[] }> {
    return this.transport.request('v1.local_models.list', {
      folder_id: folderId,
    });
  }

  async getLocalHardware(): Promise<LocalHardwareProfile> {
    return this.transport.request<LocalHardwareProfile>('v1.local_models.hardware', {});
  }

  async preflightLocalModel(
    modelPath: string,
    requestedContext?: number
  ): Promise<LocalModelPreflightResponse> {
    return this.transport.request<LocalModelPreflightResponse>('v1.local_models.preflight', {
      model_path: modelPath,
      requested_context: requestedContext,
    });
  }

  async getLocalModelLoadProfile(
    modelPath: string
  ): Promise<{ profile: LocalModelLoadProfile }> {
    return this.transport.request('v1.local_models.profile.get', { model_path: modelPath });
  }

  async saveLocalModelLoadProfile(
    profile: Omit<LocalModelLoadProfile, 'revision' | 'updated_at'>
  ): Promise<{ profile: LocalModelLoadProfile }> {
    return this.transport.request('v1.local_models.profile.save', profile);
  }

  async ensureLocalModel(
    modelPath: string,
    forceReload?: boolean
  ): Promise<{ instance: LoadedModelInstance; drift_detected: boolean; reloaded: boolean }> {
    return this.transport.request<{
      instance: LoadedModelInstance;
      drift_detected: boolean;
      reloaded: boolean;
    }>('v1.local_models.ensure', {
      model_path: modelPath,
      force_reload: forceReload ?? false,
    });
  }

  async verifyLocalModelGeneration(
    modelPathOrName?: string
  ): Promise<{ verified: boolean; model: string; test_output: string; latency_ms: number }> {
    return this.transport.request<{
      verified: boolean;
      model: string;
      test_output: string;
      latency_ms: number;
    }>('v1.local_models.verify', {
      model_path: modelPathOrName,
    });
  }

  async loadLocalModel(modelPath: string): Promise<LoadedModelInstance> {
    return this.transport.request<LoadedModelInstance>('v1.local_models.load', {
      model_path: modelPath,
    });
  }

  async unloadLocalModel(
    modelPathOrId: string
  ): Promise<{ evicted: boolean; model: string }> {
    return this.transport.request<{ evicted: boolean; model: string }>('v1.local_models.unload', {
      model_path: modelPathOrId,
    });
  }

  async listLoadedInstances(): Promise<{ instances: LoadedModelInstance[] }> {
    return this.transport.request<{ instances: LoadedModelInstance[] }>(
      'v1.local_models.instances',
      {}
    );
  }

  async listClientKeys(): Promise<any[]> {
    return this.transport.request<any[]>('v1.keys.list', {});
  }

  async generateClientKey(name?: string): Promise<any> {
    return this.transport.request<any>('v1.keys.generate', { name });
  }

  async revokeClientKey(keyId: string): Promise<{ revoked: boolean }> {
    return this.transport.request<{ revoked: boolean }>('v1.keys.revoke', { key_id: keyId });
  }

  async checkLlmStatus(): Promise<{
    configured: boolean;
    active_provider: string | null;
    message: string;
  }> {
    return this.transport.request<{
      configured: boolean;
      active_provider: string | null;
      message: string;
    }>('v1.llm.status', {});
  }
}
