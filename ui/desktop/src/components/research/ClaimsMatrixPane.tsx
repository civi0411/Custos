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

const MOCK_CLAIMS: ResearchClaim[] = [
  {
    id: 'claim_01',
    statement:
      'Zero-trust capability tickets eliminate double-dispatch race conditions in multi-agent swarms.',
    level: 'L3_SEALED',
    confidenceScore: 0.98,
    invariants: ['INV-01 Double-Dispatch Fencing', 'INV-02 Deterministic Replay'],
    sealedProofUri: 'cas://bafy2bzace4v3k99a77z',
    createdAt: Date.now() - 3600000,
    evidenceLinks: [
      {
        passageAnchorId: 'anc_01',
        sourceTitle: 'Self-Organizing Invariant Architectures (Nature 2024)',
        exactText:
          'In empirical evaluations across 10,000 runs, phantom state execution was reduced by 99.8% while maintaining zero I/O leakages.',
        relation: 'SUPPORTS',
        rationale: 'Empirical benchmark with 10k runs confirms rate reduction under invariant gate.',
        verifiedBy: 'deterministic_engine',
      },
    ],
  },
  {
    id: 'claim_02',
    statement:
      'Optimistic scheduling without CAS certificates exhibits 14.2% double-dispatch failure under >15ms network latency.',
    level: 'L2_VERIFIED',
    confidenceScore: 0.94,
    invariants: ['INV-NET-LATENCY-03'],
    createdAt: Date.now() - 1800000,
    evidenceLinks: [
      {
        passageAnchorId: 'anc_02',
        sourceTitle: 'Convergence Rates under Asymmetric Latency (ArXiv 2025)',
        exactText:
          'When latency jitter exceeds 15ms, optimistic scheduling incurs double-dispatch vulnerability unless fenced by invariant CAS certificates.',
        relation: 'SUPPORTS',
        rationale: 'Mathematical lower bound proof and synthetic jitter simulations.',
        verifiedBy: 'deterministic_engine',
      },
      {
        passageAnchorId: 'anc_03',
        sourceTitle: 'Legacy Distributed Consensus Review',
        exactText: 'Optimistic locking achieves 99% accuracy in local subnet clusters.',
        relation: 'QUALIFIES',
        rationale: 'Only holds true under sub-millisecond local LAN conditions.',
        verifiedBy: 'expert_review',
      },
    ],
  },
  {
    id: 'claim_03',
    statement:
      'Quantized 4-bit neural kernels match FP16 convergence when evaluated on protein folding benchmarks.',
    level: 'L1_CITED',
    confidenceScore: 0.65,
    invariants: ['INV-NUMERIC-STABILITY'],
    createdAt: Date.now() - 900000,
    evidenceLinks: [
      {
        passageAnchorId: 'anc_04',
        sourceTitle: 'Quantization Limits in Structural Biology',
        exactText: 'RMSD divergence stays below 0.5 Angstroms on small globular targets.',
        relation: 'QUALIFIES',
        rationale: 'Citation provided but full multi-domain replay is pending execution.',
        verifiedBy: 'expert_review',
      },
    ],
  },
  {
    id: 'claim_04',
    statement:
      'Arbitrary shell execution within agent loops can be audited post-hoc without execution fencing.',
    level: 'L0_UNGROUNDED',
    confidenceScore: 0.12,
    invariants: ['INV-SECURITY-ZERO-IO'],
    createdAt: Date.now() - 300000,
    evidenceLinks: [
      {
        passageAnchorId: 'anc_05',
        sourceTitle: 'Post-Hoc Log Auditing Limitations',
        exactText: 'Post-hoc scanning cannot prevent data exfiltration during runtime.',
        relation: 'REFUTES',
        rationale: 'Contradicted by empirical zero-day exfiltration benchmarks.',
        verifiedBy: 'deterministic_engine',
      },
    ],
  },
];

interface ClaimsMatrixPaneProps {
  onHandoffToCoding?: (selectedClaims: ResearchClaim[]) => void;
  onShowToast?: (msg: string) => void;
}

