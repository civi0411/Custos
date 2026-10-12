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
