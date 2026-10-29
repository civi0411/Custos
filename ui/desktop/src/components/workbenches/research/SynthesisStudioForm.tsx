import React from 'react';
import {
  Code2,
  GitBranch,
  ShieldCheck,
  Search,
  Lock,
  FileCheck,
  FlaskConical,
  AlertTriangle,
  CheckCircle2,
  AlertCircle,
  ShieldAlert,
  ArrowRight,
} from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import type {
  ResearchSynthesisProposal,
  ExecutionWorkspace,
} from '@/types/domain';
import type {
  ResearchClaim,
  ResearchRecipe,
  ClaimGroundingLevel,
} from '@/types/research';

export interface SynthesisStudioFormProps {
  isComposing: boolean;
  activeProposal: ResearchSynthesisProposal | null;
  formTitle: string;
  onFormTitleChange: (v: string) => void;
  formSummary: string;
  onFormSummaryChange: (v: string) => void;
  selectedWorkspaceId: string;
  onSelectedWorkspaceIdChange: (v: string) => void;
  targetBranch: string;
  onTargetBranchChange: (v: string) => void;
  workspaces: ExecutionWorkspace[];
  claims: ResearchClaim[];
  selectedClaimIds: string[];
  onToggleClaim: (id: string) => void;
  claimSearch: string;
  onClaimSearchChange: (v: string) => void;
  recipes: ResearchRecipe[];
  selectedRecipeIds: string[];
  onToggleRecipe: (id: string) => void;
  recipeSearch: string;
  onRecipeSearchChange: (v: string) => void;
  computedLiveCaveats: string[];
  enforceVerification: boolean;
  onEnforceVerificationChange: (v: boolean) => void;
  verificationViolations: ResearchClaim[];
  executing: boolean;
  onExecuteHandoff: () => void;
}

const renderGroundingBadge = (level: ClaimGroundingLevel | string) => {
  switch (level) {
    case 'L3_SEALED':
      return (
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/[0.08] text-white border border-white/15 font-mono text-[10px] font-semibold">
          <Lock size={10} /> L3 SEALED
        </span>
      );
    case 'L2_VERIFIED':
      return (
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/[0.06] text-white/90 border border-white/10 font-mono text-[10px] font-semibold">
          <CheckCircle2 size={10} /> L2 VERIFIED
        </span>
      );
    case 'L1_CITED':
      return (
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/[0.04] text-white/70 border border-white/10 font-mono text-[10px] font-semibold">
          <FileCheck size={10} /> L1 CITED
        </span>
      );
    case 'L0_UNGROUNDED':
    default:
      return (
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/[0.02] text-white/40 border border-white/5 font-mono text-[10px] font-semibold">
          <AlertCircle size={10} /> L0 UNGROUNDED
        </span>
      );
  }
};

