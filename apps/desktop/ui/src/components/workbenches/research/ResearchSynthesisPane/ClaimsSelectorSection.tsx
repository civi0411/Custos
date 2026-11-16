import React from 'react';
import { ShieldCheck, Search, Lock } from 'lucide-react';
import type { ResearchClaim } from '@/types/research';
import { renderGroundingBadge } from './badges';

export interface ClaimsSelectorSectionProps {
  claims: ResearchClaim[];
  selectedClaimIds: string[];
  claimSearch: string;
  isComposing: boolean;
  activeProposalStatus?: string;
  onClaimSearchChange: (q: string) => void;
  onToggleClaim: (id: string) => void;
}

export const ClaimsSelectorSection: React.FC<ClaimsSelectorSectionProps> = ({
  claims,
  selectedClaimIds,
  claimSearch,
  isComposing,
  activeProposalStatus,
  onClaimSearchChange,
  onToggleClaim,
}) => {
  const isReadOnly = !isComposing && activeProposalStatus === 'handoff_completed';

  return (
    <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
          <ShieldCheck size={13} />
          <span>Grounding Claims Matrix ({selectedClaimIds.length} Selected)</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search size={11} className="absolute left-2 top-1.5 text-[var(--color-fg-muted)]" />
            <input
              type="text"
              placeholder="Filter claims..."
              value={claimSearch}
              onChange={(e) => onClaimSearchChange(e.target.value)}
              className="bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded pl-6 pr-2 py-0.5 text-[11px] text-[var(--color-editor-fg)] focus:outline-none"
            />
          </div>
        </div>
      </div>

      <div className="max-h-60 overflow-y-auto space-y-2 pr-1">
        {claims
          .filter(
            (c) =>
              claimSearch === '' ||
              c.statement.toLowerCase().includes(claimSearch.toLowerCase()) ||
              c.id.toLowerCase().includes(claimSearch.toLowerCase())
          )
          .map((claim) => {
            const isChecked = selectedClaimIds.includes(claim.id);

            return (
              <div
                key={claim.id}
                onClick={() => {
                  if (!isReadOnly) {
                    onToggleClaim(claim.id);
                  }
                }}
                className={`p-3 rounded-lg border transition cursor-pointer flex items-start gap-3 ${
                  isChecked
                    ? 'bg-[var(--color-surface-2)] border-[var(--workbench-accent)]/60'
                    : 'bg-[var(--color-surface-0)] border-[var(--color-border-muted)] hover:border-[var(--color-border-default)]'
                }`}
              >
                <input
                  type="checkbox"
                  checked={isChecked}
                  onChange={() => onToggleClaim(claim.id)}
                  disabled={isReadOnly}
                  className="mt-0.5 rounded border-[var(--color-border-default)] accent-[var(--workbench-accent)] cursor-pointer"
                />
                <div className="flex-1 space-y-1">
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-mono text-[10px] text-[var(--color-fg-subtle)]">
                      {claim.id}
                    </span>
                    <div className="flex items-center gap-2">
                      {renderGroundingBadge(claim.level)}
                      <span className="font-mono text-[10px] text-[var(--color-fg-muted)]">
                        {(claim.confidenceScore * 100).toFixed(0)}% conf
                      </span>
                    </div>
                  </div>
                  <p className="text-xs text-[var(--color-editor-fg)] leading-relaxed">
                    "{claim.statement}"
                  </p>
                  {claim.sealedProofUri && (
                    <div className="font-mono text-[9px] text-purple-400 flex items-center gap-1">
                      <Lock size={9} /> Proof: {claim.sealedProofUri}
                    </div>
                  )}
                </div>
              </div>
            );
          })}
      </div>
    </div>
  );
};
