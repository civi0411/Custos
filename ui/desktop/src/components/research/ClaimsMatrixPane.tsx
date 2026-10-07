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
          <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-purple-500/10 text-purple-400 border border-purple-500/20 font-mono text-[10px] font-bold shadow-[0_0_10px_rgba(168,85,247,0.2)]">
            <Lock size={12} /> L3 SEALED
          </span>
        );
      case 'L2_VERIFIED':
        return (
          <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono text-[10px] font-bold shadow-[0_0_10px_rgba(16,185,129,0.2)]">
            <CheckCircle2 size={12} /> L2 VERIFIED
          </span>
        );
      case 'L1_CITED':
        return (
          <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-amber-500/10 text-amber-400 border border-amber-500/20 font-mono text-[10px] font-semibold">
            <FileCheck size={12} /> L1 CITED
          </span>
        );
      case 'L0_UNGROUNDED':
        return (
          <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-rose-500/10 text-rose-400 border border-rose-500/20 font-mono text-[10px] font-semibold">
            <AlertCircle size={12} /> L0 UNGROUNDED
          </span>
        );
    }
  };

  return (
    <div className="flex h-full w-full bg-[#04080F] text-[#e0e6ed] overflow-hidden select-none font-sans">
      {/* ── LEFT: CLAIMS TABLE LIST (w-[55%]) ── */}
      <div className="w-[55%] border-r border-[#1e2430] flex flex-col bg-[#080d16] shadow-[4px_0_24px_-10px_rgba(0,0,0,0.5)] z-10 relative">
        {/* Table Header */}
        <div className="h-14 px-5 border-b border-[#1e2430] flex items-center justify-between shrink-0 bg-[#080d16]/80 backdrop-blur-md">
          <div className="flex items-center gap-2">
            <div className="p-1.5 rounded-lg bg-gradient-to-br from-indigo-500 to-purple-600 shadow-lg shadow-indigo-500/20">
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
            className="group flex items-center gap-2 px-3 py-1.5 rounded-lg bg-gradient-to-r from-blue-500 to-indigo-600 text-white text-[11.5px] font-bold hover:shadow-[0_0_15px_rgba(59,130,246,0.4)] transition-all disabled:opacity-30 disabled:grayscale disabled:shadow-none hover:scale-105 active:scale-95"
            title="Convert verified claims to invariant constraints for Coding Workbench"
          >
            <Code size={14} className="group-hover:animate-pulse" />
            <span>Handoff to Coding <span className="bg-white/20 px-1.5 rounded ml-1">{selectedForHandoff.length}</span></span>
          </button>
        </div>

        {/* Claims Table Rows */}
        <div className="flex-1 overflow-y-auto p-4 space-y-3 select-text relative">
          <div className="absolute top-0 left-0 w-full h-[200px] bg-gradient-to-b from-indigo-500/5 to-transparent pointer-events-none" />
          
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
                    ? 'bg-gradient-to-br from-[#131926] to-[#0d131f] border-indigo-500/40 shadow-[0_8px_25px_-8px_rgba(99,102,241,0.2)]'
                    : 'bg-[#0d131f]/60 border-[#1e2430] hover:bg-[#131926] hover:border-[#2d3342] hover:shadow-lg hover:-translate-y-0.5'
                }`}
              >
                {isSelected && (
                  <div className="absolute left-0 top-0 bottom-0 w-1 bg-gradient-to-b from-indigo-400 to-purple-500" />
                )}

                {/* Row Header */}
                <div className="flex items-center justify-between gap-3 mb-3 relative z-10">
                  <div className="flex items-center gap-3">
                    <div className="relative flex items-center justify-center">
                      <input
                        type="checkbox"
                        checked={isCheckedForHandoff}
                        onChange={(e) => toggleSelectForHandoff(claim.id, e as any)}
                        disabled={!isVerifiable}
                        className="peer relative appearance-none w-5 h-5 rounded-md border-2 border-[#2d3342] checked:border-blue-500 checked:bg-blue-500 cursor-pointer disabled:cursor-not-allowed disabled:opacity-30 transition-all"
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
                    <span className="font-mono text-[10px] text-[#6e7681] uppercase tracking-wider">
                      Confidence
                    </span>
                    <span
                      className={`font-mono text-[13px] font-bold ${
                        claim.confidenceScore >= 0.8
                          ? 'text-emerald-400 drop-shadow-[0_0_8px_rgba(16,185,129,0.5)]'
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
                      className="font-mono text-[10px] px-2 py-1 rounded bg-[#111722] border border-[#2d3342] text-blue-300 font-medium"
                    >
                      {inv}
                    </span>
                  ))}
                  {claim.sealedProofUri && (
                    <span className="flex items-center gap-1 font-mono text-[10px] px-2 py-1 rounded bg-purple-500/10 text-purple-300 border border-purple-500/20">
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
        <div className="h-14 px-6 border-b border-[#1e2430] bg-[#080d16]/90 backdrop-blur-xl flex items-center justify-between shrink-0 z-20">
          <div className="flex items-center gap-2">
            <div className="p-1.5 rounded-lg bg-emerald-500/10 border border-emerald-500/20">
              <Scale className="w-4 h-4 text-emerald-400" />
            </div>
            <span className="font-bold text-white text-[13px] tracking-wide">
              Anchored Evidence
            </span>
          </div>
          <span className="text-[11px] text-emerald-400 font-mono font-bold bg-emerald-400/10 px-2.5 py-1 rounded-md border border-emerald-400/20">
            {activeClaim.evidenceLinks.length} Anchor(s)
          </span>
        </div>

        {/* Drawer Body */}
        <div className="flex-1 overflow-y-auto relative">
           <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[400px] h-[400px] bg-emerald-500/5 blur-[120px] rounded-full pointer-events-none" />
           
           <div className="p-8 space-y-8 select-text relative z-10 max-w-2xl mx-auto">
            {/* Active Statement Summary */}
            <div className="relative group">
              <div className="absolute inset-0 bg-gradient-to-br from-indigo-500/10 to-purple-500/10 rounded-2xl blur-md" />
              <div className="relative p-5 rounded-2xl bg-[#0d131f]/80 backdrop-blur-md border border-indigo-500/20 space-y-2">
                <div className="flex items-center gap-2">
                  <div className="w-1.5 h-1.5 rounded-full bg-indigo-400 animate-pulse" />
                  <span className="text-[10px] text-indigo-400 font-bold uppercase tracking-widest">
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
                <div className="h-px bg-gradient-to-r from-transparent to-[#2d3342] flex-1" />
                <h4 className="text-[11px] font-bold text-[#6e7681] uppercase tracking-[0.2em]">
                  Cross-Referenced Passages
                </h4>
                <div className="h-px bg-gradient-to-l from-transparent to-[#2d3342] flex-1" />
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
                    className={`p-5 rounded-2xl border ${borderColorClass} ${bgColorClass} backdrop-blur-sm space-y-4 relative overflow-hidden group hover:shadow-[0_0_20px_rgba(0,0,0,0.2)] transition-shadow`}
                  >
                    {/* Top line indicator */}
                    <div className={`absolute top-0 left-0 w-full h-1 ${isSupport ? 'bg-emerald-500/50' : isRefute ? 'bg-rose-500/50' : 'bg-amber-500/50'}`} />

                    <div className="flex items-start justify-between gap-4">
                      <span className="font-bold text-white text-[13px] leading-snug">
                        {link.sourceTitle || 'Source Document'}
                      </span>
                      <span
                        className={`font-mono text-[10px] font-bold px-2.5 py-1 rounded-md shrink-0 shadow-sm ${badgeBg}`}
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
                        <span className="font-mono text-[10px] px-2 py-0.5 rounded bg-[#111722] text-blue-400 border border-blue-500/20 flex items-center gap-1.5">
                          <CheckCircle2 size={10} className="text-blue-400" />
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