export const ClaimsMatrixPane: React.FC<ClaimsMatrixPaneProps> = ({
  onHandoffToCoding,
  onShowToast,
}) => {
  const [claims, setClaims] = useState<ResearchClaim[]>(MOCK_CLAIMS);
  const [selectedClaimId, setSelectedClaimId] = useState<string>(MOCK_CLAIMS[0].id);
  const [selectedForHandoff, setSelectedForHandoff] = useState<string[]>([
    MOCK_CLAIMS[0].id,
    MOCK_CLAIMS[1].id,
  ]);

  useEffect(() => {
    daemonClient
      .listResearchClaims()
      .then((loaded) => {
        if (loaded && loaded.length > 0) {
          setClaims(loaded);
          setSelectedClaimId(loaded[0].id);
        }
      })
      .catch((err) => console.warn('Using mock claims fallback:', err));
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
      onShowToast?.('Select at least one verified claim for coding handoff.');
      return;
    }

    for (const claim of handoffClaims) {
      try {
        await daemonClient.handoffClaimToCoding(claim.id, claim.statement);
      } catch (err) {
        console.warn(`Handoff for ${claim.id} handled locally:`, err);
      }
    }

    onHandoffToCoding?.(handoffClaims);
    onShowToast?.(
      `Handed off ${handoffClaims.length} verified claim(s) to Coding Workbench Task Spec!`
    );
  };

  const renderLevelBadge = (level: ClaimGroundingLevel) => {
    switch (level) {
      case 'L3_SEALED':
        return (
          <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-[#21262d] text-neutral-300 border border-[#30363d] font-sans text-[10px] font-bold ">
            <Lock size={12} /> L3 SEALED
          </span>
        );
      case 'L2_VERIFIED':
        return (
          <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-sans text-[10px] font-bold ">
            <CheckCircle2 size={12} /> L2 VERIFIED
          </span>
        );
      case 'L1_CITED':
        return (
          <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-amber-500/10 text-amber-400 border border-amber-500/20 font-sans text-[10px] font-semibold">
            <FileCheck size={12} /> L1 CITED
          </span>
        );
      case 'L0_UNGROUNDED':
        return (
          <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-rose-500/10 text-rose-400 border border-rose-500/20 font-sans text-[10px] font-semibold">
            <AlertCircle size={12} /> L0 UNGROUNDED
          </span>
        );
    }
  };

  return (
    <div className="flex h-full w-full bg-[#04080F] text-[#e0e6ed] overflow-hidden select-none font-sans">
      {/* ── LEFT: CLAIMS TABLE LIST (w-1/2) ── */}
      <div className="w-1/2 min-w-[280px] shrink-0 border-r border-[#1e2430] flex flex-col bg-[#080d16] z-10 relative">
        {/* Table Header */}
        <div className="min-h-14 py-2 px-5 border-b border-[#1e2430] flex flex-wrap items-center justify-between gap-2 shrink-0 bg-[#080d16]/80 ">
          <div className="flex items-center gap-2">
            <div className="p-1.5 rounded-lg bg-[#21262d] shadow-lg ">
              <ShieldCheck className="w-4 h-4 text-white" />
            </div>
            <span className="font-bold text-white text-[13px] tracking-wide">
              Claims Matrix <span className="text-[#6e7681] font-normal ml-1">L0 - L3</span>
            </span>
          </div>

          {/* Handoff Action Button */}
          <button
            onClick={handleHandoff}
            disabled={selectedForHandoff.length === 0}
            className="group flex items-center gap-2 px-3 py-1.5 rounded-lg bg-[#21262d] text-white text-[11.5px] font-bold  transition-all disabled:opacity-30 disabled:grayscale disabled:shadow-none  "
            title="Convert verified claims to invariant constraints for Coding Workbench"
          >
            <Code size={14} className="group-hover:animate-pulse" />
            <span>Handoff to Coding <span className="bg-white/20 px-1.5 rounded ml-1">{selectedForHandoff.length}</span></span>
          </button>
        </div>

        {/* Claims Table Rows */}
        <div className="flex-1 overflow-y-auto p-4 space-y-3 select-text relative">
          
          {claims.map((claim) => {
            const isSelected = claim.id === selectedClaimId;
            const isCheckedForHandoff = selectedForHandoff.includes(claim.id);
            const isVerifiable = claim.level === 'L2_VERIFIED' || claim.level === 'L3_SEALED';

            return (
              <div
                key={claim.id}
                onClick={() => setSelectedClaimId(claim.id)}
                className={`group p-4 rounded-2xl border cursor-pointer transition-all duration-300 ease-out relative overflow-hidden ${
                  isSelected
                    ? 'bg-[#21262d] border-[#30363d] '
                    : 'bg-[#0d131f]/60 border-[#1e2430] hover:bg-[#131926] hover:border-[#2d3342] hover:shadow-lg hover:-translate-y-0.5'
                }`}
              >

                {/* Row Header */}
                <div className="flex flex-wrap items-center justify-between gap-2 gap-3 mb-3 relative z-10">
                  <div className="flex items-center gap-3">
                    <div className="relative flex items-center justify-center">
                      <input
                        type="checkbox"
                        checked={isCheckedForHandoff}
                        onChange={(e) => toggleSelectForHandoff(claim.id, e as any)}
                        disabled={!isVerifiable}
                        className="peer relative appearance-none w-5 h-5 rounded-md border-2 border-[#2d3342] checked:border-[#30363d] checked:bg-[#21262d] cursor-pointer disabled:cursor-not-allowed disabled:opacity-30 transition-all"
                        title={
                          isVerifiable
                            ? 'Select for Coding Handoff'
                            : 'Only L2/L3 claims can be handed off'
                        }
                      />
                      <svg className="absolute w-3 h-3 pointer-events-none opacity-0 peer-checked:opacity-100 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="20 6 9 17 4 12"></polyline>
                      </svg>
                    </div>
                    {renderLevelBadge(claim.level)}
                  </div>

                  <div className="flex flex-col items-end gap-1">
                    <span className="font-sans text-[10px] text-[#6e7681] uppercase tracking-wider">
                      Confidence
                    </span>
                    <span
                      className={`font-sans text-[13px] font-bold ${
                        claim.confidenceScore >= 0.8
                          ? 'text-emerald-400 drop-'
                          : claim.confidenceScore >= 0.5
                          ? 'text-amber-400'
                          : 'text-rose-400'
                      }`}
                    >
                      {(claim.confidenceScore * 100).toFixed(0)}%
                    </span>
                  </div>
                </div>

                {/* Statement Body */}
                <p className={`text-[14.5px] font-medium leading-relaxed mb-4 relative z-10 transition-colors ${isSelected ? 'text-white' : 'text-[#c9d1d9] group-hover:text-white'}`}>
                  {claim.statement}
                </p>

                {/* Invariant Tag List */}
                <div className="flex flex-wrap items-center gap-2 relative z-10">
                  {claim.invariants.map((inv) => (
                    <span
                      key={inv}
                      className="font-sans text-[10px] px-2 py-1 rounded bg-[#111722] border border-[#2d3342] text-neutral-300 font-medium"
                    >
                      {inv}
                    </span>
                  ))}
                  {claim.sealedProofUri && (
                    <span className="flex items-center gap-1 font-sans text-[10px] px-2 py-1 rounded bg-[#21262d] text-neutral-300 border border-[#30363d]">
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
      <div className="flex-1 flex flex-col bg-[#04080F] overflow-hidden relative">
        {/* Drawer Header */}
        <div className="min-h-14 py-2 px-6 border-b border-[#1e2430] bg-[#080d16]/90  flex flex-wrap items-center justify-between gap-2 shrink-0 z-20">
          <div className="flex items-center gap-2">
            <div className="p-1.5 rounded-lg bg-[#21262d] border border-[#30363d]">
              <Scale className="w-4 h-4 text-[#8b949e]" />
            </div>
            <span className="font-bold text-white text-[13px] tracking-wide">
              Anchored Evidence
            </span>
          </div>
          <span className="text-[11px] text-[#8b949e] font-sans font-bold bg-[#21262d] px-2.5 py-1 rounded-md border border-[#30363d]">
            {activeClaim.evidenceLinks.length} Anchor(s)
          </span>
        </div>

        {/* Drawer Body */}
        <div className="flex-1 overflow-y-auto relative">
           
           <div className="p-8 space-y-8 select-text relative z-10 max-w-2xl mx-auto">
            {/* Active Statement Summary */}
            <div className="relative group">
              <div className="relative p-5 rounded-2xl bg-[#0d131f]/80  border border-[#30363d] space-y-2">
                <div className="flex items-center gap-2">
                  <div className="w-1.5 h-1.5 rounded-full bg-[#21262d] animate-pulse" />
                  <span className="text-[10px] text-neutral-300 font-bold uppercase tracking-widest">
                    Selected Proposition
                  </span>
                </div>
                <p className="text-[15px] text-white leading-relaxed font-serif font-medium italic">
                  "{activeClaim.statement}"
                </p>
              </div>
            </div>

            {/* Evidence List */}
            <div className="space-y-4">
              <div className="flex items-center gap-3">
                <div className="h-px bg-[#21262d] flex-1" />
                <h4 className="text-[11px] font-bold text-[#6e7681] uppercase tracking-[0.2em]">
                  Cross-Referenced Passages
                </h4>
                <div className="h-px bg-[#21262d] flex-1" />
              </div>

              {activeClaim.evidenceLinks.map((link, idx) => {
                const isSupport = link.relation === 'SUPPORTS';
                const isRefute = link.relation === 'REFUTES';

                const borderColorClass = isSupport ? 'border-emerald-500/30' : isRefute ? 'border-rose-500/30' : 'border-amber-500/30';
                const bgColorClass = isSupport ? 'bg-emerald-500/5' : isRefute ? 'bg-rose-500/5' : 'bg-amber-500/5';
                const badgeBg = isSupport ? 'bg-emerald-500/20 text-emerald-400' : isRefute ? 'bg-rose-500/20 text-rose-400' : 'bg-amber-500/20 text-amber-400';

                return (
                  <div
                    key={idx}
                    className={`p-5 rounded-2xl border ${borderColorClass} ${bgColorClass}  space-y-4 relative overflow-hidden group  transition-shadow`}
                  >
                    {/* Top line indicator */}
                    <div className={`absolute top-0 left-0 w-full h-1 ${isSupport ? 'bg-emerald-500/50' : isRefute ? 'bg-rose-500/50' : 'bg-amber-500/50'}`} />

                    <div className="flex items-start justify-between gap-4">
                      <span className="font-bold text-white text-[13px] leading-snug">
                        {link.sourceTitle || 'Source Document'}
                      </span>
                      <span
                        className={`font-sans text-[10px] font-bold px-2.5 py-1 rounded-md shrink-0 shadow-sm ${badgeBg}`}
                      >
                        {link.relation}
                      </span>
                    </div>

                    {/* Exact Text */}
                    <div className={`p-4 rounded-xl bg-[#04080F]/50 border-l-2 ${isSupport ? 'border-l-emerald-500/50' : isRefute ? 'border-l-rose-500/50' : 'border-l-amber-500/50'} text-[13.5px] text-[#c9d1d9] italic font-serif leading-relaxed shadow-inner`}>
                      "{link.exactText}"
                    </div>

                    {/* Verification Rationale */}
                    <div className="flex flex-col gap-2 pt-2 border-t border-white/5">
                      <div className="flex items-start gap-2">
                        <ChevronRight className="w-4 h-4 text-[#6e7681] shrink-0 mt-0.5" />
                        <span className="text-[12.5px] text-[#a1abb7] leading-relaxed font-medium">
                          {link.rationale}
                        </span>
                      </div>
                      <div className="flex justify-end">
                        <span className="font-sans text-[10px] px-2 py-0.5 rounded bg-[#111722] text-neutral-300 border border-[#30363d] flex items-center gap-1.5">
                          <CheckCircle2 size={10} className="text-neutral-300" />
                          verified: {link.verifiedBy}
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
