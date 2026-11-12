import React from 'react';
import { CheckCircle2, Cpu, ShieldCheck, XCircle } from 'lucide-react';
import type { ArtifactInspectorData } from '@/types/research';

export const ArtifactEnvironmentTab: React.FC<{ activeEnv?: string }> = ({ activeEnv }) => (
  <div className="space-y-3 font-mono text-xs">
    <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-2">
      <div className="flex items-center gap-2 text-[10px] text-[var(--color-fg-subtle)] uppercase font-semibold tracking-wider">
        <Cpu size={13} /> Runtime Silicon & Environment
      </div>
      <div className="text-[var(--color-editor-fg)] leading-relaxed font-sans text-xs">
        {activeEnv || 'Environment receipt unavailable'}
      </div>
    </div>

    <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-2">
      <div className="flex items-center gap-2 text-[10px] text-[var(--color-fg-subtle)] uppercase font-semibold tracking-wider">
        <ShieldCheck size={13} /> Reproducibility Lockfile
      </div>
      <div className="text-xs text-[var(--color-fg-muted)] font-sans">
        No lockfile receipt is attached to this artifact version.
      </div>
    </div>
  </div>
);

export const ArtifactReviewTab: React.FC<{
  reviewStatus?: boolean | null;
  reviewFindings?: ArtifactInspectorData['reviewFindings'];
}> = ({ reviewStatus, reviewFindings }) => {
  const isPassed = reviewStatus === true;

  return (
    <div className="space-y-4">
      <div
        className={`p-4 rounded-xl border flex items-center justify-between ${
          reviewStatus == null
            ? 'bg-[var(--color-surface-1)] border-[var(--color-border-muted)]'
            : isPassed
              ? 'bg-emerald-500/5 border-emerald-500/30'
              : 'bg-rose-500/5 border-rose-500/30'
        }`}
      >
        <div className="flex items-center gap-3">
          {reviewStatus == null ? (
            <div className="p-1.5 rounded-full bg-amber-500/10 text-amber-500">
              <ShieldCheck className="w-5 h-5" />
            </div>
          ) : isPassed ? (
            <div className="p-1.5 rounded-full bg-emerald-500/10 text-emerald-500">
              <CheckCircle2 className="w-5 h-5" />
            </div>
          ) : (
            <div className="p-1.5 rounded-full bg-rose-500/10 text-rose-500">
              <XCircle className="w-5 h-5" />
            </div>
          )}
          <div>
            <div className="text-xs font-semibold text-[var(--color-editor-fg)]">
              {reviewStatus == null
                ? 'No Review Receipt'
                : isPassed
                  ? 'All Invariant Gates Passed'
                  : 'Invariant Check Failed'}
            </div>
            <div className="text-[11px] text-[var(--color-fg-muted)] mt-0.5">
              {reviewStatus == null
                ? 'Attach a versioned verifier result before treating this artifact as accepted.'
                : 'Status reported by the attached artifact review record.'}
            </div>
          </div>
        </div>
        <span
          className={`font-mono text-[10px] px-2.5 py-1 rounded-md font-semibold uppercase tracking-wider ${
            reviewStatus == null
              ? 'bg-amber-500/10 text-amber-500 border border-amber-500/30'
              : isPassed
                ? 'bg-emerald-500/10 text-emerald-500 border border-emerald-500/30'
                : 'bg-rose-500/10 text-rose-500 border border-rose-500/30'
          }`}
        >
          {reviewStatus == null ? 'Unknown' : isPassed ? 'Passed' : 'Failed'}
        </span>
      </div>

      <div className="space-y-2.5">
        <div className="flex items-center gap-3">
          <div className="h-px bg-[var(--color-border-muted)] flex-1" />
          <h4 className="text-[10px] font-semibold text-[var(--color-fg-subtle)] uppercase tracking-wider">
            Invariant Verification Logs
          </h4>
          <div className="h-px bg-[var(--color-border-muted)] flex-1" />
        </div>

        {(reviewFindings ?? []).map((f, idx) => (
          <div
            key={idx}
            className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-2"
          >
            <div className="flex items-center justify-between">
              <span className="font-medium text-xs text-[var(--color-editor-fg)]">{f.title}</span>
              <span
                className={`text-[10px] font-semibold font-mono px-2 py-0.5 rounded uppercase ${
                  f.level === 'ok'
                    ? 'bg-emerald-500/10 text-emerald-500 border border-emerald-500/20'
                    : f.level === 'warn'
                      ? 'bg-amber-500/10 text-amber-500 border border-amber-500/20'
                      : 'bg-rose-500/10 text-rose-500 border border-rose-500/20'
                }`}
              >
                {f.level}
              </span>
            </div>
            {f.evidence && (
              <div className="font-mono text-xs text-[var(--color-fg-muted)] bg-[var(--color-surface-2)] p-2.5 rounded-lg border border-[var(--color-border-muted)]">
                {f.evidence}
              </div>
            )}
          </div>
        ))}
        {!reviewFindings?.length && (
          <div className="rounded-xl border border-[var(--color-border-muted)] p-5 text-center text-xs text-[var(--color-fg-muted)]">
            No verifier findings are attached.
          </div>
        )}
      </div>
    </div>
  );
};
