import React from 'react';
import OpenAI from '~icons/simple-icons/openai';
import Claude from '~icons/logos/claude-icon';
import Gemini from '~icons/logos/google-gemini-icon';
import DeepSeek from '~icons/logos/deepseek-icon';
import Copilot from '~icons/simple-icons/githubcopilot';
import Antigravity from '~icons/logos/antigravity';
import Ollama from '~icons/simple-icons/ollama';
import Server from '~icons/lucide/server';
import Cpu from '~icons/lucide/cpu';
import {
  Bot,
  Terminal,
  Zap,
  Sparkles,
  ShieldCheck,
  Flame,
  Compass,
  Code,
} from 'lucide-react';

import Mistral from '~icons/logos/mistral-ai-icon';
import Cerebras from '~icons/logos/cerebras-icon';
import Nvidia from '~icons/logos/nvidia';
import Together from '~icons/logos/togetherai-icon';
import Moonshot from '~icons/logos/moonshot-ai-icon';
import MiniMax from '~icons/logos/minimax-icon';
import Qwen from '~icons/logos/qwen-icon';
import OpenRouter from '~icons/simple-icons/openrouter';
import Perplexity from '~icons/logos/perplexity-icon';

type IconProps = { size?: number; className?: string };

export const GroqIcon: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...props}>
    <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" fill="#F55036" stroke="#F55036" />
  </svg>
);

export const XAIIcon: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="currentColor" {...props}>
    <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
  </svg>
);

export const ZhipuIcon: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="none" {...props}>
    <path d="M12 2L21 7.2V16.8L12 22L3 16.8V7.2L12 2Z" stroke="#2563EB" strokeWidth="2" strokeLinejoin="round" />
    <path d="M12 6L17 8.9V14.7L12 17.6L7 14.7V8.9L12 6Z" fill="#2563EB" fillOpacity="0.25" stroke="#3B82F6" strokeWidth="1.5" />
    <circle cx="12" cy="11.8" r="2.5" fill="#60A5FA" />
  </svg>
);

export const CohereIcon: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="none" {...props}>
    <circle cx="9" cy="12" r="5" stroke="#D97757" strokeWidth="2.5" />
    <circle cx="15" cy="12" r="5" stroke="#D97757" strokeWidth="2.5" />
    <path d="M9 12C9 13.65 10.35 15 12 15C13.65 15 15 13.65 15 12C15 10.35 13.65 9 12 9C10.35 9 9 10.35 9 12Z" fill="#D97757" />
  </svg>
);

export const SambaNovaIcon: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="none" {...props}>
    <rect x="3" y="3" width="18" height="18" rx="4" stroke="#10B981" strokeWidth="2" />
    <circle cx="8.5" cy="8.5" r="2" fill="#10B981" />
    <circle cx="15.5" cy="15.5" r="2" fill="#10B981" />
    <path d="M8.5 15.5L15.5 8.5" stroke="#34D399" strokeWidth="2" strokeLinecap="round" />
  </svg>
);

export const FireworksIcon: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="#EF4444" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...props}>
    <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
    <circle cx="12" cy="12" r="3" fill="#EF4444" fillOpacity="0.4" />
  </svg>
);

export const SiliconFlowIcon: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="none" {...props}>
    <rect x="3" y="3" width="8" height="8" rx="2" fill="#3B82F6" />
    <rect x="13" y="3" width="8" height="8" rx="2" fill="#60A5FA" />
    <rect x="3" y="13" width="8" height="8" rx="2" fill="#2563EB" />
    <rect x="13" y="13" width="8" height="8" rx="2" fill="#1D4ED8" />
  </svg>
);

export const LMStudioIcon: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...props}>
    <rect x="3" y="3" width="18" height="18" rx="4" stroke="#8B5CF6" strokeWidth="2" />
    <path d="M7 8v8h4" stroke="#A78BFA" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    <path d="M13 16V8l2.5 3.5L18 8v8" stroke="#A78BFA" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

