export type ProviderAuthType = 'oauth' | 'api_key';

export type ProviderServiceId =
  | 'antigravity'
  | 'anthropic'
  | 'openai'
  | 'gemini'
  | 'deepseek'
  | 'local'
  | 'copilot'
  | 'openrouter'
  | 'groq'
  | 'custom';

export type AccountStatus = 'active' | 'rate_limited' | 'quota_exhausted' | 'offline';

export interface ConnectedAccount {
  id: string;
  providerId: ProviderServiceId;
  providerName: string;
  accountName: string;
  authType: ProviderAuthType;
  apiKeyMasked?: string;
  endpointUrl?: string;
  oauthEmail?: string;
  oauthTier?: string;
  oauthExpiresAt?: string;
  status: AccountStatus;
  statusLabel?: string;
  badgeColor?: string;
  defaultModel: string;
  supportedModels: string[];
  catalogModels?: import('./domain').ModelCatalogOption[];
  catalogSource?: 'live' | 'cached' | 'unavailable';
  catalogError?: string;
  catalogFetchedAt?: number;
  contextWindow?: number;
  fastMode?: boolean;
  latencyMs?: number;
  createdAt: string;
  lastUsedAt?: string;
}

export type ComboStrategy = 'fallback' | 'round_robin' | 'cost_optimized';

export interface ComboModelTarget {
  modelId: string;
  modelName: string;
  providerId: ProviderServiceId;
  providerName: string;
  priority: number; // 1 = Primary, 2 = Fallback 1, 3 = Fallback 2...
  weight?: number; // for round_robin
}

export interface ModelCombo {
  id: string;
  name: string;
  description: string;
  strategy: ComboStrategy;
  targets: ComboModelTarget[];
  enabled: boolean;
  createdAt: string;
  updatedAt?: string;
}

export interface UsageRecord {
  id: string;
  account_id: string | null;
  model: string | null;
  status: string;
  recorded_at: string;
  input_tokens: number | null;
  output_tokens: number | null;
  total_tokens: number | null;
  rate_limited: boolean;
}

export interface ProviderQuotaSnapshot {
  account_id: string;
  status: 'available' | 'unavailable' | 'unsupported' | 'not_applicable';
  source?: 'provider' | 'provider_catalog';
  windows: { name: string; used_percent: number; resets_at: string | number | null }[];
  fetched_at: string;
  error?: string;
}

export interface ProviderUsageReport {
  records: UsageRecord[];
  quotas: ProviderQuotaSnapshot[];
  fetched_at: string;
}
