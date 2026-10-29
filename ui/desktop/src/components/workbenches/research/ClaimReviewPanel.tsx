import React from 'react';
import { X } from 'lucide-react';
import type { ReviewerRecord } from '@/types/domain';
import type { ResearchClaim } from '@/types/research';

export interface ClaimReviewPanelProps {
  claim: ResearchClaim | null;
  validating: boolean;
  reviewError: string | null;
  reviews: ReviewerRecord[];
  onClose: () => void;
}

export const ClaimReviewPanel: React.FC<ClaimReviewPanelProps> = ({
  claim,
  validating,
  reviewError,
  reviews,
  onClose,
}) => {
  if (!claim) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-6"
      role="dialog"
      aria-modal="true"
      aria-label="Claim review records"
    >
      <section className="w-full max-w-xl max-h-[80vh] overflow-auto rounded-xl border border-border-default bg-canvas p-4 space-y-3">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-sm font-semibold">Claim review records</h2>
          <button
            type="button"
            autoFocus
            aria-label="Close reviews"
            className="canvas-icon"
            onClick={onClose}
          >
            <X size={16} />
          </button>
        </div>
        <p className="text-xs text-fg-muted">{claim.statement}</p>
        {validating ? (
          <p role="status" className="text-xs">
            Loading records…
          </p>
        ) : reviewError ? (
          <p role="alert" className="text-xs text-rose-400">
            {reviewError}
          </p>
        ) : reviews.length === 0 ? (
          <p className="text-xs text-fg-muted">No review records attached.</p>
        ) : (
          reviews.map((review) => (
            <article
              key={review.id}
              className="rounded border border-border-default p-3 text-xs space-y-2"
            >
              <div className="flex justify-between gap-2">
                <strong>{review.reviewer}</strong>
                <span>
                  {review.status} · {review.is_fresh ? 'Current' : 'Stale'}
                </span>
              </div>
              <p>{review.evidence_summary}</p>
              <p className="text-fg-muted">
                {review.method} · {review.id}
              </p>
              {review.evidence_digest && (
                <code className="block break-all text-[11px]">
                  {review.evidence_digest}
                </code>
              )}
            </article>
          ))
        )}
      </section>
    </div>
  );
};