const brands: Record<string, React.ComponentType<React.SVGProps<SVGSVGElement>>> = {
  openai: OpenAI,
  codex: OpenAI,
  anthropic: Claude,
  claude: Claude,
  gemini: Gemini,
  google: Gemini,
  deepseek: DeepSeek,
  copilot: Copilot,
  antigravity: Antigravity,
  ollama: Ollama,
  local: Ollama,
  lmstudio: LMStudioIcon,
  server: Server,
  custom: Server,
  groq: GroqIcon,
  cerebras: Cerebras,
  mistral: Mistral,
  nvidia: Nvidia,
  together: Together,
  moonshot: Moonshot,
  kimi: Moonshot,
  zhipu: ZhipuIcon,
  glm: ZhipuIcon,
  minimax: MiniMax,
  alibaba: Qwen,
  qwen: Qwen,
  xai: XAIIcon,
  grok: XAIIcon,
  openrouter: OpenRouter,
  perplexity: Perplexity,
  cohere: CohereIcon,
  sambanova: SambaNovaIcon,
  fireworks: FireworksIcon,
  siliconflow: SiliconFlowIcon,
};

export interface ProviderBrandMeta {
  id: string;
  name: string;
  brandColor: string;
  bgLight: string;
  borderColor: string;
  badgeText: string;
  category: 'oauth' | 'api_key' | 'local' | 'compatible';
}

