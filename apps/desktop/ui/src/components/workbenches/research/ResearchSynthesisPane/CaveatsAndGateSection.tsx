import React from 'react';
import {
  AlertTriangle,
  CheckCircle2,
  AlertCircle,
  ShieldCheck,
  ShieldAlert,
  ArrowRight,
} from 'lucide-react';
import type { ResearchClaim } from '@/types/research';

export interface CaveatsAndGateSectionProps {
  computedLiveCaveats: string[];
  enforceVerification: boolean;
  verificationViolations: ResearchClaim[];
  executing: boolean;
  onEnforceVerificationChange: (v: boolean) => void;
  onExecuteHandoff: () => void;
}

export const CaveatsAndGateSection: React.FC<CaveatsAndGateSectionProps> = ({
  computedLiveCaveats,
  enforceVerification,
  verificationViolations,
  executing,
  onEnforceVerificationChange,
  onExecuteHandoff,
}) => {
  return (
    <>
      {/* Section 4: Live Caveats & Invariants Generator */}
      <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
            <AlertTriangle size={13} className="text-amber-400" />
            <span>Synthesis Caveats & Pre-Condition Ledger</span>
          </div>
          <span className="text-[10px] font-mono text-[var(--color-fg-muted)]">
            {computedLiveCaveats.length} Caveat(s) Identified
          </span>
        </div>

        {computedLiveCaveats.length === 0 ? (
          <div className="p-3 rounded-lg bg-white/[0.04] border border-white/10 text-white/80 text-xs flex items-center gap-2">
            <CheckCircle2 size={14} className="text-white/70" />
            <span>All selected claims satisfy Gate 4 verification preconditions. Zero grounding caveats detected.</span>
          </div>
        ) : (
          <div className="space-y-1.5">
            {computedLiveCaveats.map((cav, idx) => (
              <div
                key={idx}
                className="p-2.5 rounded-lg bg-white/[0.03] border border-white/10 text-white/70 text-xs flex items-start gap-2 leading-relaxed"
              >
                <AlertCircle size={13} className="text-white/50 shrink-0 mt-0.5" />
                <span>{cav}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Section 5: Fail-Closed Gate Controls & Final Action */}
      <div className="p-4 rounded-xl bg-[var(--color-surface-2)] border border-[var(--color-border-default)] space-y-4">
        <div className="flex items-center justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 text-xs font-semibold text-[var(--color-editor-fg)]">
              <ShieldCheck size={14} className="text-white/70" />
              <span>Enforce Verification Pre-conditions (Gate 4 Fail-Closed Posture)</span>
            </div>
            <p className="text-[10px] text-[var(--color-fg-muted)] mt-0.5">
              Requires reproducible verifier records (L2+) or cryptographic proof anchors.
            </p>
          </div>
          <label className="relative inline-flex items-center cursor-pointer shrink-0">
            <input
              type="checkbox"
              checked={enforceVerification}
              onChange={(e) => onEnforceVerificationChange(e.target.checked)}
              className="sr-only peer"
            />
            <div className="w-10 h-5 bg-[var(--color-surface-3)] peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-white/40"></div>
          </label>
        </div>

        {enforceVerification && verificationViolations.length > 0 && (
          <div className="p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/25 text-rose-300 text-xs flex items-start gap-2">
            <ShieldAlert size={14} className="text-rose-400 shrink-0 mt-0.5" />
            <div>
              <span className="font-semibold">Blocked:</span> {verificationViolations.length} claim(s) lack required L2+ verifier records.
            </div>
          </div>
        )}

        <div className="flex items-center justify-between pt-2 border-t border-[var(--color-border-muted)]">
          <div className="text-[11px] text-[var(--color-fg-subtle)]">
            Output: Immutable Task Contracts with <code className="font-mono text-[10px] text-zinc-300">Diff</code> and <code className="font-mono text-[10px] text-zinc-300">TestResult</code> evidence requirements.
          </div>
          <button
            onClick={onExecuteHandoff}
            disabled={executing || (enforceVerification && verificationViolations.length > 0)}
            className="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold transition flex items-center gap-2 shadow-xs disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <ArrowRight size={14} className={executing ? 'animate-pulse' : ''} />
            <span>Execute Handoff to Coding</span>
          </button>
        </div>
      </div>
    </>
  );
};
