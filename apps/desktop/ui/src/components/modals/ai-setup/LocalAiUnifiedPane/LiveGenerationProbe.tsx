import React from 'react';
import { Terminal, Play } from 'lucide-react';
import type { LoadedModelInstance } from '@/types/domain';
import type { LMStudioModelItem } from './types';

export interface LiveGenerationProbeProps {
  testModelId: string;
  setTestModelId: (val: string) => void;
  testPrompt: string;
  setTestPrompt: (val: string) => void;
  instances: LoadedModelInstance[];
  lmsModels: LMStudioModelItem[];
  onRunTest: () => void;
  isGenerating: boolean;
  testError: string | null;
  testResult: string | null;
  testLatencyMs: number | null;
  testTokens: { total?: number; tps?: number } | null;
}

export const LiveGenerationProbe: React.FC<LiveGenerationProbeProps> = ({
  testModelId,
  setTestModelId,
  testPrompt,
  setTestPrompt,
  instances,
  lmsModels,
  onRunTest,
  isGenerating,
  testError,
  testResult,
  testLatencyMs,
  testTokens,
}) => {
  return (
    <div className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-3 shadow-xs">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Terminal className="size-3.5 text-[var(--color-fg-muted)]" />
          <h3 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-editor-fg)] font-mono">
            Live Generation Probe
          </h3>
        </div>
        {testTokens && (
          <span className="font-mono text-[11px] text-[var(--color-fg-muted)]">
            {testLatencyMs}ms · {testTokens.tps} tok/s ({testTokens.total} tokens)
          </span>
        )}
      </div>

      <div className="flex flex-col sm:flex-row gap-2">
        {/* Target model select */}
        <select
          value={testModelId}
          onChange={(e) => setTestModelId(e.target.value)}
          className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 font-mono text-xs text-[var(--color-editor-fg)] focus:border-[var(--color-border-emphasis)] outline-none cursor-pointer max-w-[200px]"
        >
          <option value="">Auto (Active Model)</option>
          {instances.map((inst) => (
            <option key={`opt-inst-${inst.model_path}`} value={inst.model_path}>
              VRAM: {inst.model_name}
            </option>
          ))}
          {lmsModels.map((m) => (
            <option key={`opt-lms-${m.id}`} value={`lmstudio/${m.id}`}>
              LM Studio: {m.id}
            </option>
          ))}
        </select>

        <input
          type="text"
          value={testPrompt}
          onChange={(e) => setTestPrompt(e.target.value)}
          placeholder="Type a test prompt..."
          className="flex-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
        />

        <button
          type="button"
          onClick={onRunTest}
          disabled={isGenerating}
          className="flex items-center gap-1.5 rounded-lg bg-[var(--color-editor-fg)] px-4 py-1.5 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] transition disabled:opacity-40 cursor-pointer shrink-0 shadow-xs"
        >
          <Play className={`size-3 ${isGenerating ? 'animate-spin' : 'fill-current'}`} />
          {isGenerating ? 'Testing…' : 'Run Test'}
        </button>
      </div>

      {testError && (
        <div className="rounded-lg border border-rose-500/25 bg-rose-500/10 p-2.5 text-xs text-rose-300 font-mono">
          {testError}
        </div>
      )}

      {testResult && (
        <div className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3.5 font-mono text-xs text-[var(--color-editor-fg)] whitespace-pre-wrap max-h-48 overflow-y-auto leading-relaxed shadow-inner">
          {testResult}
        </div>
      )}
    </div>
  );
};
