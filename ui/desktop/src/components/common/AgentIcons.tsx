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

type IconProps = { size?: number; className?: string };

const brands: Record<string, React.ComponentType<React.SVGProps<SVGSVGElement>>> = {
  openai: OpenAI,
  anthropic: Claude,
  gemini: Gemini,
  deepseek: DeepSeek,
  copilot: Copilot,
  antigravity: Antigravity,
  ollama: Ollama,
  local: Ollama,
  server: Server,
};

export function ProviderBrandIcon({ provider, size = 14, className }: IconProps & { provider: string }): React.JSX.Element {
  const Icon = brands[provider] || Cpu;
  return <Icon width={size} height={size} className={className} aria-hidden="true" focusable="false" />;
}

export function OpenAIIcon(props: IconProps) { return <ProviderBrandIcon provider="openai" {...props} />; }
export function ClaudeIcon(props: IconProps) { return <ProviderBrandIcon provider="anthropic" {...props} />; }
export function GeminiIcon(props: IconProps) { return <ProviderBrandIcon provider="gemini" {...props} />; }
export function DeepSeekIcon(props: IconProps) { return <ProviderBrandIcon provider="deepseek" {...props} />; }

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
