import React, { useState } from 'react';
import { ShieldCheck, Trash2 } from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import {
  TARGET_TYPES,
  REVIEW_METHODS,
  REVIEW_STATUSES,
  SEVERITIES,
} from './reviewConsts';
import type {
  RecordReviewParams,
  ReviewTargetType,
  ReviewMethod,
  ReviewStatus,
  FindingSeverity,
  ReviewFinding,
} from '@/types/domain';

export interface RecordReviewFormProps {
  onSubmit: (params: RecordReviewParams) => Promise<void>;
  onClose: () => void;
  onShowToast?: (msg: string) => void;
}

function renderSeverityBadge(severity: FindingSeverity) {
  switch (severity) {
    case 'blocker':
      return (
        <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-rose-500/20 text-rose-300 border border-rose-500/30">
          BLOCKER
        </span>
      );
    case 'error':
      return (
        <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-rose-500/10 text-rose-400 border border-rose-500/20">
          ERROR
        </span>
      );
    case 'warning':
      return (
        <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20">
          WARN
        </span>
      );
    case 'info':
      return (
        <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20">
          INFO
        </span>
      );
    default:
      return null;
  }
}

export const RecordReviewForm: React.FC<RecordReviewFormProps> = ({
  onSubmit,
  onClose,
  onShowToast,
}) => {
  const [targetType, setTargetType] = useState<ReviewTargetType>('artifact');
  const [targetId, setTargetId] = useState('');
  const [reviewer, setReviewer] = useState('');
  const [method, setMethod] = useState<ReviewMethod>('automated_verifier');
  const [status, setStatus] = useState<ReviewStatus>('approved');
  const [summary, setSummary] = useState('');
  const [digest, setDigest] = useState('');
  const [findings, setFindings] = useState<ReviewFinding[]>([]);
  const [submitting, setSubmitting] = useState(false);

  // Finding mini-draft
  const [findingSeverity, setFindingSeverity] = useState<FindingSeverity>('info');
  const [findingCriterion, setFindingCriterion] = useState('');
  const [findingMessage, setFindingMessage] = useState('');
  const [findingFilePath, setFindingFilePath] = useState('');
  const [findingLineNumber, setFindingLineNumber] = useState('');

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
    setFindings([...findings, newFinding]);
    setFindingCriterion('');
    setFindingMessage('');
    setFindingFilePath('');
    setFindingLineNumber('');
  };

  const handleRemoveFinding = (index: number) => {
    setFindings(findings.filter((_, i) => i !== index));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!targetId.trim()) {
      onShowToast?.('Target identifier is required');
      return;
    }
    if (!reviewer.trim()) {
      onShowToast?.('Reviewer identifier is required');
      return;
    }
    if (!summary.trim()) {
      onShowToast?.('Evidence summary is required');
      return;
    }

    try {
      setSubmitting(true);
      await onSubmit({
        target_type: targetType,
        target_id: targetId.trim(),
        reviewer: reviewer.trim(),
        method,
        status,
        evidence_summary: summary.trim(),
        evidence_digest: digest.trim() || undefined,
        findings: findings.length > 0 ? findings : undefined,
      });
    } finally {
      setSubmitting(false);
    }
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
        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-6 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-medium text-fg-subtle mb-1">
                Target Type
              </label>
              <NativeSelect
                value={targetType}
                onChange={(e) => setTargetType(e.target.value as ReviewTargetType)}
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
                value={targetId}
                onChange={(e) => setTargetId(e.target.value)}
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
                value={reviewer}
                onChange={(e) => setReviewer(e.target.value)}
                required
                className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default font-mono"
              />
            </div>

            <div>
              <label className="block text-xs font-medium text-fg-subtle mb-1">
                Review Method
              </label>
              <NativeSelect
                value={method}
                onChange={(e) => setMethod(e.target.value as ReviewMethod)}
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
                value={status}
                onChange={(e) => setStatus(e.target.value as ReviewStatus)}
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
              value={summary}
              onChange={(e) => setSummary(e.target.value)}
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
              value={digest}
              onChange={(e) => setDigest(e.target.value)}
              className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default font-mono"
            />
          </div>

          {/* Findings Builder */}
          <div className="pt-2 border-t border-border-subtle space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-xs font-semibold text-fg-subtle uppercase tracking-wider">
                Findings ({findings.length})
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
            {findings.length > 0 && (
              <div className="space-y-1.5 max-h-36 overflow-y-auto">
                {findings.map((f, i) => (
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
