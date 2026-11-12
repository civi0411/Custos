import type {
  ProviderItem,
  ClientApiKey,
  ConnectedAccount,
  ProviderServiceId,
} from '@/types';
import type {
  ProviderConfigRecord,
  OAuthTokenRecord,
} from '@/types/domain';

export function mapBackendProvider(p: any): ProviderItem {
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
    statusLabel: p.status === 'configured' ? 'Configured' : p.status_label || (status === 'primary' ? 'Enabled' : 'Offline'),
    badgeColor: status === 'primary' ? '#10b981' : status === 'standby' ? '#3b82f6' : '#6b7280',
    apiKey: p.api_key_masked || (p.api_key && p.api_key.trim().length > 0 ? `${p.api_key.slice(0, 6)}••••••••` : 'Not configured'),
    quotaUsed: p.quota_used || undefined,
    quotaTotal: p.quota_total || undefined,
    quotaPercent: typeof p.quota_percent === 'number' ? p.quota_percent : undefined,
    endpoint: p.endpoint_url || p.endpoint || undefined,
    defaultModel: p.default_model || p.model || undefined,
    contextWindow: p.context_window || undefined,
    fastMode: p.fast_mode || undefined,
    latency: typeof p.latency_ms === 'number' ? `${p.latency_ms}ms` : 'Not measured',
    iconType,
  };
}

export function mapBackendKey(k: any): ClientApiKey {
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

export const PROVIDER_LABELS: Record<string, string> = {
  antigravity: 'Google Antigravity',
  copilot: 'GitHub Copilot',
  gemini: 'Google Gemini',
  openai: 'OpenAI / Codex',
  anthropic: 'Anthropic Claude',
  deepseek: 'DeepSeek',
  openrouter: 'OpenRouter Gateway',
  groq: 'Groq Cloud',
  cerebras: 'Cerebras',
  nvidia: 'NVIDIA NIM',
  mistral: 'Mistral AI',
  together: 'Together AI',
  moonshot: 'Moonshot Kimi',
  zhipu: 'Zhipu GLM / Z.ai',
  minimax: 'MiniMax',
  alibaba: 'Alibaba Qwen',
  qwen: 'Alibaba Qwen',
  xai: 'xAI (Grok)',
  sambanova: 'SambaNova Cloud',
  fireworks: 'Fireworks AI',
  siliconflow: 'SiliconFlow',
  nebius: 'Nebius AI Studio',
  hyperbolic: 'Hyperbolic',
  cohere: 'Cohere',
  perplexity: 'Perplexity AI',
  local: 'Local Inference',
  lmstudio: 'LM Studio',
  custom: 'Custom Gateway',
};

export const normalizeProviderServiceId = (service?: string): ProviderServiceId => {
  const s = (service || '').toLowerCase();
  if (s.includes('antigravity')) return 'antigravity';
  if (s.includes('copilot')) return 'copilot';
  if (s.includes('anthropic') || s.includes('claude')) return 'anthropic';
  if (s.includes('openai') || s.includes('gpt') || s.includes('codex')) return 'openai';
  if (s.includes('gemini') || s.includes('google')) return 'gemini';
  if (s.includes('deepseek')) return 'deepseek';
  if (s.includes('openrouter')) return 'openrouter';
  if (s.includes('groq')) return 'groq';
  if (s.includes('cerebras')) return 'cerebras';
  if (s.includes('nvidia') || s.includes('nemotron')) return 'nvidia';
  if (s.includes('mistral') || s.includes('codestral')) return 'mistral';
  if (s.includes('together')) return 'together';
  if (s.includes('moonshot') || s.includes('kimi')) return 'moonshot';
  if (s.includes('zhipu') || s.includes('glm')) return 'zhipu';
  if (s.includes('minimax')) return 'minimax';
  if (s.includes('alibaba') || s.includes('qwen') || s.includes('dashscope')) return 'alibaba';
  if (s.includes('xai') || s.includes('grok')) return 'xai';
  if (s.includes('sambanova')) return 'sambanova';
  if (s.includes('fireworks')) return 'fireworks';
  if (s.includes('siliconflow')) return 'siliconflow';
  if (s.includes('nebius')) return 'nebius';
  if (s.includes('hyperbolic')) return 'hyperbolic';
  if (s.includes('cohere')) return 'cohere';
  if (s.includes('perplexity') || s.includes('sonar')) return 'perplexity';
  if (s.includes('lmstudio') || s.includes('lm-studio')) return 'lmstudio';
  if (s.includes('local') || s.includes('ollama')) return 'local';
  return 'custom';
};

export const defaultProbeBaseUrl = (service: ProviderServiceId): string | undefined => {
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
    case 'groq':
      return 'https://api.groq.com/openai/v1';
    case 'cerebras':
      return 'https://api.cerebras.ai/v1';
    case 'nvidia':
      return 'https://integrate.api.nvidia.com/v1';
    case 'mistral':
      return 'https://api.mistral.ai/v1';
    case 'together':
      return 'https://api.together.xyz/v1';
    case 'moonshot':
      return 'https://api.moonshot.cn/v1';
    case 'zhipu':
      return 'https://open.bigmodel.cn/api/paas/v4';
    case 'minimax':
      return 'https://api.minimax.chat/v1';
    case 'alibaba':
    case 'qwen':
      return 'https://dashscope.aliyuncs.com/compatible-mode/v1';
    case 'xai':
      return 'https://api.x.ai/v1';
    case 'sambanova':
      return 'https://api.sambanova.ai/v1';
    case 'fireworks':
      return 'https://api.fireworks.ai/inference/v1';
    case 'siliconflow':
      return 'https://api.siliconflow.cn/v1';
    case 'nebius':
      return 'https://api.studio.nebius.ai/v1';
    case 'hyperbolic':
      return 'https://api.hyperbolic.xyz/v1';
    case 'openrouter':
      return 'https://openrouter.ai/api/v1';
    case 'perplexity':
      return 'https://api.perplexity.ai';
    case 'cohere':
      return 'https://api.cohere.com/v2';
    case 'lmstudio':
      return 'http://127.0.0.1:1234/v1';
    case 'local':
      return 'http://localhost:11434';
    default:
      return undefined;
  }
};

export const isProviderConnected = (p: ProviderConfigRecord): boolean => {
  const status = (p.status || '').toLowerCase();
  const statusConnected =
    status === 'configured' ||
    status === 'active' ||
    status === 'primary' ||
    status === 'standby';
  const localProvider =
    normalizeProviderServiceId(p.service_type) === 'local' ||
    normalizeProviderServiceId(p.service_type) === 'lmstudio';
  return statusConnected && (localProvider || p.api_key_masked !== 'none');
};

export const safeIsoFromMillis = (value?: number): string => {
  if (!value || value > 253402300799000) {
    return new Date().toISOString();
  }
  return new Date(value).toISOString();
};

export const tokenLooksOauth = (token?: OAuthTokenRecord | null): boolean =>
  Boolean(token?.has_refresh_token || token?.scope?.trim());

export const uniqueModels = (models: string[]): string[] => {
  const seen = new Set<string>();
  return models.filter((model) => {
    const key = model.trim().toLowerCase();
    if (!key || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
};

export const AI_LOCAL_ACCOUNT_ID = 'ai-local-auto';
export const AI_LOCAL_ENDPOINT = 'http://localhost:11434';

export const isTransientAiLocalAccount = (account?: ConnectedAccount): boolean =>
  account?.id === AI_LOCAL_ACCOUNT_ID;
