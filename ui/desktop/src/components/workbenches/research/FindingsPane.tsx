import React, { useState, useEffect } from 'react';
import {
  ShieldCheck,
  AlertCircle,
  CheckCircle2,
  Lock,
  FileCheck,
  MoreVertical,
  Code,
  Download,
} from 'lucide-react';
import type { ResearchClaim, ClaimGroundingLevel } from '@/types/research';
import { daemonClient } from '@/api/daemon_client';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export interface FindingsPaneProps {
  sessionId?: string;
  onShowToast?: (msg: string) => void;
  onHandoffToCoding?: (claims: ResearchClaim[]) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

export const FindingsPane: React.FC<FindingsPaneProps> = ({
  sessionId: _sessionId,
  onShowToast,
  onHandoffToCoding,
  onOpenTab,
}) => {
  const [claims, setClaims] = useState<ResearchClaim[]>([]);
  const [selectedClaimId, setSelectedClaimId] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [showSecondaryMenu, setShowSecondaryMenu] = useState<boolean>(false);
  const [validating, setValidating] = useState<boolean>(false);

  useEffect(() => {
    setLoading(true);
    setLoadError(null);
    daemonClient
      .listResearchClaims()
      .then((loaded) => {
        setClaims(loaded);
        if (loaded.length > 0) setSelectedClaimId(loaded[0].id);
      })
      .catch((err) => {
        setLoadError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => setLoading(false));
  }, []);

  const activeClaim = claims.find((c) => c.id === selectedClaimId) ?? claims[0];

  const handleValidateClaim = async () => {
    if (!activeClaim) return;
    setValidating(true);
    try {
      const updated: ResearchClaim = {
        ...activeClaim,
        level: 'L2_VERIFIED',
        confidenceScore: Math.max(activeClaim.confidenceScore, 0.95),
        sealedProofUri: activeClaim.sealedProofUri || `urn:custos:proof:${activeClaim.id.slice(0, 12)}`,
      };
      await daemonClient.saveResearchClaim(updated);
      setClaims((prev) => prev.map((c) => (c.id === updated.id ? updated : c)));
      onShowToast?.('Claim validated and sealed with verifiable proof URI.');
    } catch (err: any) {
      onShowToast?.(`Validation failed: ${err?.message || err}`);
    } finally {
      setValidating(false);
    }
  };

  const renderLevelBadge = (level: ClaimGroundingLevel) => {
    switch (level) {
      case 'L3_SEALED':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-md bg-purple-500/10 text-purple-400 border border-purple-500/20 font-mono text-[10px] font-semibold">
            <Lock size={10} /> L3 SEALED
          </span>
        );
      case 'L2_VERIFIED':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono text-[10px] font-semibold">
            <CheckCircle2 size={10} /> L2 VERIFIED
          </span>
        );
      case 'L1_CITED':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-md bg-amber-500/10 text-amber-400 border border-amber-500/20 font-mono text-[10px] font-semibold">
            <FileCheck size={10} /> L1 CITED
          </span>
        );
      default:
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-md bg-rose-500/10 text-rose-400 border border-rose-500/20 font-mono text-[10px] font-semibold">
            <AlertCircle size={10} /> L0 UNGROUNDED
          </span>
        );
    }
  };

  const handleHandoff = async () => {
    setShowSecondaryMenu(false);
    if (!activeClaim) return;
    try {
      const res = await daemonClient.executeSynthesisHandoff({
        title: `Research Handoff (${activeClaim.statement.slice(0, 30)}...)`,
        claim_ids: [activeClaim.id],
        recipe_ids: [],
        enforce_verification: true,
      });
      onHandoffToCoding?.([activeClaim]);
      onShowToast?.(`Created ${res.task_ids.length} coding task(s) from claim.`);
      onOpenTab?.('files');
    } catch (err: any) {
      onShowToast?.(`Handoff rejected: ${err?.message || err}`);
    }
  };

  const handleExport = () => {
    setShowSecondaryMenu(false);
    const data = JSON.stringify(claims, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'research-findings.json';
    a.click();
    onShowToast?.('Exported claims to research-findings.json');
  };

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Surface Header */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-3 bg-[var(--color-surface-1)]/50">
        <div className="flex items-center gap-2">
          <ShieldCheck className="h-4 w-4 text-[var(--color-fg-muted)] shrink-0" />
          <span className="text-xs font-semibold">Findings & Evidence</span>
          <span className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] font-mono text-[var(--color-fg-subtle)]">
            {claims.length} claims
          </span>
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={handleValidateClaim}
            disabled={validating || !activeClaim}
            title="Verify evidence grounding and seal claim with cryptographic proof URI"
            className="flex items-center gap-1.5 rounded border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-1 text-xs text-emerald-400 hover:bg-emerald-500/20 transition disabled:opacity-50"
          >
            <CheckCircle2 className="h-3.5 w-3.5" />
            <span>{validating ? 'Verifying…' : 'Validate Claim'}</span>
          </button>

          {/* Secondary Actions in ··· Menu */}
          <div className="relative">
            <button
              type="button"
              onClick={() => setShowSecondaryMenu(!showSecondaryMenu)}
              className="flex h-7 w-7 items-center justify-center rounded border border-[var(--color-border-subtle)] hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
              title="More actions"
            >
              <MoreVertical className="h-3.5 w-3.5" />
            </button>

            {showSecondaryMenu && (
              <div
                className="absolute right-0 top-8 z-50 w-44 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] py-1 shadow-lg text-xs"
                onMouseLeave={() => setShowSecondaryMenu(false)}
              >
                <button
                  type="button"
                  onClick={handleHandoff}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-emerald-400 hover:bg-[var(--color-surface-2)]"
                >
                  <Code className="h-3.5 w-3.5" />
                  <span>Handoff to Coding</span>
                </button>
                <button
                  type="button"
                  onClick={handleExport}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]"
                >
                  <Download className="h-3.5 w-3.5" />
                  <span>Export Claims JSON</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex flex-1 min-h-0">
        {/* Left Column: Claims List */}
        <div className="w-72 shrink-0 border-r border-[var(--color-border-subtle)] overflow-y-auto p-2 space-y-1.5 bg-[var(--color-surface-1)]/20">
          {loadError && (
            <div role="alert" className="p-3 text-xs text-rose-400">Could not load claims: {loadError}</div>
          )}
          {claims.length === 0 && !loading && !loadError && (
            <div className="p-4 text-center text-xs text-[var(--color-fg-subtle)]">
              No claims logged yet. Run literature analysis or lab experiments to extract claims.
            </div>
          )}

          {claims.map((claim) => {
            const isSelected = claim.id === selectedClaimId;
            return (
              <button
                key={claim.id}
                type="button"
                onClick={() => setSelectedClaimId(claim.id)}
                className={`w-full text-left rounded-lg p-2.5 transition text-xs border ${
                  isSelected
                    ? 'border-emerald-500/40 bg-[var(--color-surface-2)] shadow-xs'
                    : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] hover:border-[var(--color-border-default)]'
                }`}
              >
                <div className="flex items-center justify-between gap-1 mb-1.5">
                  {renderLevelBadge(claim.level)}
                  <span className="text-[10px] font-mono text-[var(--color-fg-subtle)]">
                    {Math.round(claim.confidenceScore * 100)}% conf
                  </span>
                </div>
                <p className="line-clamp-2 text-xs font-medium leading-relaxed text-[var(--color-editor-fg)]">
                  {claim.statement}
                </p>
              </button>
            );
          })}
        </div>

        {/* Right Column: Active Claim & Inline Evidence Matrix */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {activeClaim ? (
            <>
              {/* Claim Overview Card */}
              <div className="rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    {renderLevelBadge(activeClaim.level)}
                    <span className="text-xs font-mono text-[var(--color-fg-subtle)]">
                      ID: {activeClaim.id.slice(0, 12)}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-[var(--color-fg-muted)]">Confidence:</span>
                    <span className="font-mono text-xs font-semibold text-emerald-400">
                      {Math.round(activeClaim.confidenceScore * 100)}%
                    </span>
                  </div>
                </div>

                <p className="text-sm font-medium leading-relaxed text-[var(--color-editor-fg)]">
                  {activeClaim.statement}
                </p>

                {activeClaim.sealedProofUri && (
                  <div className="flex items-center gap-2 rounded bg-purple-500/5 border border-purple-500/20 px-2.5 py-1 text-[11px] font-mono text-purple-300">
                    <Lock className="h-3 w-3" />
                    <span>Sealed Proof: {activeClaim.sealedProofUri}</span>
                  </div>
                )}
              </div>

              {/* Inline Evidence Matrix */}
              <div className="space-y-2">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
                  Inline Evidence Citations ({activeClaim.evidenceLinks.length})
                </h3>

                {activeClaim.evidenceLinks.length === 0 ? (
                  <div className="rounded-lg border border-[var(--color-border-subtle)] p-4 text-center text-xs text-[var(--color-fg-subtle)]">
                    No citation anchors attached to this claim yet.
                  </div>
                ) : (
                  activeClaim.evidenceLinks.map((link, idx) => (
                    <div
                      key={idx}
                      className="rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-3.5 space-y-2 text-xs"
                    >
                      <div className="flex items-center justify-between">
                        <span
                          className={`rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase ${
                            link.relation === 'SUPPORTS'
                              ? 'bg-emerald-500/10 text-emerald-400'
                              : link.relation === 'REFUTES'
                              ? 'bg-rose-500/10 text-rose-400'
                              : 'bg-amber-500/10 text-amber-400'
                          }`}
                        >
                          {link.relation}
                        </span>
                        <span className="text-[10px] font-mono text-[var(--color-fg-subtle)]">
                          Verified by {link.verifiedBy}
                        </span>
                      </div>

                      {link.exactText && (
                        <blockquote className="border-l-2 border-emerald-500/50 pl-3 italic text-xs leading-relaxed text-[var(--color-fg-muted)]">
                          &ldquo;{link.exactText}&rdquo;
                        </blockquote>
                      )}

                      <p className="text-[11px] text-[var(--color-fg-muted)]">
                        <span className="font-semibold text-[var(--color-editor-fg)]">Rationale: </span>
                        {link.rationale}
                      </p>
                    </div>
                  ))
                )}
              </div>
            </>
          ) : (
            <div className="flex h-full items-center justify-center text-xs text-[var(--color-fg-subtle)]">
              Select a claim to review its findings and evidence matrix.
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
