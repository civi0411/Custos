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

export interface ModelSelectorProps {
  currentModel?: string;
  onSelectModel?: (modelId: string, effort?: string, accountId?: string) => void;
  className?: string;
  compact?: boolean;
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
