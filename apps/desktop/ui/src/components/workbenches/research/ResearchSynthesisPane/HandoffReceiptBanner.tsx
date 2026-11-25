import React from 'react';
import { CheckCircle2, Code2, Check, Copy, ExternalLink } from 'lucide-react';
import type { HandoffToCodingResult } from '@/types/domain';

export interface HandoffReceiptBannerProps {
  lastHandoffResult: HandoffToCodingResult | null;
  copiedId: string | null;
  onCopyText: (text: string, idKey: string) => void;
  onNavigateToTask?: (taskId: string, result: HandoffToCodingResult) => void;
}

export const HandoffReceiptBanner: React.FC<HandoffReceiptBannerProps> = ({
  lastHandoffResult,
  copiedId,
  onCopyText,
  onNavigateToTask,
}) => {
  if (!lastHandoffResult) return null;

  return (
    <div className="m-4 p-4 rounded-xl border border-white/15 bg-white/[0.04] text-white/90 space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 font-semibold text-xs text-white">
          <CheckCircle2 size={16} className="text-white/70" />
          Coding Handoff Dispatched Successfully
        </div>
        <span className="font-mono text-[10px] bg-white/10 px-2 py-0.5 rounded text-white/80 border border-white/10">
          Handoff ID: {lastHandoffResult.handoff_id}
        </span>
      </div>
      <p className="text-[11px] text-white/70">
        Created {lastHandoffResult.task_ids.length} task(s) on branch{' '}
        <code className="font-mono px-1 py-0.5 rounded bg-white/10">
          {lastHandoffResult.target_branch || 'default'}
        </code>.
      </p>
      <div className="flex flex-wrap gap-1.5 pt-1">
        {lastHandoffResult.task_ids.map((tid) => (
          <span
            key={tid}
            className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-[var(--color-surface-1)] border border-white/10 text-xs font-mono text-[var(--color-editor-fg)]"
          >
            <Code2 size={12} className="text-white/60" />
            <span>{tid}</span>
            <button
              onClick={() => onCopyText(tid, tid)}
              className="text-[var(--color-fg-muted)] hover:text-white"
              title="Copy Task ID"
            >
              {copiedId === tid ? <Check size={11} className="text-white" /> : <Copy size={11} />}
            </button>
            {onNavigateToTask && (
              <button
                onClick={() => onNavigateToTask(tid, lastHandoffResult)}
                className="text-white/70 hover:text-white ml-1"
                title="View Task in Engineering Workbench"
              >
                <ExternalLink size={11} />
              </button>
            )}
          </span>
        ))}
      </div>
    </div>
  );
};
