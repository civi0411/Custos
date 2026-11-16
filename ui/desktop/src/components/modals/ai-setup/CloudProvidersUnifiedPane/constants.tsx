import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import type { CustomSelectOption } from '@/components/common/CustomSelect';
import type { ProviderServiceId } from '@/types';

export const providerOptions: CustomSelectOption[] = [
  {
    value: 'anthropic',
    label: 'Anthropic Claude',
    sublabel: 'Claude 3.7 Sonnet, 3.5 Haiku',
    icon: <ProviderBrandIcon provider="anthropic" size={16} />,
  },
  {
    value: 'openai',
    label: 'OpenAI / Codex',
    sublabel: 'GPT-4o, o3-mini, o1',
    icon: <ProviderBrandIcon provider="openai" size={16} />,
  },
  {
    value: 'gemini',
    label: 'Google Cloud / Gemini',
    sublabel: 'Gemini 2.5 Pro, Flash (1M tokens)',
    icon: <ProviderBrandIcon provider="gemini" size={16} />,
  },
  {
    value: 'deepseek',
    label: 'DeepSeek',
    sublabel: 'DeepSeek V3, R1 Reasoner',
    icon: <ProviderBrandIcon provider="deepseek" size={16} />,
  },
  {
    value: 'openrouter',
    label: 'OpenRouter',
    sublabel: '200+ AI models unified gateway',
    icon: <ProviderBrandIcon provider="openrouter" size={16} />,
  },
  {
    value: 'groq',
    label: 'Groq LPU',
    sublabel: 'Ultra-fast inference (Llama, Qwen)',
    icon: <ProviderBrandIcon provider="groq" size={16} />,
  },
  {
    value: 'cerebras',
    label: 'Cerebras',
    sublabel: 'World-record fast token generation',
    icon: <ProviderBrandIcon provider="cerebras" size={16} />,
  },
  {
    value: 'nvidia',
    label: 'NVIDIA NIM',
    sublabel: 'Free tier Nemotron, Llama 3, DeepSeek',
    icon: <ProviderBrandIcon provider="nvidia" size={16} />,
  },
  {
    value: 'mistral',
    label: 'Mistral AI',
    sublabel: 'Codestral, Mistral Large',
    icon: <ProviderBrandIcon provider="mistral" size={16} />,
  },
  {
    value: 'together',
    label: 'Together AI',
    sublabel: 'Open-source frontier models',
    icon: <ProviderBrandIcon provider="together" size={16} />,
  },
  {
    value: 'moonshot',
    label: 'Moonshot Kimi',
    sublabel: 'Kimi K3 (1M context window)',
    icon: <ProviderBrandIcon provider="moonshot" size={16} />,
  },
  {
    value: 'zhipu',
    label: 'Zhipu GLM / Z.ai',
    sublabel: 'GLM-5.3, GLM-4 coding models',
    icon: <ProviderBrandIcon provider="zhipu" size={16} />,
  },
  {
    value: 'minimax',
    label: 'MiniMax',
    sublabel: 'MiniMax M3.1 Flash, M3',
    icon: <ProviderBrandIcon provider="minimax" size={16} />,
  },
  {
    value: 'alibaba',
    label: 'Alibaba Qwen (DashScope)',
    sublabel: 'Qwen 2.5 Coder 32B, Qwen 3.8',
    icon: <ProviderBrandIcon provider="alibaba" size={16} />,
  },
  {
    value: 'perplexity',
    label: 'Perplexity AI',
    sublabel: 'Sonar search-grounded models',
    icon: <ProviderBrandIcon provider="perplexity" size={16} />,
  },
  {
    value: 'xai',
    label: 'xAI (Grok)',
    sublabel: 'Grok 4.6, Grok Code Fast',
    icon: <ProviderBrandIcon provider="xai" size={16} />,
  },
  {
    value: 'sambanova',
    label: 'SambaNova Cloud',
    sublabel: 'Ultra-fast Llama 3.3 70B & DeepSeek R1',
    icon: <ProviderBrandIcon provider="sambanova" size={16} />,
  },
  {
    value: 'fireworks',
    label: 'Fireworks AI',
    sublabel: 'High-speed serverless inference gateway',
    icon: <ProviderBrandIcon provider="fireworks" size={16} />,
  },
  {
    value: 'siliconflow',
    label: 'SiliconFlow',
    sublabel: 'Frontier open models (DeepSeek, Qwen)',
    icon: <ProviderBrandIcon provider="siliconflow" size={16} />,
  },
  {
    value: 'nebius',
    label: 'Nebius AI Studio',
    sublabel: 'Enterprise high-performance GPU serving',
    icon: <ProviderBrandIcon provider="nebius" size={16} />,
  },
  {
    value: 'hyperbolic',
    label: 'Hyperbolic',
    sublabel: 'Decentralized fast compute & open LLMs',
    icon: <ProviderBrandIcon provider="hyperbolic" size={16} />,
  },
  {
    value: 'cohere',
    label: 'Cohere',
    sublabel: 'Command R+, Command R Enterprise',
    icon: <ProviderBrandIcon provider="cohere" size={16} />,
  },
  {
    value: 'custom',
    label: 'Custom (OpenAI-Compatible)',
    sublabel: 'Local/Remote Gateway (vLLM, 9Router, TGI)',
    icon: <ProviderBrandIcon provider="server" size={16} />,
  },
];

