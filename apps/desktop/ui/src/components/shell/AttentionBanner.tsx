import React from 'react';
import { CheckCircle2, XCircle, FileDiff, X } from 'lucide-react';

export interface AttentionItem {
  id: string;
  type: 'permit_request' | 'uncertain_effect' | 'gate_failed';
  title: string;
  description: string;
  riskLevel: 'Low' | 'Medium' | 'High';
  targetResource: string;
  worktreeId?: string;
  onApprove?: () => void;
  onReject?: () => void;
  onInspect?: () => void;
}

export interface AttentionBannerProps {
  items: AttentionItem[];
  onDismiss?: (id: string) => void;
}

export const AttentionBanner: React.FC<AttentionBannerProps> = ({ items, onDismiss }) => {
  if (!items || items.length === 0) return null;

  const activeItem = items[0];

  const riskBadgeColor =
    activeItem.riskLevel === 'High'
      ? 'bg-rose-500/15 text-rose-300 border-rose-500/30'
      : 'bg-white/6 text-white/70 border-white/12';

  return (
    <div className="bg-[#18181b] border-b border-white/10 px-4 py-2 flex items-center justify-between text-xs font-sans text-white z-40 select-none shadow-xs animate-in slide-in-from-top-1 duration-150">
      <div className="flex items-center gap-3 min-w-0">
        <div className="flex items-center gap-1.5 shrink-0">
          <span className="size-2 rounded-full bg-white/80 animate-pulse shrink-0" />
          <span className="font-mono font-semibold text-white/90 uppercase tracking-wider text-[10px] bg-white/6 border border-white/12 px-1.5 py-0.5 rounded">
            Checkpoint ({items.length})
          </span>
        </div>

        <div className="flex items-center gap-2 truncate">
          <span className="font-semibold text-white/90">{activeItem.title}:</span>
          <span className="text-white/60 truncate">{activeItem.description}</span>
          <span className={`px-1.5 py-0.2 rounded font-mono text-[10px] border ${riskBadgeColor}`}>
            Risk: {activeItem.riskLevel}
          </span>
          {activeItem.worktreeId && (
            <span className="font-mono text-[10px] bg-white/4 text-white/60 px-1.5 py-0.2 rounded border border-white/8">
              {activeItem.worktreeId}
            </span>
          )}
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0 ml-4">
        {activeItem.onInspect && (
          <button
            type="button"
            onClick={activeItem.onInspect}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-white/6 hover:bg-white/10 text-white/80 border border-white/10 transition text-[11px] font-medium cursor-pointer"
          >
            <FileDiff className="size-3.5 text-white/50" />
            <span>Inspect Diff</span>
          </button>
        )}
        {activeItem.onReject && (
          <button
            type="button"
            onClick={activeItem.onReject}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-white/4 hover:bg-white/8 text-white/60 hover:text-white border border-white/10 transition text-[11px] cursor-pointer"
          >
            <XCircle className="size-3.5" />
            <span>Reject</span>
          </button>
        )}
        {activeItem.onApprove && (
          <button
            type="button"
            onClick={activeItem.onApprove}
            className="flex items-center gap-1.5 px-3 py-1 rounded bg-white hover:bg-white/90 text-black font-semibold shadow-xs transition text-[11px] cursor-pointer"
          >
            <CheckCircle2 className="size-3.5 text-black" />
            <span>Approve Permit</span>
          </button>
        )}
        {onDismiss && (
          <button
            type="button"
            onClick={() => onDismiss(activeItem.id)}
            className="p-1 rounded text-white/40 hover:text-white hover:bg-white/8 transition ml-1 cursor-pointer"
            title="Dismiss notification"
            aria-label="Dismiss notification"
          >
            <X className="size-3.5" />
          </button>
        )}
      </div>
    </div>
  );
};
