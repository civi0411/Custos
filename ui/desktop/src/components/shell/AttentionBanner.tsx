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
      ? 'bg-rose-500/10 text-rose-400 border-rose-500/20'
      : activeItem.riskLevel === 'Medium'
      ? 'bg-amber-500/10 text-amber-400 border-amber-500/20'
      : 'bg-blue-500/10 text-blue-400 border-blue-500/20';

  return (
    <div className="bg-surface-1 border-b border-border-default px-4 py-2 flex items-center justify-between text-xs font-sans text-fg-editor z-40 select-none shadow-xs animate-in slide-in-from-top-1 duration-150">
      <div className="flex items-center gap-3 min-w-0">
        <div className="flex items-center gap-1.5 shrink-0">
          <span className="flex h-2 w-2 relative">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-amber-400 opacity-75"></span>
            <span className="relative inline-flex rounded-full h-2 w-2 bg-amber-500"></span>
          </span>
          <span className="font-semibold text-amber-500 uppercase tracking-wider text-[10px] bg-amber-500/15 border border-amber-500/30 px-1.5 py-0.5 rounded">
            Needs You ({items.length})
          </span>
        </div>

        <div className="flex items-center gap-2 truncate">
          <span className="font-semibold text-fg-editor">{activeItem.title}:</span>
          <span className="text-fg-muted truncate">{activeItem.description}</span>
          <span className={`px-1.5 py-0.2 rounded font-mono text-[10px] border ${riskBadgeColor}`}>
            Risk: {activeItem.riskLevel}
          </span>
          {activeItem.worktreeId && (
            <span className="font-mono text-[10px] bg-surface-2 text-fg-editor px-1.5 py-0.2 rounded border border-border-default">
              {activeItem.worktreeId}
            </span>
          )}
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0 ml-4">
        {activeItem.onInspect && (
          <button
            onClick={activeItem.onInspect}
            className="flex items-center gap-1 px-2.5 py-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-editor border border-border-default transition text-[11px]"
          >
            <FileDiff className="w-3.5 h-3.5 text-fg-muted" />
            <span>Inspect Diff</span>
          </button>
        )}
        {activeItem.onReject && (
          <button
            onClick={activeItem.onReject}
            className="flex items-center gap-1 px-2.5 py-1 rounded bg-rose-500/10 hover:bg-rose-500/20 text-rose-500 border border-rose-500/30 transition text-[11px]"
          >
            <XCircle className="w-3.5 h-3.5" />
            <span>Reject</span>
          </button>
        )}
        {activeItem.onApprove && (
          <button
            onClick={activeItem.onApprove}
            className="flex items-center gap-1 px-3 py-1 rounded bg-amber-500 hover:bg-amber-400 text-black font-semibold shadow-xs transition text-[11px]"
          >
            <CheckCircle2 className="w-3.5 h-3.5" />
            <span>Approve Permit</span>
          </button>
        )}
        {onDismiss && (
          <button
            onClick={() => onDismiss(activeItem.id)}
            className="p-1 rounded text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition ml-1"
            title="Dismiss notification"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        )}
      </div>
    </div>
  );
};