export const defaultEndpoints: Record<string, string> = {
  openrouter: 'https://openrouter.ai/api/v1',
  groq: 'https://api.groq.com/openai/v1',
  cerebras: 'https://api.cerebras.ai/v1',
  nvidia: 'https://integrate.api.nvidia.com/v1',
  together: 'https://api.together.xyz/v1',
  mistral: 'https://api.mistral.ai/v1',
  moonshot: 'https://api.moonshot.cn/v1',
  zhipu: 'https://open.bigmodel.cn/api/paas/v4',
  minimax: 'https://api.minimax.chat/v1',
  alibaba: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
  perplexity: 'https://api.perplexity.ai',
  xai: 'https://api.x.ai/v1',
  sambanova: 'https://api.sambanova.ai/v1',
  fireworks: 'https://api.fireworks.ai/inference/v1',
  siliconflow: 'https://api.siliconflow.cn/v1',
  nebius: 'https://api.studio.nebius.ai/v1',
  hyperbolic: 'https://api.hyperbolic.xyz/v1',
  cohere: 'https://api.cohere.com/v2',
  custom: 'http://localhost:20128/v1',
};

export const fallbackDefaultModels: Record<string, CustomSelectOption[]> = {
  openrouter: [
    { value: 'openrouter/auto', label: 'Auto (Best for Prompt)' },
    { value: 'anthropic/claude-3.7-sonnet', label: 'Claude 3.7 Sonnet' },
    { value: 'google/gemini-2.5-pro', label: 'Gemini 2.5 Pro' },
    { value: 'openai/gpt-4o', label: 'GPT-4o' },
  ],
  groq: [
    { value: 'llama-3.3-70b-versatile', label: 'Llama 3.3 70B Versatile' },
    { value: 'llama-3.1-8b-instant', label: 'Llama 3.1 8B Instant' },
  ],
  cerebras: [
    { value: 'llama-3.3-70b', label: 'Llama 3.3 70B (Fast)' },
  ],
  deepseek: [
    { value: 'deepseek-chat', label: 'DeepSeek V3' },
    { value: 'deepseek-reasoner', label: 'DeepSeek R1 Reasoner' },
  ],
  anthropic: [
    { value: 'claude-3-7-sonnet-20250219', label: 'Claude 3.7 Sonnet' },
    { value: 'claude-3-5-haiku-20241022', label: 'Claude 3.5 Haiku' },
  ],
  openai: [
    { value: 'gpt-4o', label: 'GPT-4o' },
    { value: 'o3-mini', label: 'o3-mini' },
  ],
  gemini: [
    { value: 'gemini-2.5-pro', label: 'Gemini 2.5 Pro' },
    { value: 'gemini-2.5-flash', label: 'Gemini 2.5 Flash' },
  ],
  custom: [
    { value: 'default', label: 'Default Model' },
  ],
};

export interface OAuthProviderCard {
  id: ProviderServiceId;
  name: string;
  sub: string;
  tag: string;
  tagColor: string;
}

export const oauthProvidersList: OAuthProviderCard[] = [
  {
    id: 'antigravity',
    name: 'Google Account (Unified)',
    sub: '1 Login unlocks Antigravity (Claude 5.5, Gemini 3.8), Gemini-CLI & Gemini API · Multi-Account Pooling',
    tag: 'Unified Umbrella',
    tagColor: 'text-emerald-400 bg-emerald-500/10 border-emerald-500/25',
  },
  {
    id: 'anthropic',
    name: 'Anthropic Claude Code',
    sub: 'Official Claude Code OAuth PKCE · Claude 3.7 Sonnet & 3.5 Haiku',
    tag: 'Claude Pro/Team',
    tagColor: 'text-amber-400 bg-amber-500/10 border-amber-500/25',
  },
  {
    id: 'openai',
    name: 'OpenAI Codex',
    sub: 'ChatGPT Plus/Team OAuth PKCE · GPT-4o, o3-mini, o1',
    tag: 'ChatGPT Plus',
    tagColor: 'text-indigo-400 bg-indigo-500/10 border-indigo-500/25',
  },
  {
    id: 'copilot',
    name: 'GitHub Copilot',
    sub: 'Device Authorization Code · Copilot Chat & Claude 3.5 Sonnet',
    tag: 'Device Flow',
    tagColor: 'text-sky-400 bg-sky-500/10 border-sky-500/25',
  },
];
