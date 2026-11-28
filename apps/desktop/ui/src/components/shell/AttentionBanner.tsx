import React from 'react';
import { CheckCircle2, FileDiff, X } from 'lucide-react';
import { GhostDismissButton, SecuritySurface } from '@/components/common/SecuritySurface';

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
    <div className="px-3 py-1.5 z-40 select-none animate-in slide-in-from-top-1 duration-150">
      <SecuritySurface
        title={activeItem.title}
        subtitle={activeItem.description}
        badge={`Checkpoint (${items.length})`}
      >
        <div className="flex items-center justify-between text-xs font-sans gap-3">
          <div className="flex items-center gap-2 min-w-0 truncate">
            <span className={`px-1.5 py-0.2 rounded font-mono text-[10px] border ${riskBadgeColor}`}>
              Risk: {activeItem.riskLevel}
            </span>
            {activeItem.worktreeId && (
              <span className="font-mono text-[10px] bg-white/4 text-white/60 px-1.5 py-0.2 rounded border border-white/8">
                {activeItem.worktreeId}
              </span>
            )}
          </div>

          <div className="flex items-center gap-2 shrink-0">
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
              <GhostDismissButton label="Reject" onClick={activeItem.onReject} />
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
              <GhostDismissButton
                onClick={() => onDismiss(activeItem.id)}
                title="Dismiss notification"
                aria-label="Dismiss notification"
                className="p-1!"
              >
                <X className="size-3.5" />
              </GhostDismissButton>
            )}
          </div>
        </div>
      </SecuritySurface>
    </div>
  );
};