export function getProviderBrandMeta(rawProvider: string): ProviderBrandMeta {
  const p = (rawProvider || '').toLowerCase().trim();
  switch (p) {
    case 'anthropic':
    case 'claude':
      return {
        id: 'anthropic',
        name: 'Anthropic Claude',
        brandColor: '#D97757',
        bgLight: 'rgba(217, 119, 87, 0.12)',
        borderColor: 'rgba(217, 119, 87, 0.3)',
        badgeText: 'Claude',
        category: 'oauth',
      };
    case 'openai':
    case 'codex':
      return {
        id: 'openai',
        name: 'OpenAI / Codex',
        brandColor: '#10A37F',
        bgLight: 'rgba(16, 163, 127, 0.12)',
        borderColor: 'rgba(16, 163, 127, 0.3)',
        badgeText: 'OpenAI',
        category: 'oauth',
      };
    case 'antigravity':
      return {
        id: 'antigravity',
        name: 'Google Antigravity',
        brandColor: '#F59E0B',
        bgLight: 'rgba(245, 158, 11, 0.12)',
        borderColor: 'rgba(245, 158, 11, 0.3)',
        badgeText: 'Google AG',
        category: 'oauth',
      };
    case 'gemini':
    case 'google':
      return {
        id: 'gemini',
        name: 'Google Gemini',
        brandColor: '#4285F4',
        bgLight: 'rgba(66, 133, 244, 0.12)',
        borderColor: 'rgba(66, 133, 244, 0.3)',
        badgeText: 'Gemini',
        category: 'api_key',
      };
    case 'deepseek':
      return {
        id: 'deepseek',
        name: 'DeepSeek',
        brandColor: '#4D6BFE',
        bgLight: 'rgba(77, 107, 254, 0.12)',
        borderColor: 'rgba(77, 107, 254, 0.3)',
        badgeText: 'DeepSeek',
        category: 'api_key',
      };
    case 'groq':
      return {
        id: 'groq',
        name: 'Groq LPU',
        brandColor: '#F55036',
        bgLight: 'rgba(245, 80, 54, 0.12)',
        borderColor: 'rgba(245, 80, 54, 0.3)',
        badgeText: 'Groq',
        category: 'api_key',
      };
    case 'cerebras':
      return {
        id: 'cerebras',
        name: 'Cerebras',
        brandColor: '#DE1A35',
        bgLight: 'rgba(222, 26, 53, 0.12)',
        borderColor: 'rgba(222, 26, 53, 0.3)',
        badgeText: 'Cerebras',
        category: 'api_key',
      };
    case 'mistral':
      return {
        id: 'mistral',
        name: 'Mistral AI',
        brandColor: '#FF7000',
        bgLight: 'rgba(255, 112, 0, 0.12)',
        borderColor: 'rgba(255, 112, 0, 0.3)',
        badgeText: 'Mistral',
        category: 'api_key',
      };
    case 'nvidia':
      return {
        id: 'nvidia',
        name: 'NVIDIA NIM',
        brandColor: '#76B900',
        bgLight: 'rgba(118, 185, 0, 0.12)',
        borderColor: 'rgba(118, 185, 0, 0.3)',
        badgeText: 'NVIDIA',
        category: 'api_key',
      };
    case 'together':
      return {
        id: 'together',
        name: 'Together AI',
        brandColor: '#3B82F6',
        bgLight: 'rgba(59, 130, 246, 0.12)',
        borderColor: 'rgba(59, 130, 246, 0.3)',
        badgeText: 'Together',
        category: 'api_key',
      };
    case 'moonshot':
    case 'kimi':
      return {
        id: 'moonshot',
        name: 'Moonshot Kimi',
        brandColor: '#06B6D4',
        bgLight: 'rgba(6, 182, 212, 0.12)',
        borderColor: 'rgba(6, 182, 212, 0.3)',
        badgeText: 'Kimi',
        category: 'api_key',
      };
    case 'zhipu':
    case 'glm':
      return {
        id: 'zhipu',
        name: 'Zhipu GLM / Z.ai',
        brandColor: '#2563EB',
        bgLight: 'rgba(37, 99, 235, 0.12)',
        borderColor: 'rgba(37, 99, 235, 0.3)',
        badgeText: 'GLM',
        category: 'oauth',
      };
    case 'minimax':
      return {
        id: 'minimax',
        name: 'MiniMax',
        brandColor: '#8B5CF6',
        bgLight: 'rgba(139, 92, 246, 0.12)',
        borderColor: 'rgba(139, 92, 246, 0.3)',
        badgeText: 'MiniMax',
        category: 'api_key',
      };
    case 'alibaba':
    case 'qwen':
      return {
        id: 'alibaba',
        name: 'Alibaba Qwen',
        brandColor: '#FF6A00',
        bgLight: 'rgba(255, 106, 0, 0.12)',
        borderColor: 'rgba(255, 106, 0, 0.3)',
        badgeText: 'Qwen',
        category: 'api_key',
      };
    case 'xai':
    case 'grok':
      return {
        id: 'xai',
        name: 'xAI (Grok)',
        brandColor: '#E4E4E7',
        bgLight: 'rgba(228, 228, 231, 0.12)',
        borderColor: 'rgba(228, 228, 231, 0.3)',
        badgeText: 'xAI',
        category: 'api_key',
      };
    case 'openrouter':
      return {
        id: 'openrouter',
        name: 'OpenRouter',
        brandColor: '#6366F1',
        bgLight: 'rgba(99, 102, 241, 0.12)',
        borderColor: 'rgba(99, 102, 241, 0.3)',
        badgeText: 'OpenRouter',
        category: 'api_key',
      };
    case 'perplexity':
      return {
        id: 'perplexity',
        name: 'Perplexity AI',
        brandColor: '#20B2AA',
        bgLight: 'rgba(32, 178, 170, 0.12)',
        borderColor: 'rgba(32, 178, 170, 0.3)',
        badgeText: 'Perplexity',
        category: 'api_key',
      };
    case 'cohere':
      return {
        id: 'cohere',
        name: 'Cohere',
        brandColor: '#D97757',
        bgLight: 'rgba(217, 119, 87, 0.12)',
        borderColor: 'rgba(217, 119, 87, 0.3)',
        badgeText: 'Cohere',
        category: 'api_key',
      };
    case 'sambanova':
      return {
        id: 'sambanova',
        name: 'SambaNova',
        brandColor: '#10B981',
        bgLight: 'rgba(16, 185, 129, 0.12)',
        borderColor: 'rgba(16, 185, 129, 0.3)',
        badgeText: 'SambaNova',
        category: 'api_key',
      };
    case 'fireworks':
      return {
        id: 'fireworks',
        name: 'Fireworks AI',
        brandColor: '#EF4444',
        bgLight: 'rgba(239, 68, 68, 0.12)',
        borderColor: 'rgba(239, 68, 68, 0.3)',
        badgeText: 'Fireworks',
        category: 'api_key',
      };
    case 'siliconflow':
      return {
        id: 'siliconflow',
        name: 'SiliconFlow',
        brandColor: '#3B82F6',
        bgLight: 'rgba(59, 130, 246, 0.12)',
        borderColor: 'rgba(59, 130, 246, 0.3)',
        badgeText: 'SiliconFlow',
        category: 'api_key',
      };
    case 'local':
    case 'ollama':
      return {
        id: 'local',
        name: 'Local Ollama / Metal',
        brandColor: '#14B8A6',
        bgLight: 'rgba(20, 184, 166, 0.12)',
        borderColor: 'rgba(20, 184, 166, 0.3)',
        badgeText: 'Local',
        category: 'local',
      };
    case 'lmstudio':
      return {
        id: 'lmstudio',
        name: 'LM Studio',
        brandColor: '#8B5CF6',
        bgLight: 'rgba(139, 92, 246, 0.12)',
        borderColor: 'rgba(139, 92, 246, 0.3)',
        badgeText: 'LM Studio',
        category: 'local',
      };
    case 'copilot':
      return {
        id: 'copilot',
        name: 'GitHub Copilot',
        brandColor: '#8B5CF6',
        bgLight: 'rgba(139, 92, 246, 0.12)',
        borderColor: 'rgba(139, 92, 246, 0.3)',
        badgeText: 'Copilot',
        category: 'oauth',
      };
    default:
      return {
        id: p || 'custom',
        name: p ? p.charAt(0).toUpperCase() + p.slice(1) : 'Custom Gateway',
        brandColor: '#71717A',
        bgLight: 'rgba(113, 113, 122, 0.12)',
        borderColor: 'rgba(113, 113, 122, 0.3)',
        badgeText: 'Custom',
        category: 'compatible',
      };
  }
}

