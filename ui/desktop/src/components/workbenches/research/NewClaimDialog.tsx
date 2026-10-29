import React, { useState } from 'react';
import { ShieldCheck, X } from 'lucide-react';

export interface NewClaimDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onCreateClaim: (statement: string) => Promise<void>;
}

export const NewClaimDialog: React.FC<NewClaimDialogProps> = ({
  isOpen,
  onClose,
  onCreateClaim,
}) => {
  const [statement, setStatement] = useState('');
  const [submitting, setSubmitting] = useState(false);

  if (!isOpen) return null;

  const handleSubmit = async () => {
    if (!statement.trim() || submitting) return;
    setSubmitting(true);
    try {
      await onCreateClaim(statement.trim());
      setStatement('');
      onClose();
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
      <div className="w-full max-w-lg rounded-xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] p-5 shadow-2xl space-y-4">
        <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-3">
          <h3 className="text-sm font-semibold text-[var(--color-editor-fg)] flex items-center gap-2">
            <ShieldCheck size={16} className="text-teal-400" />
            Log New Research Claim
          </h3>
          <button
            type="button"
            onClick={onClose}
            className="text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
          >
            <X size={16} />
          </button>
        </div>

        <div className="space-y-3 text-xs">
          <div>
            <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
              Claim Statement *
            </label>
            <textarea
              rows={3}
              value={statement}
              onChange={(e) => setStatement(e.target.value)}
              placeholder="e.g. Model inference throughput scales sublinearly when KV-cache exceeds L2 capacity..."
              className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] outline-none"
            />
          </div>

          <p className="text-[10px] text-fg-muted">Created as an ungrounded draft (L0).</p>
        </div>

        <div className="flex items-center justify-end gap-2 pt-2 border-t border-[var(--color-border-muted)]">
          <button
            type="button"
            onClick={onClose}
            className="px-3 py-1.5 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSubmit}
            disabled={!statement.trim() || submitting}
            className="rounded-lg bg-teal-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-teal-500 disabled:opacity-50 transition"
          >
            {submitting ? 'Saving…' : 'Log Claim'}
          </button>
        </div>
      </div>
    </div>
  );
};
