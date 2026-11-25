import React from 'react';
import { Layers, Search, Plus, ShieldCheck, FlaskConical } from 'lucide-react';
import type { ResearchSynthesisProposal } from '@/types/domain';
import { renderStatusBadge } from './badges';

export interface ProposalsSidebarProps {
  proposals: ResearchSynthesisProposal[];
  filteredProposals: ResearchSynthesisProposal[];
  selectedProposalId: string | null;
  isComposing: boolean;
  searchQuery: string;
  statusFilter: string;
  onSearchChange: (q: string) => void;
  onStatusFilterChange: (st: string) => void;
  onSelectProposal: (p: ResearchSynthesisProposal) => void;
  onStartNewProposal: () => void;
}

export const ProposalsSidebar: React.FC<ProposalsSidebarProps> = ({
  proposals,
  filteredProposals,
  selectedProposalId,
  isComposing,
  searchQuery,
  statusFilter,
  onSearchChange,
  onStatusFilterChange,
  onSelectProposal,
  onStartNewProposal,
}) => {
  return (
    <aside className="wb-master w-80 border-r border-[var(--color-border-muted)] flex flex-col bg-[var(--color-surface-1)]/50 shrink-0">
      {/* Search & Filter Header */}
      <div className="p-2.5 border-b border-[var(--color-border-muted)] space-y-2">
        <div className="relative">
          <Search
            size={13}
            className="absolute left-2.5 top-2 text-[var(--color-fg-muted)]"
          />
          <input
            type="text"
            placeholder="Search proposals..."
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded pl-8 pr-2 py-1 text-xs text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-muted)] focus:outline-none focus:border-[var(--workbench-accent)]"
          />
        </div>
        <div className="flex items-center gap-1 text-[10px]">
          <span className="text-[var(--color-fg-subtle)] uppercase tracking-wider font-semibold mr-1">
            Status:
          </span>
          {(['all', 'draft', 'submitted', 'handoff_completed'] as const).map((st) => (
            <button
              key={st}
              onClick={() => onStatusFilterChange(st)}
              className={`px-2 py-0.5 rounded capitalize transition ${
                statusFilter === st
                  ? 'bg-[var(--workbench-accent)] text-white font-semibold'
                  : 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-3)]'
              }`}
            >
              {st === 'handoff_completed' ? 'Completed' : st}
            </button>
          ))}
        </div>
      </div>

      {/* Proposals List */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1.5">
        {filteredProposals.length === 0 ? (
          <div className="p-6 text-center text-xs text-[var(--color-fg-muted)] space-y-2">
            <Layers size={24} className="mx-auto opacity-30 text-[var(--color-fg-subtle)]" />
            <p>No synthesis proposals found.</p>
            <button
              onClick={onStartNewProposal}
              className="text-[11px] text-[var(--workbench-accent)] hover:underline inline-flex items-center gap-1"
            >
              <Plus size={11} /> Create first synthesis proposal
            </button>
          </div>
        ) : (
          filteredProposals.map((prop) => {
            const isSelected = !isComposing && selectedProposalId === prop.id;
            const verifiedClaims = prop.claims.filter(
              (c) => c.level === 'L2_VERIFIED' || c.level === 'L3_SEALED'
            ).length;

            return (
              <div
                key={prop.id}
                onClick={() => onSelectProposal(prop)}
                className={`p-3 rounded-xl border text-left cursor-pointer transition ${
                  isSelected
                    ? 'bg-[var(--color-surface-2)] border-[var(--workbench-accent)] shadow-xs'
                    : 'bg-[var(--color-surface-1)] border-[var(--color-border-muted)] hover:bg-[var(--color-surface-2)]/60 hover:border-[var(--color-border-subtle)]'
                }`}
              >
                <div className="flex items-center justify-between gap-2 mb-1.5">
                  <span className="font-semibold text-xs text-[var(--color-editor-fg)] truncate">
                    {prop.title}
                  </span>
                  {renderStatusBadge(prop.status)}
                </div>
                <p className="text-[11px] text-[var(--color-fg-muted)] line-clamp-2 leading-relaxed mb-2">
                  {prop.summary || 'No summary specified.'}
                </p>
                <div className="flex items-center justify-between text-[10px] text-[var(--color-fg-subtle)] pt-1.5 border-t border-[var(--color-border-muted)]">
                  <div className="flex items-center gap-2">
                    <span className="flex items-center gap-1">
                      <ShieldCheck size={11} className="text-emerald-400" />
                      {verifiedClaims}/{prop.claims.length} claims
                    </span>
                    <span>•</span>
                    <span className="flex items-center gap-1">
                      <FlaskConical size={11} />
                      {prop.recipes.length} recipes
                    </span>
                  </div>
                  <span className="font-mono text-[9px]">
                    {new Date(prop.updated_at || prop.created_at).toLocaleDateString()}
                  </span>
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* Ledger Footer Metric */}
      <div className="p-2.5 border-t border-[var(--color-border-muted)] bg-[var(--color-surface-1)] text-[10px] text-[var(--color-fg-subtle)] flex items-center justify-between">
        <span>Total Proposals: {proposals.length}</span>
        <span className="font-mono text-[9px] text-white/60">Daemon Registry Online</span>
      </div>
    </aside>
  );
};
