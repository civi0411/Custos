import React, { useState } from 'react';
import { ShieldCheck, Trash2 } from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import type {
  RecordReviewParams,
  ReviewTargetType,
  ReviewMethod,
  ReviewStatus,
  FindingSeverity,
  ReviewFinding,
} from '@/types/domain';
import { TARGET_TYPES, REVIEW_METHODS, REVIEW_STATUSES, SEVERITIES } from './constants';
import { renderSeverityBadge } from './badges';

export interface RecordReviewModalProps {
  isOpen: boolean;
  submitting: boolean;
  onClose: () => void;
  onSubmit: (params: RecordReviewParams) => Promise<void>;
  onShowToast?: (msg: string) => void;
}

export const RecordReviewModal: React.FC<RecordReviewModalProps> = ({
  isOpen,
  submitting,
  onClose,
  onSubmit,
  onShowToast,
}) => {
  const [formTargetType, setFormTargetType] = useState<ReviewTargetType>('artifact');
  const [formTargetId, setFormTargetId] = useState<string>('');
  const [formReviewer, setFormReviewer] = useState<string>('custos-verifier');
  const [formMethod, setFormMethod] = useState<ReviewMethod>('automated_verifier');
  const [formStatus, setFormStatus] = useState<ReviewStatus>('approved');
  const [formSummary, setFormSummary] = useState<string>('');
  const [formDigest, setFormDigest] = useState<string>('');
  const [formFindings, setFormFindings] = useState<ReviewFinding[]>([]);

  // Findings draft
  const [findingSeverity, setFindingSeverity] = useState<FindingSeverity>('info');
  const [findingCriterion, setFindingCriterion] = useState<string>('');
  const [findingMessage, setFindingMessage] = useState<string>('');
  const [findingFilePath, setFindingFilePath] = useState<string>('');
  const [findingLineNumber, setFindingLineNumber] = useState<string>('');

  if (!isOpen) return null;

  const handleAddFinding = () => {
    if (!findingCriterion.trim() || !findingMessage.trim()) {
      onShowToast?.('Criterion and finding message are required');
      return;
    }
    const lineNum = findingLineNumber.trim() ? parseInt(findingLineNumber.trim(), 10) : undefined;
    const newFinding: ReviewFinding = {
      severity: findingSeverity,
      criterion: findingCriterion.trim(),
      message: findingMessage.trim(),
      file_path: findingFilePath.trim() || undefined,
      line_number: Number.isNaN(lineNum) ? undefined : lineNum,
    };
    setFormFindings([...formFindings, newFinding]);
    setFindingCriterion('');
    setFindingMessage('');
    setFindingFilePath('');
    setFindingLineNumber('');
  };

  const handleRemoveFinding = (index: number) => {
    setFormFindings(formFindings.filter((_, i) => i !== index));
  };

  const handleFormSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formTargetId.trim()) {
      onShowToast?.('Target identifier is required');
      return;
    }
    if (!formReviewer.trim()) {
      onShowToast?.('Reviewer identifier is required');
      return;
    }
    if (!formSummary.trim()) {
      onShowToast?.('Evidence summary is required');
      return;
    }

    const params: RecordReviewParams = {
      target_type: formTargetType,
      target_id: formTargetId.trim(),
      reviewer: formReviewer.trim(),
      method: formMethod,
      status: formStatus,
      evidence_summary: formSummary.trim(),
      evidence_digest: formDigest.trim() || undefined,
      findings: formFindings.length > 0 ? formFindings : undefined,
    };

    await onSubmit(params);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
      <div className="w-full max-w-2xl bg-bg-surface border border-border-default rounded-xl shadow-2xl flex flex-col max-h-[90vh] overflow-hidden">
        {/* Modal Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border-subtle bg-bg-surface/80">
          <div className="flex items-center gap-2.5">
            <ShieldCheck className="w-5 h-5 text-emerald-400" />
            <h2 className="text-sm font-semibold text-fg-default">
              Record Review & Criteria Verification
            </h2>
          </div>
          <button
            onClick={onClose}
            className="text-fg-subtle hover:text-fg-default text-lg leading-none p-1"
          >
            &times;
          </button>
        </div>

        {/* Modal Body */}
        <form onSubmit={handleFormSubmit} className="flex-1 overflow-y-auto p-6 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-medium text-fg-subtle mb-1">
                Target Type
              </label>
              <NativeSelect
                value={formTargetType}
                onChange={(e) => setFormTargetType(e.target.value as ReviewTargetType)}
                className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default focus:outline-none focus:border-border-default"
              >
                {TARGET_TYPES.map((t) => (
                  <option key={t.value} value={t.value}>
                    {t.label}
                  </option>
                ))}
              </NativeSelect>
            </div>

            <div>
              <label className="block text-xs font-medium text-fg-subtle mb-1">
                Target Identifier
              </label>
              <input
                type="text"
                placeholder="e.g. artifacts/report.md or git-sha"
                value={formTargetId}
                onChange={(e) => setFormTargetId(e.target.value)}
                required
                className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default font-mono"
              />
            </div>
          </div>

          <div className="grid grid-cols-3 gap-4">
            <div>
              <label className="block text-xs font-medium text-fg-subtle mb-1">
                Reviewer ID
              </label>
              <input
                type="text"
                placeholder="e.g. custos-verifier, cargo-clippy"
                value={formReviewer}
                onChange={(e) => setFormReviewer(e.target.value)}
                required
                className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default font-mono"
              />
            </div>

            <div>
              <label className="block text-xs font-medium text-fg-subtle mb-1">
                Review Method
              </label>
              <NativeSelect
                value={formMethod}
                onChange={(e) => setFormMethod(e.target.value as ReviewMethod)}
                className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default focus:outline-none focus:border-border-default"
              >
                {REVIEW_METHODS.map((m) => (
                  <option key={m.value} value={m.value}>
                    {m.label}
                  </option>
                ))}
              </NativeSelect>
            </div>

            <div>
              <label className="block text-xs font-medium text-fg-subtle mb-1">
                Status
              </label>
              <NativeSelect
                value={formStatus}
                onChange={(e) => setFormStatus(e.target.value as ReviewStatus)}
                className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default focus:outline-none focus:border-border-default"
              >
                {REVIEW_STATUSES.map((s) => (
                  <option key={s.value} value={s.value}>
                    {s.label}
                  </option>
                ))}
              </NativeSelect>
            </div>
          </div>

          <div>
            <label className="block text-xs font-medium text-fg-subtle mb-1">
              Evidence Summary
            </label>
            <textarea
              rows={3}
              placeholder="Describe the verification findings, test results, or audit rationale..."
              value={formSummary}
              onChange={(e) => setFormSummary(e.target.value)}
              required
              className="w-full px-3 py-2 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default"
            />
          </div>

          <div>
            <label className="block text-xs font-medium text-fg-subtle mb-1">
              Evidence Digest (Optional SHA256 / Hash)
            </label>
            <input
              type="text"
              placeholder="e.g. sha256:4a3b8c..."
              value={formDigest}
              onChange={(e) => setFormDigest(e.target.value)}
              className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default font-mono"
            />
          </div>

          {/* Findings Builder */}
          <div className="pt-2 border-t border-border-subtle space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-xs font-semibold text-fg-subtle uppercase tracking-wider">
                Findings ({formFindings.length})
              </label>
            </div>

            {/* Mini Form for Finding */}
            <div className="p-3 rounded-lg border border-border-subtle bg-bg-canvas/50 space-y-2">
              <div className="grid grid-cols-4 gap-2">
                <div>
                  <NativeSelect
                    value={findingSeverity}
                    onChange={(e) => setFindingSeverity(e.target.value as FindingSeverity)}
                    className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default"
                  >
                    {SEVERITIES.map((s) => (
                      <option key={s.value} value={s.value}>
                        {s.label}
                      </option>
                    ))}
                  </NativeSelect>
                </div>
                <div className="col-span-3">
                  <input
                    type="text"
                    placeholder="Criterion (e.g. zero_clippy_warnings)"
                    value={findingCriterion}
                    onChange={(e) => setFindingCriterion(e.target.value)}
                    className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default font-mono"
                  />
                </div>
              </div>

              <input
                type="text"
                placeholder="Finding message / observation"
                value={findingMessage}
                onChange={(e) => setFindingMessage(e.target.value)}
                className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default"
              />

              <div className="grid grid-cols-3 gap-2">
                <div className="col-span-2">
                  <input
                    type="text"
                    placeholder="File path (optional)"
                    value={findingFilePath}
                    onChange={(e) => setFindingFilePath(e.target.value)}
                    className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default font-mono"
                  />
                </div>
                <div>
                  <input
                    type="number"
                    placeholder="Line # (opt)"
                    value={findingLineNumber}
                    onChange={(e) => setFindingLineNumber(e.target.value)}
                    className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default font-mono"
                  />
                </div>
              </div>

              <div className="flex justify-end pt-1">
                <button
                  type="button"
                  onClick={handleAddFinding}
                  className="px-2.5 py-1 text-xs rounded bg-bg-surface hover:bg-bg-subtle border border-border-subtle text-fg-default font-medium transition-colors"
                >
                  + Add Finding
                </button>
              </div>
            </div>

            {/* Findings List in Modal */}
            {formFindings.length > 0 && (
              <div className="space-y-1.5 max-h-36 overflow-y-auto">
                {formFindings.map((f, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between p-2 rounded border border-border-subtle bg-bg-surface text-xs"
                  >
                    <div className="flex items-center gap-2 truncate">
                      {renderSeverityBadge(f.severity)}
                      <span className="font-mono text-fg-default truncate">
                        {f.criterion}:
                      </span>
                      <span className="text-fg-subtle truncate">{f.message}</span>
                    </div>
                    <button
                      type="button"
                      onClick={() => handleRemoveFinding(i)}
                      className="text-fg-subtle hover:text-rose-400 p-1 transition-colors"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Modal Footer */}
          <div className="flex items-center justify-end gap-2 pt-4 border-t border-border-subtle">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-xs font-medium rounded-md border border-border-subtle hover:bg-bg-subtle text-fg-muted hover:text-fg-default transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={submitting}
              className="px-4 py-2 text-xs font-medium rounded-md bg-emerald-600 hover:bg-emerald-500 text-white transition-colors disabled:opacity-50 shadow-sm"
            >
              {submitting ? 'Recording...' : 'Record Review'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
