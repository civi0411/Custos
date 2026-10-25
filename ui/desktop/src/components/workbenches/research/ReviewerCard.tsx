import React, { memo, useState } from 'react';
import { ChevronDown, ShieldCheck, X, AlertTriangle, CheckCircle2, AlertOctagon } from 'lucide-react';

export type FindingLevel = 'warn' | 'ok' | 'error';

export interface ReviewFinding {
  level: FindingLevel;
  title: string;
  tag?: string;
  check?: string;
  evidence?: string;
}

export interface ReviewerCardProps {
  heading?: string;
  note?: string;
  findings: ReviewFinding[];
  className?: string;
}

const BADGE_STYLES: Record<FindingLevel, { bg: string; text: string; border: string; icon: React.ElementType }> = {
  warn: {
    bg: 'bg-amber-500/15',
    text: 'text-amber-400',
    border: 'border-amber-500/30',
    icon: AlertTriangle,
  },
  ok: {
    bg: 'bg-emerald-500/15',
    text: 'text-emerald-400',
    border: 'border-emerald-500/30',
    icon: CheckCircle2,
  },
  error: {
    bg: 'bg-rose-500/15',
    text: 'text-rose-400',
    border: 'border-rose-500/30',
    icon: AlertOctagon,
  },
};

/**
 * Structured scientific reviewer findings card.
 * Traced from Open Science traceability-review & verification engine.
 * Renders citation, numerical, and figure traceability audit results.
 */
export const ReviewerCard: React.FC<ReviewerCardProps> = memo(function ReviewerCard({
  heading = 'Traceability Review Findings',
  note,
  findings,
  className = '',
}) {
  const [isOpen, setIsOpen] = useState(true);
  const [dismissed, setDismissed] = useState<ReadonlySet<number>>(new Set());

  const visible = findings
    .map((f, i) => [f, i] as const)
    .filter(([, i]) => !dismissed.has(i));

  const errorCount = visible.filter(([f]) => f.level === 'error').length;
  const warnCount = visible.filter(([f]) => f.level === 'warn').length;

  return (
    <div className={`rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] shadow-xs select-none ${className}`}>
      {/* Header Button */}
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        aria-expanded={isOpen}
        className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left hover:bg-[var(--color-surface-2)]/60 transition rounded-xl"
      >
        <div className="flex items-center gap-2.5 min-w-0">
          <ShieldCheck size={16} className={errorCount > 0 ? 'text-rose-400' : warnCount > 0 ? 'text-amber-400' : 'text-emerald-400'} />
          <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
            {heading}
          </span>
          <span className="text-xs text-[var(--color-fg-muted)] font-mono text-[11px]">
            ({visible.length} item{visible.length !== 1 ? 's' : ''}
            {errorCount > 0 && ` · ${errorCount} error${errorCount !== 1 ? 's' : ''}`}
            {warnCount > 0 && ` · ${warnCount} warning${warnCount !== 1 ? 's' : ''}`}
            {dismissed.size > 0 && ` · ${dismissed.size} dismissed`})
          </span>
        </div>

        <ChevronDown
          size={15}
          className={`text-[var(--color-fg-muted)] transition-transform duration-200 ${isOpen ? 'rotate-180' : ''}`}
        />
      </button>

      {/* Expanded Body */}
      {isOpen && (
        <div className="px-4 pb-4 space-y-3 pt-1 border-t border-[var(--color-border-subtle)]/50">
          {note && (
            <p className="text-xs text-[var(--color-fg-muted)] italic leading-relaxed pt-1">
              {note}
            </p>
          )}

          {visible.length === 0 && findings.length > 0 && (
            <p className="text-xs text-[var(--color-fg-subtle)] italic py-2">
              All review findings in this section have been dismissed for this session.
            </p>
          )}

          {visible.map(([f, i]) => {
            const badge = BADGE_STYLES[f.level];
            const BadgeIcon = badge.icon;

            return (
              <div
                key={i}
                className="group rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)]/50 p-3 space-y-1.5 transition hover:border-[var(--color-border-default)]"
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex items-center gap-2 min-w-0 flex-wrap">
                    <span
                      className={`inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] font-mono font-medium border uppercase tracking-wider ${badge.bg} ${badge.text} ${badge.border}`}
                    >
                      <BadgeIcon size={11} />
                      {f.level}
                    </span>

                    {(f.tag || f.check) && (
                      <span className="rounded bg-black/30 border border-white/10 px-1.5 py-0.5 text-[10px] font-mono text-[var(--color-fg-muted)]">
                        {f.tag ?? f.check}
                      </span>
                    )}

                    <span className="text-xs font-medium text-[var(--color-editor-fg)] break-words">
                      {f.title}
                    </span>
                  </div>

                  <button
                    type="button"
                    onClick={() => setDismissed((prev) => new Set([...prev, i]))}
                    className="shrink-0 p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-white/10 transition opacity-60 group-hover:opacity-100"
                    title="Dismiss finding"
                    aria-label={`Dismiss finding: ${f.title}`}
                  >
                    <X size={13} />
                  </button>
                </div>

                {f.evidence && (
                  <pre className="whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed text-[var(--color-fg-muted)] bg-black/40 border border-white/5 rounded-md p-2 mt-1">
                    {f.evidence}
                  </pre>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
});
