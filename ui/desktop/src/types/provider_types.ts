export type ProviderAuthType = 'oauth' | 'api_key';

export type ProviderServiceId =
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
  modelId: string;
  modelName: string;
  providerId: ProviderServiceId;
  providerName: string;
  accountName: string;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  estimatedCostUsd: number;
  requestCount: number;
  quotaLimitTokens: number; // e.g. 1_000_000 daily
  quotaUsedTokens: number;
  quotaPercent: number; // 0 - 100
  rateLimitStatus: 'normal' | 'throttled' | 'exhausted';
  resetPeriod: string; // e.g. "Resets daily at 00:00 UTC"
  lastUsedAt: string;
}
