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
  Plus,
  Search,
  PanelLeftClose,
  PanelLeft,
  Copy,
  Check,
  X,
} from 'lucide-react';
import type { ReviewerRecord } from '@/types/domain';
import type { ResearchClaim, ClaimGroundingLevel } from '@/types/research';
import { daemonClient } from '@/api/daemon_client';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export interface FindingsPaneProps {
  sessionId?: string;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (id: OrcaTabId, url?: string, title?: string) => void;
}

export const FindingsPane: React.FC<FindingsPaneProps> = ({
  sessionId: _sessionId,
  onShowToast,
  onOpenTab,
}) => {
  const [claims, setClaims] = useState<ResearchClaim[]>([]);
  const [selectedClaimId, setSelectedClaimId] = useState<string>('');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [showSecondaryMenu, setShowSecondaryMenu] = useState<boolean>(false);
  const [validating, setValidating] = useState<boolean>(false);
  const [isSidebarOpen, setIsSidebarOpen] = useState<boolean>(true);
  const [copiedProof, setCopiedProof] = useState<boolean>(false);

  // New claim modal state
  const [isNewClaimOpen, setIsNewClaimOpen] = useState(false);
  const [newStatement, setNewStatement] = useState('');
  const [reviewClaim, setReviewClaim] = useState<ResearchClaim | null>(null);
  const [reviews, setReviews] = useState<ReviewerRecord[]>([]);
  const [reviewError, setReviewError] = useState<string | null>(null);

  const loadClaims = () => {
    setLoading(true);
    setLoadError(null);
    daemonClient
      .listResearchClaims()
      .then((loaded) => {
        setClaims(loaded || []);
        if (loaded && loaded.length > 0 && !selectedClaimId) {
          setSelectedClaimId(loaded[0].id);
        }
      })
      .catch((err) => {
        setLoadError(err instanceof Error ? err.message : String(err));
        setClaims([]);
      })
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadClaims();
  }, []);

  const activeClaim = claims.find((c) => c.id === selectedClaimId) ?? claims[0];

  const handleValidateClaim = async () => {
    if (!activeClaim || validating) return;
    setReviewClaim(activeClaim); setReviews([]); setReviewError(null); setValidating(true);
    try { setReviews(await daemonClient.listReviews({ target_id: activeClaim.id })); }
    catch (error) { setReviewError(String(error)); }
    finally { setValidating(false); }
  };

  const handleCreateClaim = async () => {
    if (!newStatement.trim()) return;
    try {
      const id = crypto.randomUUID();
      const claim: ResearchClaim = {
        id,
        statement: newStatement.trim(),
        level: 'L0_UNGROUNDED',
        confidenceScore: 0,
        invariants: [],
        evidenceLinks: [],
        createdAt: Date.now(),
      };
      await daemonClient.saveResearchClaim(claim);
      onShowToast?.('New research claim logged.');
      setIsNewClaimOpen(false);
      setNewStatement('');
      loadClaims();
      setSelectedClaimId(id);
    } catch (err: any) {
      onShowToast?.(`Failed to save claim: ${err?.message || err}`);
    }
  };

  const renderLevelBadge = (level: ClaimGroundingLevel) => {
    switch (level) {
      case 'L3_SEALED':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-md bg-purple-500/10 text-purple-400 border border-purple-500/20 font-mono text-[10px] font-semibold">
            <Lock size={10} /> Sealed record
          </span>
        );
      case 'L2_VERIFIED':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono text-[10px] font-semibold">
            <CheckCircle2 size={10} /> Verified record
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

  const handleHandoff = () => {
    setShowSecondaryMenu(false);
    if (!activeClaim || !onOpenTab) return;
    onOpenTab('synthesis', `custos://synthesis?claim=${encodeURIComponent(activeClaim.id)}`, 'Synthesis');
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

  const handleCopyProof = (uri: string) => {
    void navigator.clipboard.writeText(uri);
    setCopiedProof(true);
    setTimeout(() => setCopiedProof(false), 2000);
    onShowToast?.('Proof URI copied.');
  };

  const filteredClaims = claims.filter((c) =>
    c.statement.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="flex h-full w-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)] overflow-hidden font-sans">
      {reviewClaim && <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-6" role="dialog" aria-modal="true" aria-label="Claim review records">
        <section className="w-full max-w-xl max-h-[80vh] overflow-auto rounded-xl border border-border-default bg-canvas p-4 space-y-3">
          <div className="flex items-center justify-between gap-3"><h2 className="text-sm font-semibold">Claim review records</h2><button type="button" autoFocus aria-label="Close reviews" className="canvas-icon" onClick={() => setReviewClaim(null)}><X size={16} /></button></div>
          <p className="text-xs text-fg-muted">{reviewClaim.statement}</p>
          {validating ? <p role="status" className="text-xs">Loading records…</p> : reviewError ? <p role="alert" className="text-xs text-rose-400">{reviewError}</p> : reviews.length === 0 ? <p className="text-xs text-fg-muted">No review records attached.</p> : reviews.map(review => <article key={review.id} className="rounded border border-border-default p-3 text-xs space-y-2">
            <div className="flex justify-between gap-2"><strong>{review.reviewer}</strong><span>{review.status} · {review.is_fresh ? 'Current' : 'Stale'}</span></div>
            <p>{review.evidence_summary}</p><p className="text-fg-muted">{review.method} · {review.id}</p>
            {review.evidence_digest && <code className="block break-all text-[11px]">{review.evidence_digest}</code>}
          </article>)}
        </section>
      </div>}
      {/* ── Surface Header ── */}
      <div className="wb-pane-toolbar flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] px-3 bg-[var(--color-surface-1)]">
        <div className="flex items-center gap-2 min-w-0">
          {!isSidebarOpen && (
            <button
              type="button"
              onClick={() => setIsSidebarOpen(true)}
              className="p-1 rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition shrink-0"
              title="Show claims list"
            >
              <PanelLeft size={14} />
            </button>
          )}
          <ShieldCheck className="h-4 w-4 text-workbench-accent shrink-0" />
          <span className="text-xs font-semibold text-[var(--color-editor-fg)] truncate">
            Findings & Evidence
          </span>
          <span className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.2 font-mono text-[9px] text-[var(--color-fg-subtle)] border border-[var(--color-border-muted)] shrink-0">
            {claims.length} claims
          </span>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <button
            type="button"
            onClick={handleValidateClaim}
            disabled={validating || !activeClaim}
            title="Inspect saved reviews for this claim"
            className="flex items-center gap-1.5 rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-1 text-xs font-medium text-emerald-400 hover:bg-emerald-500/20 transition disabled:opacity-50"
          >
            <CheckCircle2 className="h-3.5 w-3.5" />
            <span>{validating ? 'Loading…' : 'Inspect reviews'}</span>
          </button>

          {/* Secondary Actions in ··· Menu */}
          <div className="relative">
            <button
              type="button"
              onClick={() => setShowSecondaryMenu(!showSecondaryMenu)}
              className="flex h-7 w-7 items-center justify-center rounded-md border border-[var(--color-border-default)] hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
              title="More actions"
            >
              <MoreVertical className="h-3.5 w-3.5" />
            </button>

            {showSecondaryMenu && (
              <div
                className="absolute right-0 top-8 z-50 w-44 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] py-1 shadow-2xl text-xs"
                onMouseLeave={() => setShowSecondaryMenu(false)}
              >
                <button
                  type="button"
                  disabled={!activeClaim || !onOpenTab}
                  onClick={handleHandoff}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-emerald-400 hover:bg-[var(--color-surface-2)]"
                >
                  <Code className="h-3.5 w-3.5" />
                  <span>Prepare in Synthesis</span>
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

      {/* ── Main Content Area: Collapsible Sidebar + Detail Canvas ── */}
      <div className="wb-split flex flex-1 min-h-0">
        {/* Left Column: Claims List */}
        {isSidebarOpen && (
          <div className="wb-master w-72 md:w-80 shrink-0 border-r border-[var(--color-border-muted)] flex flex-col bg-[var(--color-surface-1)]">
            <div className="h-9 px-3 border-b border-[var(--color-border-muted)] flex items-center justify-between shrink-0">
              <span className="text-[11px] font-semibold text-[var(--color-fg-muted)] uppercase tracking-wider">
                Evidence Matrix
              </span>
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => setIsNewClaimOpen(true)}
                  className="p-1 rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
                  title="Add new claim"
                >
                  <Plus size={14} />
                </button>
                <button
                  type="button"
                  onClick={() => setIsSidebarOpen(false)}
                  className="p-1 rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
                  title="Collapse claims panel"
                >
                  <PanelLeftClose size={14} />
                </button>
              </div>
            </div>

            {/* Search */}
            <div className="p-2 border-b border-[var(--color-border-muted)]">
              <div className="relative group">
                <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--color-fg-subtle)]" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Filter claims…"
                  className="w-full bg-[var(--color-canvas)] border border-[var(--color-border-muted)] rounded-lg py-1.5 pl-8 pr-2 text-xs text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] outline-none"
                />
              </div>
            </div>

            <div className="flex-1 overflow-y-auto p-2 space-y-1.5 no-scrollbar">
              {loadError && (
                <div role="alert" className="p-3 text-xs text-rose-400">
                  Could not load claims: {loadError}
                </div>
              )}
              {filteredClaims.length === 0 && !loading && !loadError && (
                <div className="p-4 text-center text-xs text-[var(--color-fg-subtle)]">
                  {searchQuery ? 'No matching claims.' : 'No claims logged yet.'}
                </div>
              )}

              {filteredClaims.map((claim) => {
                const isSelected = claim.id === selectedClaimId;
                return (
                  <button
                    key={claim.id}
                    type="button"
                    onClick={() => setSelectedClaimId(claim.id)}
                    className={`w-full text-left rounded-xl p-2.5 transition text-xs border ${
                      isSelected
                        ? 'border-emerald-500/40 bg-[var(--color-surface-2)] shadow-xs'
                        : 'border-[var(--color-border-muted)] bg-[var(--color-surface-1)] hover:border-[var(--color-border-default)]'
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
          </div>
        )}

        {/* Right Column: Active Claim & Inline Evidence Matrix */}
        <div className="flex-1 overflow-y-auto p-6 max-w-4xl mx-auto w-full space-y-6 no-scrollbar">
          {activeClaim ? (
            <>
              {/* Claim Overview Card */}
              <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-5 space-y-4 shadow-xs">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    {renderLevelBadge(activeClaim.level)}
                    <span className="text-[11px] font-mono text-[var(--color-fg-subtle)]">
                      ID: {activeClaim.id.slice(0, 12)}…
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-[var(--color-fg-muted)]">Confidence:</span>
                    <span className="font-mono text-xs font-bold text-emerald-400">
                      {Math.round(activeClaim.confidenceScore * 100)}%
                    </span>
                  </div>
                </div>

                <p className="text-sm lg:text-base font-semibold leading-relaxed text-[var(--color-editor-fg)]">
                  {activeClaim.statement}
                </p>

                {/* Grounding progression indicator */}
                <div className="space-y-1.5 pt-1">
                  <div className="flex justify-between text-[10px] text-[var(--color-fg-subtle)] font-mono">
                    <span className={activeClaim.level === 'L0_UNGROUNDED' ? 'text-rose-400 font-semibold' : ''}>L0 Ungrounded</span>
                    <span className={activeClaim.level === 'L1_CITED' ? 'text-amber-400 font-semibold' : ''}>L1 Cited</span>
                    <span className={activeClaim.level === 'L2_VERIFIED' ? 'text-emerald-400 font-semibold' : ''}>L2 Verified</span>
                    <span className={activeClaim.level === 'L3_SEALED' ? 'text-purple-400 font-semibold' : ''}>L3 Sealed</span>
                  </div>
                  <div className="h-1.5 w-full bg-[var(--color-surface-3)] rounded-full overflow-hidden">
                    <div
                      className={`h-full transition-all duration-300 ${
                        activeClaim.level === 'L3_SEALED'
                          ? 'w-full bg-purple-500'
                          : activeClaim.level === 'L2_VERIFIED'
                          ? 'w-3/4 bg-emerald-500'
                          : activeClaim.level === 'L1_CITED'
                          ? 'w-1/2 bg-amber-500'
                          : 'w-1/4 bg-rose-500'
                      }`}
                    />
                  </div>
                </div>

                {/* Invariant rules */}
                {activeClaim.invariants && activeClaim.invariants.length > 0 && (
                  <div className="flex flex-wrap items-center gap-1.5 pt-1">
                    <span className="text-[10px] text-[var(--color-fg-subtle)] font-medium uppercase tracking-wider mr-1">
                      Invariants:
                    </span>
                    {activeClaim.invariants.map((inv) => (
                      <span
                        key={inv}
                        className="rounded bg-[var(--color-surface-2)] px-2 py-0.5 font-mono text-[9.5px] text-[var(--color-fg-muted)] border border-[var(--color-border-muted)]"
                      >
                        {inv}
                      </span>
                    ))}
                  </div>
                )}

                {/* Sealed Proof URI */}
                {activeClaim.sealedProofUri && (
                  <div className="flex items-center justify-between gap-2 rounded-lg bg-purple-500/5 border border-purple-500/20 px-3 py-1.5 text-xs font-mono text-purple-300">
                    <div className="flex items-center gap-2 truncate">
                      <Lock className="h-3.5 w-3.5 shrink-0" />
                      <span className="truncate">Proof URI: {activeClaim.sealedProofUri}</span>
                    </div>
                    <button
                      type="button"
                      onClick={() => handleCopyProof(activeClaim.sealedProofUri!)}
                      className="p-1 hover:text-white transition shrink-0"
                      title="Copy proof URI"
                    >
                      {copiedProof ? <Check size={12} className="text-emerald-400" /> : <Copy size={12} />}
                    </button>
                  </div>
                )}
              </div>

              {/* Inline Evidence Matrix */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <h3 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
                    Evidence Citations ({activeClaim.evidenceLinks.length})
                  </h3>
                </div>

                {activeClaim.evidenceLinks.length === 0 ? (
                  <div className="rounded-xl border border-dashed border-[var(--color-border-muted)] p-6 text-center text-xs text-[var(--color-fg-subtle)]">
                    No citation anchors attached to this claim yet. Run synthesis or cite passages from the Literature pane to ground evidence.
                  </div>
                ) : (
                  activeClaim.evidenceLinks.map((link, idx) => (
                    <div
                      key={idx}
                      className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 space-y-2.5 text-xs"
                    >
                      <div className="flex items-center justify-between">
                        <span
                          className={`rounded px-2 py-0.5 text-[10px] font-semibold uppercase ${
                            link.relation === 'SUPPORTS'
                              ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
                              : link.relation === 'REFUTES'
                              ? 'bg-rose-500/10 text-rose-400 border border-rose-500/20'
                              : 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
                          }`}
                        >
                          {link.relation}
                        </span>
                        <span className="text-[10px] font-mono text-[var(--color-fg-subtle)]">
                          Verified by {link.verifiedBy}
                        </span>
                      </div>

                      {link.exactText && (
                        <blockquote className="border-l-2 border-emerald-500/60 bg-[var(--color-surface-2)]/50 p-2.5 rounded-r-lg italic text-xs leading-relaxed text-[var(--color-editor-fg)]">
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
            <div className="flex h-64 flex-col items-center justify-center text-center text-[var(--color-fg-subtle)] space-y-2">
              <ShieldCheck className="h-8 w-8 text-[var(--color-border-default)]" />
              <div className="text-xs font-medium text-[var(--color-editor-fg)]">No claim selected</div>
            </div>
          )}
        </div>
      </div>

      {/* ── Add Claim Modal ── */}
      {isNewClaimOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-lg rounded-xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] p-5 shadow-2xl space-y-4">
            <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-3">
              <h3 className="text-sm font-semibold text-[var(--color-editor-fg)] flex items-center gap-2">
                <ShieldCheck size={16} className="text-teal-400" />
                Log New Research Claim
              </h3>
              <button
                type="button"
                onClick={() => setIsNewClaimOpen(false)}
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
                  value={newStatement}
                  onChange={(e) => setNewStatement(e.target.value)}
                  placeholder="e.g. Model inference throughput scales sublinearly when KV-cache exceeds L2 capacity..."
                  className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] outline-none"
                />
              </div>

              <p className="text-[10px] text-fg-muted">Created as an ungrounded draft (L0).</p>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-[var(--color-border-muted)]">
              <button
                type="button"
                onClick={() => setIsNewClaimOpen(false)}
                className="px-3 py-1.5 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleCreateClaim}
                disabled={!newStatement.trim()}
                className="rounded-lg bg-teal-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-teal-500 disabled:opacity-50 transition"
              >
                Log Claim
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default FindingsPane;
