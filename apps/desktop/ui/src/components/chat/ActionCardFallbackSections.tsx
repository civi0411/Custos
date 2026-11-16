import React from 'react';
import {
  AlertTriangle,
  ArrowRight,
  Bot,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Copy,
  Globe,
  Terminal,
} from 'lucide-react';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export const ActionTerminalCard: React.FC<{
  cmd: string;
  output: string;
  exitCode: unknown;
  isExpanded: boolean;
  onToggleExpand: () => void;
  copied: boolean;
  onCopy: (text: string) => void;
}> = ({ cmd, output, exitCode, isExpanded, onToggleExpand, copied, onCopy }) => {
  const isSuccess = exitCode === 0;
  return (
    <div className="my-2 rounded-lg border border-border-default bg-canvas overflow-hidden text-xs shadow-xs font-mono">
      <div className="flex items-center justify-between px-3 py-1.5 bg-surface-1 border-b border-border-muted">
        <div className="flex items-center gap-2 min-w-0">
          <Terminal className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
          <span className="text-[11px] text-fg-editor truncate font-semibold" title={cmd}>
            $ {cmd || 'terminal command'}
          </span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <span
            className={`inline-flex items-center gap-1 text-[10px] px-1.5 py-0.2 rounded font-sans font-medium ${
              isSuccess ? 'bg-emerald-500/10 text-emerald-400' : 'bg-rose-500/10 text-rose-400'
            }`}
          >
            {isSuccess ? <CheckCircle2 className="w-2.5 h-2.5" /> : <AlertTriangle className="w-2.5 h-2.5" />}
            <span>{isSuccess ? 'Exit 0' : `Exit ${String(exitCode)}`}</span>
          </span>
          <button
            type="button"
            onClick={onToggleExpand}
            className="p-1 rounded text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
          >
            {isExpanded ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
          </button>
        </div>
      </div>

      {isExpanded && output && (
        <div className="relative p-2.5 text-[11px] text-fg-muted overflow-x-auto max-h-52 whitespace-pre-wrap leading-relaxed border-t border-border-muted/50 bg-surface-0/60">
          <button
            type="button"
            onClick={() => onCopy(output)}
            className="absolute top-2 right-2 p-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-muted hover:text-fg-editor border border-border-default transition"
            title="Copy output"
          >
            {copied ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
          </button>
          {output}
        </div>
      )}
    </div>
  );
};

export const ActionLiteratureCard: React.FC<{
  query: string;
  resultsCount: unknown;
  onOpenTab?: (id: OrcaTabId) => void;
}> = ({ query, resultsCount, onOpenTab }) => (
  <div className="my-2 rounded-lg border border-border-default bg-surface-1 p-2.5 flex items-center justify-between text-xs shadow-xs">
    <div className="flex items-center gap-2 min-w-0">
      <Globe className="w-4 h-4 text-sky-400 shrink-0" />
      <div className="min-w-0">
        <div className="text-[11px] font-medium text-fg-editor truncate">
          Literature Query: <span className="font-mono text-sky-300">"{query}"</span>
        </div>
        {resultsCount !== undefined && (
          <div className="text-[10px] text-fg-muted">Indexed {String(resultsCount)} evidence citations</div>
        )}
      </div>
    </div>
    {onOpenTab && (
      <button
        type="button"
        onClick={() => onOpenTab('literature')}
        className="flex items-center gap-1 px-2 py-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-editor text-[11px] border border-border-default transition shrink-0 ml-2 cursor-pointer"
      >
        <span>Inspect Sources</span>
        <ArrowRight className="w-3 h-3 text-purple-400" />
      </button>
    )}
  </div>
);

export const ActionGenericToolCard: React.FC<{
  label: string;
  badgeLabel: string;
  payload: Record<string, any>;
  isExpanded: boolean;
  onToggleExpand: () => void;
}> = ({ label, badgeLabel, payload, isExpanded, onToggleExpand }) => (
  <div className="my-1.5 rounded-lg border border-border-muted bg-surface-1/50 overflow-hidden text-xs">
    <button
      type="button"
      onClick={onToggleExpand}
      className="w-full flex items-center justify-between px-3 py-1.5 text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition cursor-pointer"
    >
      <div className="flex items-center gap-2 min-w-0">
        <Bot className="w-3.5 h-3.5 text-fg-subtle shrink-0" />
        <span className="font-mono text-[11px] text-fg-editor truncate">{label}</span>
        <span className="text-[10px] font-sans px-1.5 py-0.2 rounded bg-surface-2 text-fg-muted uppercase">
          {badgeLabel}
        </span>
      </div>
      {isExpanded ? (
        <ChevronDown className="w-3.5 h-3.5 text-fg-muted" />
      ) : (
        <ChevronRight className="w-3.5 h-3.5 text-fg-muted" />
      )}
    </button>
    {isExpanded && (
      <div className="p-2 border-t border-border-muted bg-surface-0 font-mono text-[11px] max-h-48 overflow-auto text-fg-muted leading-relaxed">
        <pre className="whitespace-pre-wrap break-words">{JSON.stringify(payload, null, 2)}</pre>
      </div>
    )}
  </div>
);

