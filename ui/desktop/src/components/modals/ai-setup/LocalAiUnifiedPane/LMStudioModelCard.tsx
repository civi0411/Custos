import React from 'react';
import { Check } from 'lucide-react';
import type { LMStudioModelItem } from './types';

export interface LMStudioModelCardProps {
  model: LMStudioModelItem;
  isCurrentActive: boolean;
  onSetActive: (id: string, name: string) => void;
}

export const LMStudioModelCard: React.FC<LMStudioModelCardProps> = ({
  model: m,
  isCurrentActive,
  onSetActive,
}) => {
  return (
    <div
      className={`rounded-xl border transition-all p-3.5 text-xs ${
        isCurrentActive
          ? 'border-sky-500/40 bg-[var(--color-surface-0)] shadow-xs ring-1 ring-sky-500/20'
          : 'border-[var(--color-border-default)] bg-[var(--color-surface-0)] hover:border-[var(--color-border-emphasis)]'
      }`}
    >
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="size-2 rounded-full bg-emerald-400 shrink-0 ring-2 ring-emerald-400/20" />
            <span className="font-mono text-[var(--color-editor-fg)] font-semibold truncate">{m.id}</span>
            <span className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[9px] text-[var(--color-fg-muted)]">
              LM Studio
            </span>
            {isCurrentActive && (
              <span className="inline-flex items-center gap-1 rounded-md border border-sky-500/30 bg-sky-500/10 px-2 py-0.5 font-mono text-[9px] text-sky-300 font-medium">
                <Check className="size-2.5" />
                Active Model
              </span>
            )}
          </div>
          <div className="font-mono text-[10px] text-[var(--color-fg-subtle)] mt-1 flex flex-wrap items-center gap-2">
            <span>Arch: {m.architecture}</span>
            <span>•</span>
            <span>{m.quantization || 'Q4_K_M'}</span>
            <span>•</span>
            <span>{m.contextLength ? `${m.contextLength.toLocaleString()} ctx` : 'Dynamic Context'}</span>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {isCurrentActive ? (
            <span className="flex items-center gap-1 rounded-lg border border-sky-500/30 bg-sky-500/10 px-3 py-1 font-mono text-[11px] text-sky-300 font-medium">
              <Check className="size-3" /> Active
            </span>
          ) : (
            <button
              type="button"
              onClick={() => onSetActive(`lmstudio/${m.id}`, m.id)}
              className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1 font-mono text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:border-[var(--color-border-emphasis)] hover:bg-[var(--color-surface-3)] transition cursor-pointer"
            >
              Set Active
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
