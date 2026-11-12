import React, { useState, useEffect, useCallback, useMemo } from 'react';
import {
  Layers,
  RefreshCw,
  Plus,
  ArrowRight,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type {
  ResearchSynthesisProposal,
  SaveSynthesisProposalParams,
  HandoffToCodingParams,
  HandoffToCodingResult,
  ExecutionWorkspace,
} from '@/types/domain';
import type { ResearchClaim, ResearchRecipe } from '@/types/research';
import type { ResearchSynthesisPaneProps } from './types';
import { renderStatusBadge } from './badges';
import { ProposalsSidebar } from './ProposalsSidebar';
import { HandoffReceiptBanner } from './HandoffReceiptBanner';
import { SynthesisStudioForm } from './SynthesisStudioForm';
import { ClaimsSelectorSection } from './ClaimsSelectorSection';
import { RecipesSelectorSection } from './RecipesSelectorSection';
import { CaveatsAndGateSection } from './CaveatsAndGateSection';

export const ResearchSynthesisView: React.FC<ResearchSynthesisPaneProps> = ({
  onShowToast,
  onNavigateToTask,
  initialWorkspaceId,
  initialClaimId,
}) => {
  // Data states
  const [proposals, setProposals] = useState<ResearchSynthesisProposal[]>([]);
  const [selectedProposalId, setSelectedProposalId] = useState<string | null>(null);
  const [claims, setClaims] = useState<ResearchClaim[]>([]);
  const [recipes, setRecipes] = useState<ResearchRecipe[]>([]);
  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState<boolean>(false);
  const [executing, setExecuting] = useState<boolean>(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  // Filters & Studio state
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [claimSearch, setClaimSearch] = useState<string>('');
  const [recipeSearch, setRecipeSearch] = useState<string>('');

  // Form State for Synthesis Studio
  const [isComposing, setIsComposing] = useState<boolean>(false);
  const [formTitle, setFormTitle] = useState<string>('');
  const [formSummary, setFormSummary] = useState<string>('');
  const [selectedClaimIds, setSelectedClaimIds] = useState<string[]>([]);
  const [selectedRecipeIds, setSelectedRecipeIds] = useState<string[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>(initialWorkspaceId || '');
  const [targetBranch, setTargetBranch] = useState<string>('synthesis/handoff-v1');
  const [enforceVerification, setEnforceVerification] = useState<boolean>(true);

  // Last handoff receipt modal/card
  const [lastHandoffResult, setLastHandoffResult] = useState<HandoffToCodingResult | null>(null);

  useEffect(() => {
    if (!initialClaimId) return;
    setSelectedProposalId(null);
    setIsComposing(true);
    setSelectedClaimIds((prev) => (prev.includes(initialClaimId) ? prev : [...prev, initialClaimId]));
    setLastHandoffResult(null);
  }, [initialClaimId]);

  // Load all foundational data from daemon API
  const loadData = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const [proposalsList, claimsList, recipesList, workspacesList] = await Promise.all([
        daemonClient.listSynthesisProposals(),
        daemonClient.listResearchClaims(),
        daemonClient.listResearchRecipes(),
        daemonClient.listWorkspaces(),
      ]);

      setProposals(proposalsList);
      setClaims(claimsList);
      setRecipes(recipesList);
      setWorkspaces(workspacesList);

      if (!selectedProposalId && proposalsList.length > 0) {
        setIsComposing(true);
      } else if (proposalsList.length === 0) {
        setIsComposing(true);
      }
    } catch (err) {
      console.error('Failed to load synthesis workbench data:', err);
      setLoadError(err instanceof Error ? err.message : String(err));
      onShowToast?.(`Failed to load synthesis workbench data: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  }, [onShowToast, selectedProposalId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Selected Proposal
  const activeProposal = useMemo(() => {
    return proposals.find((p) => p.id === selectedProposalId) || null;
  }, [proposals, selectedProposalId]);

  // Filtered proposals list
  const filteredProposals = useMemo(() => {
    return proposals.filter((p) => {
      const matchesSearch =
        searchQuery === '' ||
        p.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        p.summary.toLowerCase().includes(searchQuery.toLowerCase()) ||
        p.id.toLowerCase().includes(searchQuery.toLowerCase());
      const matchesStatus = statusFilter === 'all' || p.status === statusFilter;
      return matchesSearch && matchesStatus;
    });
  }, [proposals, searchQuery, statusFilter]);

  const handleSelectProposal = (p: ResearchSynthesisProposal) => {
    setSelectedProposalId(p.id);
    setIsComposing(false);
    setFormTitle(p.title);
    setFormSummary(p.summary);
    setSelectedClaimIds(p.claims.map((c) => c.claim_id || c.claimId || ''));
    setSelectedRecipeIds(p.recipes.map((r) => r.recipe_id || r.recipeId || ''));
    setSelectedWorkspaceId(p.workspace_id || p.workspaceId || '');
    setTargetBranch(p.target_branch || p.targetBranch || 'synthesis/handoff-v1');
    setLastHandoffResult(null);
  };

  const handleStartNewProposal = () => {
    setSelectedProposalId(null);
    setIsComposing(true);
    setFormTitle('');
    setFormSummary('');
    setSelectedClaimIds([]);
    setSelectedRecipeIds([]);
    setTargetBranch('synthesis/handoff-v1');
    setLastHandoffResult(null);
  };

  const toggleClaim = (id: string) => {
    setSelectedClaimIds((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]
    );
  };

  const toggleRecipe = (id: string) => {
    setSelectedRecipeIds((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]
    );
  };

  const copyText = (text: string, idKey: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(idKey);
    setTimeout(() => setCopiedId(null), 2000);
    onShowToast?.(`Copied to clipboard: ${text}`);
  };

  const computedLiveCaveats = useMemo(() => {
    const caveats: string[] = [];
    const selectedClaimsList = claims.filter((c) => selectedClaimIds.includes(c.id));
    const ungrounded = selectedClaimsList.filter((c) => c.level === 'L0_UNGROUNDED');
    const citedOnly = selectedClaimsList.filter((c) => c.level === 'L1_CITED');

    if (ungrounded.length > 0) {
      caveats.push(
        `${ungrounded.length} ungrounded (L0) claim(s) included without empirical citations or invariant evidence.`
      );
    }
    if (citedOnly.length > 0) {
      caveats.push(
        `${citedOnly.length} claim(s) possess literature citations (L1) but lack reproducible verifier receipts.`
      );
    }
    if (selectedClaimsList.length > 0 && selectedRecipeIds.length === 0) {
      caveats.push(
        'Empirical claims selected without reproduction recipes; implementation will rely entirely on static specifications.'
      );
    }
    if (!selectedWorkspaceId) {
      caveats.push('Select a registered Git workspace before handing off to Coding.');
    }
    return caveats;
  }, [claims, selectedClaimIds, selectedRecipeIds, selectedWorkspaceId]);

  const verificationViolations = useMemo(() => {
    if (!enforceVerification) return [];
    const selectedClaimsList = claims.filter((c) => selectedClaimIds.includes(c.id));
    return selectedClaimsList.filter(
      (c) => c.level === 'L0_UNGROUNDED' && !c.sealedProofUri
    );
  }, [enforceVerification, claims, selectedClaimIds]);

  const handleSaveProposal = async () => {
    if (!formTitle.trim()) {
      onShowToast?.('Proposal title is required.');
      return;
    }
    setSaving(true);
    try {
      const params: SaveSynthesisProposalParams = {
        id: selectedProposalId || undefined,
        title: formTitle.trim(),
        summary: formSummary.trim(),
        claim_ids: selectedClaimIds,
        recipe_ids: selectedRecipeIds,
        workspace_id: selectedWorkspaceId || undefined,
        target_branch: targetBranch.trim() || undefined,
      };

      const saved = await daemonClient.saveSynthesisProposal(params);
      onShowToast?.(`Proposal "${saved.title}" saved successfully.`);
      await loadData();
      setSelectedProposalId(saved.id);
      setIsComposing(false);
    } catch (err) {
      console.error('Failed to save synthesis proposal:', err);
      onShowToast?.(`Save failed: ${String(err)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleExecuteHandoff = async () => {
    if (selectedClaimIds.length === 0 && selectedRecipeIds.length === 0) {
      onShowToast?.('Select at least one claim or reproduction recipe for coding handoff.');
      return;
    }

    if (enforceVerification && verificationViolations.length > 0) {
      onShowToast?.(
        `Gate 4 Fail-Closed Block: ${verificationViolations.length} claim(s) lack L2+ grounding or verifier proof.`
      );
      return;
    }

    if (executing || saving) return;
    const target = workspaces.find((w) => w.id === selectedWorkspaceId);
    if (!target || target.kind.type !== 'git' || target.status !== 'ready') {
      onShowToast?.('Choose a ready Git workspace for the Coding handoff.');
      return;
    }
    setExecuting(true);
    try {
      const params: HandoffToCodingParams = {
        proposal_id: selectedProposalId || undefined,
        title: formTitle.trim() || activeProposal?.title || 'Research Synthesis Handoff',
        claim_ids: selectedClaimIds,
        recipe_ids: selectedRecipeIds,
        workspace_id: selectedWorkspaceId || undefined,
        target_branch: targetBranch.trim() || undefined,
        enforce_verification: enforceVerification,
      };

      const result = await daemonClient.executeSynthesisHandoff(params);
      setLastHandoffResult(result);
      onShowToast?.(`Coding handoff created ${result.task_ids.length} Task(s). Open a Task to continue.`);
      await loadData();
      if (result.proposal_id) {
        setSelectedProposalId(result.proposal_id);
      }
    } catch (err) {
      console.error('Handoff execution failed:', err);
      onShowToast?.(`Handoff rejected: ${String(err)}`);
    } finally {
      setExecuting(false);
    }
  };

  return (
    <div className="flex flex-col h-full bg-[var(--color-surface-0)] text-[var(--color-editor-fg)] overflow-hidden font-sans select-none">
      {/* Top Header Toolbar */}
      <header className="wb-pane-toolbar h-12 border-b border-[var(--color-border-muted)] px-4 flex items-center justify-between bg-[var(--color-surface-1)] shrink-0">
        <div className="flex items-center gap-3">
          <div className="p-1.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] text-[var(--workbench-accent)]">
            <Layers size={16} />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-xs font-semibold tracking-wide text-[var(--color-editor-fg)]">
                Research handoff
              </h2>
              <span className="px-2 py-0.2 rounded-full text-[9px] font-mono font-medium uppercase tracking-wider border border-white/10 text-white/70 bg-white/[0.04]">
                Review before implementation
              </span>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={loadData}
            disabled={loading}
            className="p-1.5 rounded border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] transition disabled:opacity-50"
            title="Refresh Proposals & Claims"
          >
            <RefreshCw size={13} className={loading ? 'animate-spin' : ''} />
          </button>
          <button
            onClick={handleStartNewProposal}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-[var(--workbench-accent)] text-white hover:opacity-90 transition text-xs font-medium shadow-xs"
          >
            <Plus size={13} />
            <span>New Proposal</span>
          </button>
        </div>
      </header>

      {loadError && (
        <div role="alert" className="border-b border-rose-500/30 px-4 py-2 text-xs text-rose-500">
          Research data unavailable: {loadError}
        </div>
      )}

      {/* Main Two-Column Layout */}
      <div className="wb-split flex flex-1 overflow-hidden">
        <ProposalsSidebar
          proposals={proposals}
          filteredProposals={filteredProposals}
          selectedProposalId={selectedProposalId}
          isComposing={isComposing}
          searchQuery={searchQuery}
          statusFilter={statusFilter}
          onSearchChange={setSearchQuery}
          onStatusFilterChange={setStatusFilter}
          onSelectProposal={handleSelectProposal}
          onStartNewProposal={handleStartNewProposal}
        />

        {/* Right Column: Interactive Synthesis Studio */}
        <main className="flex-1 overflow-y-auto flex flex-col bg-[var(--color-surface-0)]">
          <HandoffReceiptBanner
            lastHandoffResult={lastHandoffResult}
            copiedId={copiedId}
            onCopyText={copyText}
            onNavigateToTask={onNavigateToTask}
          />

          <div className="p-6 space-y-6 max-w-4xl w-full mx-auto select-text">
            {/* Top Overview & Action Bar */}
            <div className="flex items-start justify-between gap-4 pb-4 border-b border-[var(--color-border-muted)]">
              <div>
                <div className="flex items-center gap-2">
                  <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">
                    {isComposing
                      ? 'Compose Research Synthesis Proposal'
                      : activeProposal?.title || 'Proposal Overview'}
                  </h3>
                  {activeProposal && !isComposing && renderStatusBadge(activeProposal.status)}
                </div>
                <p className="text-xs text-[var(--color-fg-muted)] mt-1">
                  {isComposing
                    ? 'Aggregate empirical claims and reproduction recipes into a verifiable engineering handoff package.'
                    : `Proposal ID: ${activeProposal?.id} • Last updated ${new Date(activeProposal?.updated_at || Date.now()).toLocaleString()}`}
                </p>
              </div>

              <div className="flex items-center gap-2">
                {!isComposing && activeProposal && activeProposal.status !== 'handoff_completed' && (
                  <button
                    onClick={() => setIsComposing(true)}
                    className="px-3 py-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-xs font-medium transition"
                  >
                    Edit Draft
                  </button>
                )}
                {isComposing && (
                  <button
                    onClick={handleSaveProposal}
                    disabled={saving}
                    className="px-3 py-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-xs font-medium transition flex items-center gap-1.5 disabled:opacity-50"
                  >
                    <RefreshCw size={12} className={saving ? 'animate-spin' : ''} />
                    <span>Save Draft</span>
                  </button>
                )}
                <button
                  onClick={handleExecuteHandoff}
                  disabled={executing || (enforceVerification && verificationViolations.length > 0)}
                  className="px-3.5 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium transition flex items-center gap-2 shadow-xs disabled:opacity-40 disabled:cursor-not-allowed"
                  title="Execute fail-closed handoff to create Invariant Coding tasks in daemon"
                >
                  <ArrowRight size={13} className={executing ? 'animate-pulse' : ''} />
                  <span>Execute Handoff to Coding</span>
                </button>
              </div>
            </div>

            <SynthesisStudioForm
              isComposing={isComposing}
              activeProposalStatus={activeProposal?.status}
              formTitle={formTitle}
              formSummary={formSummary}
              selectedWorkspaceId={selectedWorkspaceId}
              targetBranch={targetBranch}
              workspaces={workspaces}
              onTitleChange={setFormTitle}
              onSummaryChange={setFormSummary}
              onWorkspaceChange={setSelectedWorkspaceId}
              onBranchChange={setTargetBranch}
            />

            <ClaimsSelectorSection
              claims={claims}
              selectedClaimIds={selectedClaimIds}
              claimSearch={claimSearch}
              isComposing={isComposing}
              activeProposalStatus={activeProposal?.status}
              onClaimSearchChange={setClaimSearch}
              onToggleClaim={toggleClaim}
            />

            <RecipesSelectorSection
              recipes={recipes}
              selectedRecipeIds={selectedRecipeIds}
              recipeSearch={recipeSearch}
              isComposing={isComposing}
              activeProposalStatus={activeProposal?.status}
              onRecipeSearchChange={setRecipeSearch}
              onToggleRecipe={toggleRecipe}
            />

            <CaveatsAndGateSection
              computedLiveCaveats={computedLiveCaveats}
              enforceVerification={enforceVerification}
              verificationViolations={verificationViolations}
              executing={executing}
              onEnforceVerificationChange={setEnforceVerification}
              onExecuteHandoff={handleExecuteHandoff}
            />
          </div>
        </main>
      </div>
    </div>
  );
};
