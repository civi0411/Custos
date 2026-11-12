import React from 'react';
import {
  ShieldCheck,
  Cpu,
  FileText,
  Hash,
  Check,
  Copy,
  AlertCircle,
  FileCode,
} from 'lucide-react';
import type { ReviewerRecord } from '@/types/domain';
import { renderStatusBadge, renderSeverityBadge, renderTargetIcon } from './badges';

export interface ReviewInspectorDetailProps {
  selectedReview: ReviewerRecord | null;
  markingStale: boolean;
  copiedDigest: boolean;
  onMarkStale: (id: string) => void;
  onCopyDigest: (digest: string) => void;
}

export const ReviewInspectorDetail: React.FC<ReviewInspectorDetailProps> = ({
  selectedReview,
  markingStale,
  copiedDigest,
  onMarkStale,
  onCopyDigest,
}) => {
  if (!selectedReview) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-6 text-center text-fg-subtle space-y-1.5">
        <ShieldCheck className="w-8 h-8 text-fg-subtle opacity-40" />
        <h3 className="text-xs font-medium text-fg-default">No record selected</h3>
      </div>
    );
  }

  const isFresh = selectedReview.is_fresh ?? selectedReview.isFresh;
  const targetType = selectedReview.target_type || selectedReview.targetType;
  const targetId = selectedReview.target_id || selectedReview.targetId;
  const summary = selectedReview.evidence_summary || selectedReview.evidenceSummary;
  const digest = selectedReview.evidence_digest || selectedReview.evidenceDigest;

  return (
    <div className="flex-1 flex flex-col min-h-0 bg-bg-surface/10 overflow-y-auto">
      <div className="p-6 space-y-6 max-w-4xl">
        {/* Review Header Banner */}
        <div className="p-5 rounded-xl border border-border-subtle bg-bg-surface/60 backdrop-blur-sm space-y-4">
          <div className="flex items-start justify-between">
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                {renderTargetIcon(targetType)}
                <span className="text-xs font-mono uppercase text-fg-subtle">
                  {targetType}
                </span>
                <span className="text-fg-subtle">/</span>
                <span className="text-sm font-semibold text-fg-default font-mono">
                  {targetId}
                </span>
              </div>
              <div className="text-xs text-fg-subtle font-mono">
                Record ID: {selectedReview.id}
              </div>
            </div>

            <div className="flex items-center gap-2">
              {renderStatusBadge(selectedReview.status)}
              <span
                className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-mono border ${
                  isFresh
                    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                    : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                }`}
              >
                <span
                  className={`w-1.5 h-1.5 rounded-full ${
                    isFresh ? 'bg-emerald-400' : 'bg-amber-400'
                  }`}
                />
                {isFresh ? 'Fresh' : 'Stale'}
              </span>
            </div>
          </div>

          {/* Toolbar */}
          <div className="flex items-center justify-between pt-3 border-t border-border-subtle/50 text-xs">
            <div className="flex items-center gap-4 text-fg-subtle font-mono text-[11px]">
              <span className="flex items-center gap-1.5">
                <Cpu className="w-3.5 h-3.5 text-fg-muted" />
                Reviewer: <strong className="text-fg-default">{selectedReview.reviewer}</strong>
              </span>
              <span className="flex items-center gap-1.5">
                <ShieldCheck className="w-3.5 h-3.5 text-fg-muted" />
                Method: <strong className="text-fg-default">{selectedReview.method}</strong>
              </span>
            </div>

            <div className="flex items-center gap-2">
              {isFresh && (
                <button
                  onClick={() => onMarkStale(selectedReview.id)}
                  disabled={markingStale}
                  className="px-2.5 py-1 text-xs rounded border border-amber-500/30 text-amber-400 hover:bg-amber-500/10 transition-colors disabled:opacity-50"
                >
                  {markingStale ? 'Marking...' : 'Mark Stale'}
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Summary & Digest Card */}
        <div className="p-5 rounded-xl border border-border-subtle bg-bg-surface/40 space-y-4">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle flex items-center gap-2">
            <FileText className="w-3.5 h-3.5 text-fg-muted" />
            Evidence Summary
          </h2>
          <div className="text-xs text-fg-default leading-relaxed whitespace-pre-wrap p-3 rounded-lg bg-bg-canvas/50 border border-border-subtle font-sans">
            {summary}
          </div>

          {digest && (
            <div className="space-y-1.5">
              <span className="text-[11px] font-mono text-fg-subtle flex items-center gap-1.5">
                <Hash className="w-3 h-3" /> Cryptographic Evidence Digest (CAS)
              </span>
              <div className="flex items-center justify-between p-2 rounded-lg bg-bg-canvas border border-border-subtle font-mono text-xs">
                <span className="text-fg-muted truncate">{digest}</span>
                <button
                  onClick={() => onCopyDigest(digest)}
                  className="p-1 text-fg-subtle hover:text-fg-default transition-colors ml-2"
                  title="Copy digest"
                >
                  {copiedDigest ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Findings Section */}
        <div className="p-5 rounded-xl border border-border-subtle bg-bg-surface/40 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle flex items-center gap-2">
              <AlertCircle className="w-3.5 h-3.5 text-fg-muted" />
              Review Findings ({selectedReview.findings?.length || 0})
            </h2>
          </div>

          {!selectedReview.findings || selectedReview.findings.length === 0 ? (
            <div className="p-6 text-center text-xs text-fg-subtle rounded-lg border border-dashed border-border-subtle">
              No findings recorded. Verification completed without issues.
            </div>
          ) : (
            <div className="space-y-2">
              {selectedReview.findings.map((f, idx) => (
                <div
                  key={idx}
                  className="p-3 rounded-lg border border-border-subtle bg-bg-canvas/60 space-y-2"
                >
                  <div className="flex items-center justify-between text-xs">
                    <div className="flex items-center gap-2">
                      {renderSeverityBadge(f.severity)}
                      <span className="font-mono font-medium text-fg-default">
                        {f.criterion}
                      </span>
                    </div>
                    {(f.file_path || f.filePath) && (
                      <span className="font-mono text-[11px] text-fg-subtle flex items-center gap-1">
                        <FileCode className="w-3 h-3" />
                        {f.file_path || f.filePath}
                        {(f.line_number ?? f.lineNumber) !== undefined &&
                          `:${f.line_number ?? f.lineNumber}`}
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-fg-muted pl-1 leading-relaxed">
                    {f.message}
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Gate 4 Integrity Audit Card */}
        <div className="p-4 rounded-xl border border-emerald-500/20 bg-emerald-500/5 space-y-2">
          <div className="flex items-center gap-2 text-xs font-semibold text-emerald-400">
            <ShieldCheck className="w-4 h-4" />
            Gate 4 Verifier Contract Enforcement
          </div>
          <p className="text-[10px] text-fg-subtle">
            Active, fresh verification records are required before promotion or handoff.
          </p>
        </div>
      </div>
    </div>
  );
};
