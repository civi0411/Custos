import React, { useState, useEffect, useMemo } from 'react';
import {
  Layers,
  ShieldCheck,
  CheckCircle2,
  XCircle,
  Clock,
  RefreshCw,
  Search,
  Plus,
  ArrowRight,
  Code2,
  Copy,
  Check,
  ExternalLink,
  FlaskConical,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type {
  ResearchSynthesisProposal,
  SaveSynthesisProposalParams,
  HandoffToCodingParams,
  HandoffToCodingResult,
  SynthesisProposalStatus,
} from '@/types/domain';
import { useHandoffData } from './useHandoffData';
import { SynthesisStudioForm } from './SynthesisStudioForm';

interface ResearchSynthesisPaneProps {
  onShowToast?: (msg: string) => void;
  initialWorkspaceId?: string;
  initialClaimId?: string;
  onNavigateToTask?: (taskId: string, result: HandoffToCodingResult) => void;
}

export const ResearchSynthesisPane: React.FC<ResearchSynthesisPaneProps> = ({
  onShowToast,
  onNavigateToTask,
  initialWorkspaceId,
  initialClaimId,
}) => {
  // Data states
  const {
    proposals,
    claims,
    recipes,
    workspaces,
    loading,
    loadError,
    loadData,
  } = useHandoffData(onShowToast);
  const [selectedProposalId, setSelectedProposalId] = useState<string | null>(null);
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

  useEffect(() => {
    if (!initialClaimId) return;
    setSelectedProposalId(null);
    setIsComposing(true);
    setSelectedClaimIds(previous => previous.includes(initialClaimId) ? previous : [...previous, initialClaimId]);
    setLastHandoffResult(null);
  }, [initialClaimId]);

  useEffect(() => {
    if (!selectedProposalId) {
      setIsComposing(true);
    }
  }, [selectedProposalId]);

  // Last handoff receipt modal/card
  const [lastHandoffResult, setLastHandoffResult] = useState<HandoffToCodingResult | null>(null);

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

  // Switch to viewing an existing proposal
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

  // Start composing a new proposal
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

  // Toggle claim selection in composer
  const toggleClaim = (id: string) => {
    setSelectedClaimIds((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]
    );
  };

  // Toggle recipe selection in composer
  const toggleRecipe = (id: string) => {
    setSelectedRecipeIds((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]
    );
  };

  // Copy helper
  const copyText = (text: string, idKey: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(idKey);
    setTimeout(() => setCopiedId(null), 2000);
    onShowToast?.(`Copied to clipboard: ${text}`);
  };

  // Live caveat calculation
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

  // Check verification posture status
  const verificationViolations = useMemo(() => {
    if (!enforceVerification) return [];
    const selectedClaimsList = claims.filter((c) => selectedClaimIds.includes(c.id));
    return selectedClaimsList.filter(
      (c) => c.level === 'L0_UNGROUNDED' && !c.sealedProofUri
    );
  }, [enforceVerification, claims, selectedClaimIds]);

  // Save proposal
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

  // Execute Handoff to Coding
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
    const target = workspaces.find(workspace => workspace.id === selectedWorkspaceId);
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



  const renderStatusBadge = (status: SynthesisProposalStatus) => {
    switch (status) {
      case 'handoff_completed':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-white/[0.08] text-white border border-white/15 text-[10px] font-semibold uppercase tracking-wider">
            <CheckCircle2 size={10} /> Completed
          </span>
        );
      case 'submitted':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-white/[0.06] text-white/80 border border-white/10 text-[10px] font-semibold uppercase tracking-wider">
            <ShieldCheck size={10} /> Submitted
          </span>
        );
      case 'rejected':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-white/[0.03] text-white/40 border border-white/5 text-[10px] font-semibold uppercase tracking-wider">
            <XCircle size={10} /> Rejected
          </span>
        );
      case 'draft':
      default:
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-white/[0.04] text-white/60 border border-white/10 text-[10px] font-semibold uppercase tracking-wider">
            <Clock size={10} /> Draft
          </span>
        );
    }
  };

  return (
    <div className="flex flex-col h-full bg-[var(--color-surface-0)] text-[var(--color-editor-fg)] overflow-hidden font-sans select-none">
      {/* ─── Top Header Toolbar ─── */}
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

      {loadError && <div role="alert" className="border-b border-rose-500/30 px-4 py-2 text-xs text-rose-500">Research data unavailable: {loadError}</div>}

      {/* ─── Main Two-Column Layout ─── */}
      <div className="wb-split flex flex-1 overflow-hidden">
        {/* ── Left Column: Proposals Ledger ── */}
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
                onChange={(e) => setSearchQuery(e.target.value)}
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
                  onClick={() => setStatusFilter(st)}
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
                  onClick={handleStartNewProposal}
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
                    onClick={() => handleSelectProposal(prop)}
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

        {/* ── Right Column: Interactive Synthesis Studio ── */}
        <main className="flex-1 overflow-y-auto flex flex-col bg-[var(--color-surface-0)]">
          {/* Banner for completed handoff if just performed */}
          {lastHandoffResult && (
            <div className="m-4 p-4 rounded-xl border border-white/15 bg-white/[0.04] text-white/90 space-y-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 font-semibold text-xs text-white">
                  <CheckCircle2 size={16} className="text-white/70" />
                  Coding Handoff Dispatched Successfully
                </div>
                <span className="font-mono text-[10px] bg-white/10 px-2 py-0.5 rounded text-white/80 border border-white/10">
                  Handoff ID: {lastHandoffResult.handoff_id}
                </span>
              </div>
              <p className="text-[11px] text-white/70">
                Created {lastHandoffResult.task_ids.length} task(s) on branch <code className="font-mono px-1 py-0.5 rounded bg-white/10">{lastHandoffResult.target_branch || 'default'}</code>.
              </p>
              <div className="flex flex-wrap gap-1.5 pt-1">
                {lastHandoffResult.task_ids.map((tid) => (
                  <span
                    key={tid}
                    className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-[var(--color-surface-1)] border border-white/10 text-xs font-mono text-[var(--color-editor-fg)]"
                  >
                    <Code2 size={12} className="text-white/60" />
                    <span>{tid}</span>
                    <button
                      onClick={() => copyText(tid, tid)}
                      className="text-[var(--color-fg-muted)] hover:text-white"
                      title="Copy Task ID"
                    >
                      {copiedId === tid ? <Check size={11} className="text-white" /> : <Copy size={11} />}
                    </button>
                    {onNavigateToTask && (
                      <button
                        onClick={() => onNavigateToTask(tid, lastHandoffResult)}
                        className="text-white/70 hover:text-white ml-1"
                        title="View Task in Engineering Workbench"
                      >
                        <ExternalLink size={11} />
                      </button>
                    )}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Form Content */}
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
              activeProposal={activeProposal}
              formTitle={formTitle}
              onFormTitleChange={setFormTitle}
              formSummary={formSummary}
              onFormSummaryChange={setFormSummary}
              selectedWorkspaceId={selectedWorkspaceId}
              onSelectedWorkspaceIdChange={setSelectedWorkspaceId}
              targetBranch={targetBranch}
              onTargetBranchChange={setTargetBranch}
              workspaces={workspaces}
              claims={claims}
              selectedClaimIds={selectedClaimIds}
              onToggleClaim={toggleClaim}
              claimSearch={claimSearch}
              onClaimSearchChange={setClaimSearch}
              recipes={recipes}
              selectedRecipeIds={selectedRecipeIds}
              onToggleRecipe={toggleRecipe}
              recipeSearch={recipeSearch}
              onRecipeSearchChange={setRecipeSearch}
              computedLiveCaveats={computedLiveCaveats}
              enforceVerification={enforceVerification}
              onEnforceVerificationChange={setEnforceVerification}
              verificationViolations={verificationViolations}
              executing={executing}
              onExecuteHandoff={handleExecuteHandoff}
            />
          </div>
        </main>
      </div>
    </div>
  );
};