export const SynthesisStudioForm: React.FC<SynthesisStudioFormProps> = ({
  isComposing,
  activeProposal,
  formTitle,
  onFormTitleChange,
  formSummary,
  onFormSummaryChange,
  selectedWorkspaceId,
  onSelectedWorkspaceIdChange,
  targetBranch,
  onTargetBranchChange,
  workspaces,
  claims,
  selectedClaimIds,
  onToggleClaim,
  claimSearch,
  onClaimSearchChange,
  recipes,
  selectedRecipeIds,
  onToggleRecipe,
  recipeSearch,
  onRecipeSearchChange,
  computedLiveCaveats,
  enforceVerification,
  onEnforceVerificationChange,
  verificationViolations,
  executing,
  onExecuteHandoff,
}) => {
  const isReadOnly = !isComposing && activeProposal?.status === 'handoff_completed';

  return (
    <div className="space-y-6">
      {/* Section 1: Objective & Scope */}
      <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-4">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
          <Code2 size={13} />
          <span>Synthesis Objective & Target Worktree</span>
        </div>

        <div className="space-y-3">
          <div>
            <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
              Proposal Title
            </label>
            <input
              type="text"
              disabled={isReadOnly}
              value={formTitle}
              onChange={(e) => onFormTitleChange(e.target.value)}
              placeholder="e.g. Implement Genomic Pipeline Transformer from Verified L2 Claims"
              className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-3 py-1.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60"
            />
          </div>

          <div>
            <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
              Summary & Scope
            </label>
            <textarea
              rows={3}
              disabled={isReadOnly}
              value={formSummary}
              onChange={(e) => onFormSummaryChange(e.target.value)}
              placeholder="Provide architecture rationale, invariants to enforce, and handoff goals..."
              className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-3 py-2 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60 resize-y"
            />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
            <div>
              <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                Execution Workspace / Worktree
              </label>
              <NativeSelect
                disabled={isReadOnly}
                value={selectedWorkspaceId}
                onChange={(e) => onSelectedWorkspaceIdChange(e.target.value)}
                className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-3 py-1.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60"
              >
                <option value="">Choose a Git workspace</option>
                {workspaces
                  .filter((ws) => ws.kind.type === 'git' && ws.status === 'ready')
                  .map((ws) => (
                    <option key={ws.id} value={ws.id}>
                      {ws.name} ({ws.kind.type}) - {ws.path}
                    </option>
                  ))}
              </NativeSelect>
            </div>

            <div>
              <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                Target Git Branch
              </label>
              <div className="relative">
                <GitBranch
                  size={13}
                  className="absolute left-2.5 top-2.5 text-[var(--color-fg-muted)]"
                />
                <input
                  type="text"
                  disabled={isReadOnly}
                  value={targetBranch}
                  onChange={(e) => onTargetBranchChange(e.target.value)}
                  placeholder="synthesis/handoff-v1"
                  className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg pl-8 pr-3 py-1.5 text-xs font-mono text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60"
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Section 2: Empirical Claims Selection */}
      <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
            <ShieldCheck size={13} />
            <span>Grounding Claims Matrix ({selectedClaimIds.length} Selected)</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="relative">
              <Search
                size={11}
                className="absolute left-2 top-1.5 text-[var(--color-fg-muted)]"
              />
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

      {/* Section 3: Reproduction Recipes Selection */}
      <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
            <FlaskConical size={13} />
            <span>Reproduction Recipes ({selectedRecipeIds.length} Selected)</span>
          </div>
          <div className="relative">
            <Search
              size={11}
              className="absolute left-2 top-1.5 text-[var(--color-fg-muted)]"
            />
            <input
              type="text"
              placeholder="Filter recipes..."
              value={recipeSearch}
              onChange={(e) => onRecipeSearchChange(e.target.value)}
              className="bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded pl-6 pr-2 py-0.5 text-[11px] text-[var(--color-editor-fg)] focus:outline-none"
            />
          </div>
        </div>

        <div className="max-h-56 overflow-y-auto space-y-2 pr-1">
          {recipes.length === 0 ? (
            <p className="text-xs text-[var(--color-fg-muted)] py-3 text-center">
              No reproduction recipes saved in research repository.
            </p>
          ) : (
            recipes
              .filter(
                (r) =>
                  recipeSearch === '' ||
                  r.name.toLowerCase().includes(recipeSearch.toLowerCase()) ||
                  r.command.toLowerCase().includes(recipeSearch.toLowerCase())
              )
              .map((recipe) => {
                const isChecked = selectedRecipeIds.includes(recipe.id);

                return (
                  <div
                    key={recipe.id}
                    onClick={() => {
                      if (!isReadOnly) {
                        onToggleRecipe(recipe.id);
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
                      onChange={() => onToggleRecipe(recipe.id)}
                      disabled={isReadOnly}
                      className="mt-0.5 rounded border-[var(--color-border-default)] accent-[var(--workbench-accent)] cursor-pointer"
                    />
                    <div className="flex-1 space-y-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-semibold text-xs text-[var(--color-editor-fg)]">
                          {recipe.name}
                        </span>
                        <span className="font-mono text-[10px] text-[var(--color-fg-subtle)]">
                          {recipe.inputs?.length || 0} inputs •{' '}
                          {recipe.outputs?.length || 0} outputs
                        </span>
                      </div>
                      <div className="p-2 rounded bg-[var(--color-surface-3)] font-mono text-[11px] text-[var(--color-editor-fg)]">
                        <code>{recipe.command}</code>
                      </div>
                    </div>
                  </div>
                );
              })
          )}
        </div>
      </div>

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
            <span>
              All selected claims satisfy Gate 4 verification preconditions. Zero grounding
              caveats detected.
            </span>
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
              <span>
                Enforce Verification Pre-conditions (Gate 4 Fail-Closed Posture)
              </span>
            </div>
            <p className="text-[10px] text-[var(--color-fg-muted)] mt-0.5">
              Requires reproducible verifier records (L2+) or cryptographic proof
              anchors.
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
              <span className="font-semibold">Blocked:</span>{' '}
              {verificationViolations.length} claim(s) lack required L2+ verifier
              records.
            </div>
          </div>
        )}

        <div className="flex items-center justify-between pt-2 border-t border-[var(--color-border-muted)]">
          <div className="text-[11px] text-[var(--color-fg-subtle)]">
            Output: Immutable Task Contracts with{' '}
            <code className="font-mono text-[10px] text-zinc-300">Diff</code> and{' '}
            <code className="font-mono text-[10px] text-zinc-300">TestResult</code>{' '}
            evidence requirements.
          </div>
          <button
            onClick={onExecuteHandoff}
            disabled={
              executing ||
              (enforceVerification && verificationViolations.length > 0)
            }
            className="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold transition flex items-center gap-2 shadow-xs disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <ArrowRight size={14} className={executing ? 'animate-pulse' : ''} />
            <span>Execute Handoff to Coding</span>
          </button>
        </div>
      </div>
    </div>
  );
};
