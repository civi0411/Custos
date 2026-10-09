import React, { useState, useEffect } from 'react';
import {
  ShieldCheck,
  AlertCircle,
  CheckCircle2,
  Lock,
  Scale,
  Code,
  FileCheck,
  ChevronRight,
  Fingerprint
} from 'lucide-react';
import { ResearchClaim, ClaimGroundingLevel } from '@/types/research';
import { daemonClient } from '@/api/daemon_client';

interface ClaimsMatrixPaneProps {
  onHandoffToCoding?: (selectedClaims: ResearchClaim[]) => void;
  onShowToast?: (msg: string) => void;
}

export const ClaimsMatrixPane: React.FC<ClaimsMatrixPaneProps> = ({
  onHandoffToCoding,
  onShowToast,
}) => {
  const [claims, setClaims] = useState<ResearchClaim[]>([]);
  const [selectedClaimId, setSelectedClaimId] = useState<string>('');
  const [selectedForHandoff, setSelectedForHandoff] = useState<string[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    daemonClient
      .listResearchClaims()
      .then((loaded) => {
        setClaims(loaded);
        setSelectedClaimId(loaded[0]?.id ?? '');
      })
      .catch((err) => setLoadError(String(err)));
  }, []);

  const activeClaim = claims.find((c) => c.id === selectedClaimId) ?? claims[0];

  const toggleSelectForHandoff = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setSelectedForHandoff((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]
    );
  };

  const handleHandoff = async () => {
    const handoffClaims = claims.filter((c) => selectedForHandoff.includes(c.id));
    if (handoffClaims.length === 0) {
      onShowToast?.('Select at least one claim for Coding handoff. Verification remains separate.');
      return;
    }

    try {
      const result = await daemonClient.executeSynthesisHandoff({
        title: `Research Handoff (${handoffClaims.length} Claims)`,
        claim_ids: handoffClaims.map((c) => c.id),
        recipe_ids: [],
        enforce_verification: true,
      });
      onHandoffToCoding?.(handoffClaims);
      onShowToast?.(`Created ${result.task_ids.length} Invariant Coding task(s) via Gate 4 handoff.`);
      setSelectedForHandoff([]);
    } catch (err) {
      onShowToast?.(`Handoff rejected by Gate 4 fail-closed gate: ${String(err)}`);
    }
  };

  const renderLevelBadge = (level: ClaimGroundingLevel) => {
    switch (level) {
      case 'L3_SEALED':
        return (
          <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-purple-500/10 text-purple-400 border border-purple-500/20 font-mono text-[10px] font-semibold">
            <Lock size={11} /> L3 SEALED
          </span>
        );
      case 'L2_VERIFIED':
        return (
          <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 font-mono text-[10px] font-semibold">
            <CheckCircle2 size={11} /> L2 VERIFIED
          </span>
        );
      case 'L1_CITED':
        return (
          <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-amber-500/10 text-amber-500 border border-amber-500/20 font-mono text-[10px] font-semibold">
            <FileCheck size={11} /> L1 CITED
          </span>
        );
      case 'L0_UNGROUNDED':
        return (
          <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-rose-500/10 text-rose-400 border border-rose-500/20 font-mono text-[10px] font-semibold">
            <AlertCircle size={11} /> L0 UNGROUNDED
          </span>
        );
    }
  };

  if (!activeClaim) {
    return (
      <div className="flex h-full items-center justify-center bg-[var(--color-canvas)] p-6 text-xs text-[var(--color-fg-muted)]">
        {loadError ? `Research claims unavailable: ${loadError}` : 'No research claims yet.'}
      </div>
    );
  }

  return (
    <div className="flex h-full w-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] overflow-hidden select-none font-sans">
      {/* ── LEFT: CLAIMS TABLE LIST (w-[55%]) ── */}
      <div className="w-[55%] border-r border-[var(--color-border-muted)] flex flex-col bg-[var(--color-surface-1)] z-10 relative">
        {/* Table Header */}
        <div className="h-12 px-4 border-b border-[var(--color-border-muted)] flex items-center justify-between shrink-0 bg-[var(--color-surface-1)]">
          <div className="flex items-center gap-2">
            <ShieldCheck className="w-4 h-4 workbench-accent" />
            <span className="font-semibold text-[var(--color-editor-fg)] text-xs">
              Claims Matrix <span className="text-[var(--color-fg-subtle)] font-normal ml-1">L0 - L3</span>
            </span>
          </div>

          {/* Handoff Action Button */}
          <button
            onClick={handleHandoff}
            disabled={selectedForHandoff.length === 0}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] text-xs font-medium transition disabled:opacity-40 disabled:cursor-not-allowed"
            title="Create Coding tasks from selected research claims; verification is separate"
          >
            <Code size={13} />
            <span>Handoff to Coding</span>
            {selectedForHandoff.length > 0 && (
              <span className="bg-[var(--color-surface-3)] px-1.5 rounded text-[10px] font-mono font-semibold">
                {selectedForHandoff.length}
              </span>
            )}
          </button>
        </div>

        {/* Claims Table Rows */}
        <div className="flex-1 overflow-y-auto p-3.5 space-y-2 select-text relative">
          {claims.map((claim) => {
            const isSelected = claim.id === selectedClaimId;
            const isCheckedForHandoff = selectedForHandoff.includes(claim.id);
            const isVerifiable = claim.level === 'L2_VERIFIED' || claim.level === 'L3_SEALED';

            return (
              <div
                key={claim.id}
                onClick={() => setSelectedClaimId(claim.id)}
                className={`group p-3.5 rounded-xl border cursor-pointer transition relative overflow-hidden ${
                  isSelected
                    ? 'bg-[var(--color-surface-2)] border-[var(--color-border-default)] shadow-xs'
                    : 'bg-[var(--color-surface-0)] border-[var(--color-border-muted)] hover:border-[var(--color-border-default)] hover:bg-[var(--color-surface-2)]/60'
                }`}
              >
                {isSelected && (
                  <div className="absolute left-0 top-0 bottom-0 w-1 bg-[var(--workbench-accent)]" />
                )}

                {/* Row Header */}
                <div className="flex items-center justify-between gap-3 mb-2 relative z-10">
                  <div className="flex items-center gap-2.5">
                    <div className="relative flex items-center justify-center">
                      <input
                        type="checkbox"
                        checked={isCheckedForHandoff}
                        onChange={(e) => toggleSelectForHandoff(claim.id, e as any)}
                        disabled={!isVerifiable}
                        className="peer appearance-none w-4 h-4 rounded border border-[var(--color-border-default)] checked:border-[var(--workbench-accent)] checked:bg-[var(--workbench-accent)] cursor-pointer disabled:cursor-not-allowed disabled:opacity-30 transition"
                        title={
                          isVerifiable
                            ? 'Select for Coding Handoff'
                            : 'Only L2/L3 claims can be handed off'
                        }
                      />
                      <svg className="absolute w-2.5 h-2.5 pointer-events-none opacity-0 peer-checked:opacity-100 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="20 6 9 17 4 12"></polyline>
                      </svg>
                    </div>
                    {renderLevelBadge(claim.level)}
                  </div>

                  <div className="flex items-center gap-1.5">
                    <span className="font-mono text-[10px] text-[var(--color-fg-subtle)] uppercase">
                      Confidence
                    </span>
                    <span
                      className={`font-mono text-xs font-semibold ${
                        claim.confidenceScore >= 0.8
                          ? 'text-emerald-500'
                          : claim.confidenceScore >= 0.5
                          ? 'text-amber-500'
                          : 'text-rose-500'
                      }`}
                    >
                      {(claim.confidenceScore * 100).toFixed(0)}%
                    </span>
                  </div>
                </div>

                {/* Statement Body */}
                <p className="text-xs font-medium leading-relaxed mb-3 relative z-10 text-[var(--color-editor-fg)]">
                  {claim.statement}
                </p>

                {/* Invariant Tag List */}
                <div className="flex flex-wrap items-center gap-1.5 relative z-10">
                  {claim.invariants.map((inv) => (
                    <span
                      key={inv}
                      className="font-mono text-[10px] px-2 py-0.5 rounded bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] text-[var(--color-fg-muted)] font-medium"
                    >
                      {inv}
                    </span>
                  ))}
                  {claim.sealedProofUri && (
                    <span className="flex items-center gap-1 font-mono text-[10px] px-2 py-0.5 rounded bg-purple-500/10 text-purple-400 border border-purple-500/20">
                      <Fingerprint size={10} />
                      {claim.sealedProofUri}
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* ── RIGHT: EVIDENCE & RATIONALE DRAWER (w-[45%]) ── */}
      <div className="flex-1 flex flex-col bg-[var(--color-canvas)] overflow-hidden relative">
        {/* Drawer Header */}
        <div className="h-12 px-5 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] flex items-center justify-between shrink-0 z-20">
          <div className="flex items-center gap-2">
            <Scale className="w-4 h-4 text-[var(--color-fg-muted)]" />
            <span className="font-semibold text-[var(--color-editor-fg)] text-xs">
              Anchored Evidence
            </span>
          </div>
          <span className="text-[10px] text-[var(--color-fg-muted)] font-mono font-medium bg-[var(--color-surface-2)] px-2 py-0.5 rounded border border-[var(--color-border-muted)]">
            {activeClaim.evidenceLinks.length} Anchor(s)
          </span>
        </div>

        {/* Drawer Body */}
        <div className="flex-1 overflow-y-auto relative p-6">
          <div className="space-y-6 select-text relative z-10 max-w-2xl mx-auto">
            {/* Active Statement Summary */}
            <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-2">
              <div className="flex items-center gap-2">
                <span className="workbench-kicker">
                  Selected Proposition
                </span>
              </div>
              <p className="text-xs text-[var(--color-editor-fg)] leading-relaxed font-medium">
                "{activeClaim.statement}"
              </p>
            </div>

            {/* Evidence List */}
            <div className="space-y-3">
              <div className="flex items-center gap-3">
                <div className="h-px bg-[var(--color-border-muted)] flex-1" />
                <h4 className="text-[10px] font-semibold text-[var(--color-fg-subtle)] uppercase tracking-wider">
                  Cross-Referenced Passages
                </h4>
                <div className="h-px bg-[var(--color-border-muted)] flex-1" />
              </div>

              {activeClaim.evidenceLinks.map((link, idx) => {
                const isSupport = link.relation === 'SUPPORTS';
                const isRefute = link.relation === 'REFUTES';

                const relationColor = isSupport
                  ? 'text-emerald-500 bg-emerald-500/10 border-emerald-500/20'
                  : isRefute
                  ? 'text-rose-500 bg-rose-500/10 border-rose-500/20'
                  : 'text-amber-500 bg-amber-500/10 border-amber-500/20';

                return (
                  <div
                    key={idx}
                    className="p-4 rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] space-y-3 transition"
                  >
                    <div className="flex items-start justify-between gap-4">
                      <span className="font-semibold text-[var(--color-editor-fg)] text-xs leading-snug">
                        {link.sourceTitle || 'Source Document'}
                      </span>
                      <span
                        className={`font-mono text-[10px] font-semibold px-2 py-0.5 rounded border shrink-0 ${relationColor}`}
                      >
                        {link.relation}
                      </span>
                    </div>

                    {/* Exact Text */}
                    <div className={`p-3 rounded-lg bg-[var(--color-surface-2)] border-l-2 ${
                      isSupport ? 'border-l-emerald-500' : isRefute ? 'border-l-rose-500' : 'border-l-amber-500'
                    } text-xs text-[var(--color-editor-fg)] leading-relaxed`}>
                      "{link.exactText}"
                    </div>

                    {/* Verification Rationale */}
                    <div className="flex flex-col gap-2 pt-2 border-t border-[var(--color-border-muted)]">
                      <div className="flex items-start gap-1.5">
                        <ChevronRight className="w-3.5 h-3.5 text-[var(--color-fg-subtle)] shrink-0 mt-0.5" />
                        <span className="text-xs text-[var(--color-fg-muted)] leading-relaxed">
                          {link.rationale}
                        </span>
                      </div>
                      <div className="flex justify-end">
                        <span className="font-mono text-[10px] px-2 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border border-[var(--color-border-muted)] flex items-center gap-1.5">
                          <CheckCircle2 size={10} className="text-emerald-500" />
                          assessment: {link.verifiedBy}
                        </span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