export function ProviderBrandIcon({ provider, size = 14, className }: IconProps & { provider: string }): React.JSX.Element {
  const p = (provider || '').toLowerCase().trim();
  const Icon = brands[p] || Cpu;
  return <Icon width={size} height={size} className={className} aria-hidden="true" focusable="false" />;
}

export function OpenAIIcon(props: IconProps) { return <ProviderBrandIcon provider="openai" {...props} />; }
export function ClaudeIcon(props: IconProps) { return <ProviderBrandIcon provider="anthropic" {...props} />; }
export function GeminiIcon(props: IconProps) { return <ProviderBrandIcon provider="gemini" {...props} />; }
export function DeepSeekIcon(props: IconProps) { return <ProviderBrandIcon provider="deepseek" {...props} />; }
export function AntigravityIcon(props: IconProps) { return <ProviderBrandIcon provider="antigravity" {...props} />; }

/**
 * Resolves a dedicated brand icon or stylized icon for any of the 46 supported AI coding agents.
 */
export function getAgentBrandIcon(agentId: string, size = 14, className?: string): React.JSX.Element {
  const cn = className || '';
  switch (agentId) {
    case 'claude':
    case 'claude-code':
    case 'claude-agent-teams':
    case 'openclaude':
      return <ClaudeIcon size={size} className={`shrink-0 ${cn}`} />;
    case 'codex':
      return <OpenAIIcon size={size} className={`shrink-0 ${cn}`} />;
    case 'copilot':
      return <ProviderBrandIcon provider="copilot" size={size} className={`shrink-0 ${cn}`} />;
    case 'gemini':
      return <GeminiIcon size={size} className={`shrink-0 ${cn}`} />;
    case 'antigravity':
      return <ProviderBrandIcon provider="antigravity" size={size} className={`shrink-0 ${cn}`} />;
    case 'dsh':
      return <DeepSeekIcon size={size} className={`shrink-0 ${cn}`} />;
    case 'grok':
      return <Sparkles style={{ width: size, height: size }} className={`text-zinc-200 shrink-0 ${cn}`} />;
    case 'goose':
      return <Zap style={{ width: size, height: size }} className={`text-cyan-400 shrink-0 ${cn}`} />;
    case 'opencode':
    case 'opencode2':
      return <Bot style={{ width: size, height: size }} className={`text-blue-400 shrink-0 ${cn}`} />;
    case 'aider':
      return <Terminal style={{ width: size, height: size }} className={`text-emerald-400 shrink-0 ${cn}`} />;
    case 'cursor':
      return <Cpu style={{ width: size, height: size }} className={`text-purple-400 shrink-0 ${cn}`} />;
    case 'cline':
      return <Bot style={{ width: size, height: size }} className={`text-amber-400 shrink-0 ${cn}`} />;
    case 'kimi':
      return <Sparkles style={{ width: size, height: size }} className={`text-sky-400 shrink-0 ${cn}`} />;
    case 'pi':
    case 'omp':
      return <Terminal style={{ width: size, height: size }} className={`text-amber-300 shrink-0 ${cn}`} />;
    case 'kilo':
    case 'kiro':
      return <Zap style={{ width: size, height: size }} className={`text-rose-400 shrink-0 ${cn}`} />;
    case 'crush':
      return <Flame style={{ width: size, height: size }} className={`text-pink-400 shrink-0 ${cn}`} />;
    case 'aug':
      return <Cpu style={{ width: size, height: size }} className={`text-indigo-400 shrink-0 ${cn}`} />;
    case 'droid':
      return <Bot style={{ width: size, height: size }} className={`text-emerald-300 shrink-0 ${cn}`} />;
    case 'mistral-vibe':
      return <Sparkles style={{ width: size, height: size }} className={`text-orange-400 shrink-0 ${cn}`} />;
    case 'qwen-code':
      return <Bot style={{ width: size, height: size }} className={`text-blue-300 shrink-0 ${cn}`} />;
    case 'rovo':
      return <ShieldCheck style={{ width: size, height: size }} className={`text-blue-500 shrink-0 ${cn}`} />;
    case 'hermes':
      return <Sparkles style={{ width: size, height: size }} className={`text-violet-400 shrink-0 ${cn}`} />;
    case 'devin':
      return <Bot style={{ width: size, height: size }} className={`text-teal-400 shrink-0 ${cn}`} />;
    case 'trae':
      return <Terminal style={{ width: size, height: size }} className={`text-cyan-300 shrink-0 ${cn}`} />;
    case 'zcode':
      return <Cpu style={{ width: size, height: size }} className={`text-yellow-400 shrink-0 ${cn}`} />;
    case 'mimo-code':
      return <Bot style={{ width: size, height: size }} className={`text-orange-300 shrink-0 ${cn}`} />;
    case 'ante':
      return <Sparkles style={{ width: size, height: size }} className={`text-fuchsia-400 shrink-0 ${cn}`} />;
    case 'muse':
      return <Compass style={{ width: size, height: size }} className={`text-blue-400 shrink-0 ${cn}`} />;
    case 'qoder':
    case 'qoder-cn':
      return <Code style={{ width: size, height: size }} className={`text-emerald-300 shrink-0 ${cn}`} />;
    case 'prime-agent':
    case 'amp':
      return <Zap style={{ width: size, height: size }} className={`text-amber-400 shrink-0 ${cn}`} />;
    case 'autohand':
      return <Bot style={{ width: size, height: size }} className={`text-indigo-300 shrink-0 ${cn}`} />;
    case 'codebuff':
    case 'freebuff':
      return <Bot style={{ width: size, height: size }} className={`text-lime-400 shrink-0 ${cn}`} />;
    case 'command-code':
    case 'continue':
      return <Terminal style={{ width: size, height: size }} className={`text-sky-300 shrink-0 ${cn}`} />;
    case 'openclaw':
      return <Flame style={{ width: size, height: size }} className={`text-red-400 shrink-0 ${cn}`} />;
    case 'codebuddy':
      return <Bot style={{ width: size, height: size }} className={`text-violet-300 shrink-0 ${cn}`} />;
    case 'jcode':
      return <Code style={{ width: size, height: size }} className={`text-teal-400 shrink-0 ${cn}`} />;
    case 'governed-agent-runtime':
      return <ShieldCheck style={{ width: size, height: size }} className={`text-purple-400 shrink-0 ${cn}`} />;
    default:
      return <Bot style={{ width: size, height: size }} className={`text-fg-muted shrink-0 ${cn}`} />;
  }
}
