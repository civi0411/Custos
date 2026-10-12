import React from 'react';
import { Server, Cpu } from 'lucide-react';

type IconProps = { size?: number; className?: string };

const OpenAI: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="currentColor" {...props}>
    <path d="M22.282 9.821a5.985 5.985 0 0 0-.516-4.91 6.046 6.046 0 0 0-6.51-2.9A6.065 6.065 0 0 0 4.981 4.18a5.985 5.985 0 0 0-3.998 2.9 6.046 6.046 0 0 0 .743 7.097 5.98 5.98 0 0 0 .51 4.911 6.051 6.051 0 0 0 6.515 2.9A5.985 5.985 0 0 0 13.26 24a6.056 6.056 0 0 0 5.772-4.206 5.99 5.99 0 0 0 3.997-2.9 6.056 6.056 0 0 0-.747-7.073zM13.26 22.43a4.476 4.476 0 0 1-2.876-1.04l.141-.081 4.779-2.758a.795.795 0 0 0 .392-.681v-6.737l2.02 1.168a.071.071 0 0 1 .038.052v5.583a4.504 4.504 0 0 1-4.494 4.494zM3.6 18.304a4.47 4.47 0 0 1-.535-3.014l.142.085 4.783 2.759a.771.771 0 0 0 .78 0l5.843-3.369v2.332a.08.08 0 0 1-.033.062L9.74 19.95a4.5 4.5 0 0 1-6.14-1.646zm-1.22-9.61a4.472 4.472 0 0 1 2.34-1.974V12.2a.767.767 0 0 0 .388.677l5.84 3.37-2.02 1.167a.088.088 0 0 1-.073.007l-4.84-2.796A4.499 4.499 0 0 1 2.38 8.694zm15.864 3.486L12.4 8.81v-.004l-5.844-3.37 2.02-1.166a.08.08 0 0 1 .073-.008l4.84 2.796a4.497 4.497 0 0 1 4.755 7.122zm2.156 3.12a4.474 4.474 0 0 1-2.341 1.974V11.8a.767.767 0 0 0-.388-.677l-5.84-3.37 2.02-1.167a.088.088 0 0 1 .073-.007l4.84 2.796a4.499 4.499 0 0 1 1.636 6.025zM12 13.383l-2.613-1.508 2.613-1.508 2.613 1.508L12 13.383z"/>
  </svg>
);

const Claude: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="currentColor" {...props}>
    <path d="M17.415 4.125a.8.8 0 0 0-.743.498l-3.328 8.214-1.92-5.76A.8.8 0 0 0 10.665 6.5h-1.33a.8.8 0 0 0-.76.547L5.14 18.25a.8.8 0 0 0 .76 1.05h1.75a.8.8 0 0 0 .76-.547l1.373-4.12 2.775 5.55a.8.8 0 0 0 .716.442h1.61a.8.8 0 0 0 .744-.505l5.25-14.875a.8.8 0 0 0-.743-1.07h-2.97z"/>
  </svg>
);

const Gemini: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="currentColor" {...props}>
    <path d="M12 0C12 6.627 6.627 12 0 12c6.627 0 12 5.373 12 12 0-6.627 5.373-12 12-12-6.627 0-12-5.373-12-12z"/>
  </svg>
);

const DeepSeek: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="currentColor" {...props}>
    <path d="M12 2C6.477 2 2 6.477 2 12c0 4.418 2.865 8.166 6.839 9.489.5.092.682-.217.682-.482 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0 0 22 12c0-5.523-4.477-10-10-10z"/>
  </svg>
);

const Copilot: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="currentColor" {...props}>
    <path d="M12 2a9 9 0 0 0-9 9c0 3.8 2.4 7.1 5.8 8.4.4.1.6-.2.6-.4v-1.5c-2.3.5-2.8-1.1-2.8-1.1-.4-1-.9-1.3-.9-1.3-.7-.5.1-.5.1-.5.8.1 1.2.8 1.2.8.7 1.3 1.9.9 2.4.7.1-.5.3-.9.5-1.1-1.8-.2-3.8-.9-3.8-4.1 0-.9.3-1.6.8-2.2-.1-.2-.4-1.1.1-2.2 0 0 .7-.2 2.3.9a8 8 0 0 1 4.2 0c1.6-1.1 2.3-.9 2.3-.9.5 1.1.2 2 .1 2.2.6.6.8 1.3.8 2.2 0 3.2-2 3.9-3.8 4.1.3.3.6.8.6 1.6v2.4c0 .2.2.5.6.4A9 9 0 0 0 21 11a9 9 0 0 0-9-9z"/>
  </svg>
);

const Antigravity: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="currentColor" {...props}>
    <path d="M12 2L2 19.5h20L12 2zm0 3.8l6.5 11.4H5.5L12 5.8z"/>
  </svg>
);

const Ollama: React.FC<React.SVGProps<SVGSVGElement>> = (props) => (
  <svg viewBox="0 0 24 24" fill="currentColor" {...props}>
    <path d="M10 3a2 2 0 0 0-2 2v2H6a2 2 0 0 0-2 2v9a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3V9a2 2 0 0 0-2-2h-2V5a2 2 0 0 0-2-2h-4zm0 2h4v2h-4V5zm-4 4h12v9a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V9zm2 3a1 1 0 1 0 0 2 1 1 0 0 0 0-2zm8 0a1 1 0 1 0 0 2 1 1 0 0 0 0-2zm-5 4a3 3 0 0 0 2 0v1h-2v-1z"/>
  </svg>
);

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
