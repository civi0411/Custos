import React, { useState, useEffect, useCallback, useMemo } from 'react';
import {
  Layers,
  ShieldCheck,
  ShieldAlert,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Clock,
  RefreshCw,
  Search,
  Plus,
  GitBranch,
  ArrowRight,
  Code2,
  Copy,
  Check,
  Lock,
  FileCheck,
  AlertCircle,
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
  ExecutionWorkspace,
} from '@/types/domain';
import type { ResearchClaim, ResearchRecipe, ClaimGroundingLevel } from '@/types/research';

interface ResearchSynthesisPaneProps {
  onShowToast?: (msg: string) => void;
  onNavigateToTask?: (taskId: string) => void;
}

export const ResearchSynthesisPane: React.FC<ResearchSynthesisPaneProps> = ({
  onShowToast,
  onNavigateToTask,
}) => {
  // Data states
  const [proposals, setProposals] = useState<ResearchSynthesisProposal[]>([]);
  const [selectedProposalId, setSelectedProposalId] = useState<string | null>(null);
  const [claims, setClaims] = useState<ResearchClaim[]>([]);
  const [recipes, setRecipes] = useState<ResearchRecipe[]>([]);
  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
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
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>('');
  const [targetBranch, setTargetBranch] = useState<string>('synthesis/handoff-v1');
  const [enforceVerification, setEnforceVerification] = useState<boolean>(true);

  // Last handoff receipt modal/card
  const [lastHandoffResult, setLastHandoffResult] = useState<HandoffToCodingResult | null>(null);

  // Load all foundational data from daemon API
  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [proposalsList, claimsList, recipesList, workspacesList] = await Promise.all([
        daemonClient.listSynthesisProposals().catch(() => []),
        daemonClient.listResearchClaims().catch(() => []),
        daemonClient.listResearchRecipes().catch(() => []),
        daemonClient.listWorkspaces().catch(() => []),
      ]);

      setProposals(proposalsList);
      setClaims(claimsList);
      setRecipes(recipesList);
      setWorkspaces(workspacesList);

      if (workspacesList.length > 0 && !selectedWorkspaceId) {
        setSelectedWorkspaceId(workspacesList[0].id);
      }

      // If nothing selected, select first proposal or open composer if empty
      if (!selectedProposalId && proposalsList.length > 0) {
        setSelectedProposalId(proposalsList[0].id);
        setIsComposing(false);
      } else if (proposalsList.length === 0) {
        setIsComposing(true);
      }
    } catch (err) {
      console.error('Failed to load synthesis workbench data:', err);
      onShowToast?.(`Failed to load synthesis workbench data: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  }, [selectedProposalId, selectedWorkspaceId, onShowToast]);

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
    // Pre-select any verified claims by default for developer convenience
    const verifiedIds = claims
      .filter((c) => c.level === 'L2_VERIFIED' || c.level === 'L3_SEALED')
      .map((c) => c.id);
    setSelectedClaimIds(verifiedIds);
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
      caveats.push('No Git execution workspace target designated; default workspace will be bound.');
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
      onShowToast?.(
        `Coding Handoff created ${result.task_ids.length} Invariant Task(s) in workspace.`
      );
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

  const renderGroundingBadge = (level: ClaimGroundingLevel | string) => {
    switch (level) {
      case 'L3_SEALED':
        return (
          <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-purple-500/10 text-purple-400 border border-purple-500/20 font-mono text-[10px] font-semibold">
            <Lock size={10} /> L3 SEALED
          </span>
        );
      case 'L2_VERIFIED':
        return (
          <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono text-[10px] font-semibold">
            <CheckCircle2 size={10} /> L2 VERIFIED
          </span>
        );
      case 'L1_CITED':
        return (
          <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20 font-mono text-[10px] font-semibold">
            <FileCheck size={10} /> L1 CITED
          </span>
        );
      case 'L0_UNGROUNDED':
      default:
        return (
          <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-rose-500/10 text-rose-400 border border-rose-500/20 font-mono text-[10px] font-semibold">
            <AlertCircle size={10} /> L0 UNGROUNDED
          </span>
        );
    }
  };

  const renderStatusBadge = (status: SynthesisProposalStatus) => {
    switch (status) {
      case 'handoff_completed':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-[10px] font-semibold uppercase tracking-wider">
            <CheckCircle2 size={10} /> Completed
          </span>
        );
      case 'submitted':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 text-[10px] font-semibold uppercase tracking-wider">
            <ShieldCheck size={10} /> Submitted
          </span>
        );
      case 'rejected':
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-rose-500/10 text-rose-400 border border-rose-500/20 text-[10px] font-semibold uppercase tracking-wider">
            <XCircle size={10} /> Rejected
          </span>
        );
      case 'draft':
      default:
        return (
          <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-400 border border-amber-500/20 text-[10px] font-semibold uppercase tracking-wider">
            <Clock size={10} /> Draft
          </span>
        );
    }
  };

  return (
    <div className="flex flex-col h-full bg-[var(--color-surface-0)] text-[var(--color-editor-fg)] overflow-hidden font-sans select-none">
      {/* ─── Top Header Toolbar ─── */}
      <header className="h-12 border-b border-[var(--color-border-muted)] px-4 flex items-center justify-between bg-[var(--color-surface-1)] shrink-0">
        <div className="flex items-center gap-3">
          <div className="p-1.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] text-[var(--workbench-accent)]">
            <Layers size={16} />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-xs font-semibold tracking-wide text-[var(--color-editor-fg)]">
                Research Synthesis & Coding Handoff
              </h2>
              <span className="px-2 py-0.2 rounded-full text-[9px] font-mono font-medium uppercase tracking-wider border border-emerald-500/30 text-emerald-400 bg-emerald-500/10">
                Gate 4 Fail-Closed
              </span>
            </div>
            <p className="text-[10px] text-[var(--color-fg-muted)]">
              Synthesize empirical claims, verified recipes, and artifact lineage into invariant-governed Coding tasks.
            </p>
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

      {/* ─── Main Two-Column Layout ─── */}
      <div className="flex flex-1 overflow-hidden">
        {/* ── Left Column: Proposals Ledger ── */}
        <aside className="w-80 border-r border-[var(--color-border-muted)] flex flex-col bg-[var(--color-surface-1)]/50 shrink-0">
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
            <span className="font-mono text-[9px] text-emerald-400">Daemon Registry Online</span>
          </div>
        </aside>

        {/* ── Right Column: Interactive Synthesis Studio ── */}
        <main className="flex-1 overflow-y-auto flex flex-col bg-[var(--color-surface-0)]">
          {/* Banner for completed handoff if just performed */}
          {lastHandoffResult && (
            <div className="m-4 p-4 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-300 space-y-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 font-semibold text-xs text-emerald-300">
                  <CheckCircle2 size={16} className="text-emerald-400" />
                  Coding Handoff Dispatched Successfully
                </div>
                <span className="font-mono text-[10px] bg-emerald-500/20 px-2 py-0.5 rounded text-emerald-300 border border-emerald-500/30">
                  Handoff ID: {lastHandoffResult.handoff_id}
                </span>
              </div>
              <p className="text-xs text-emerald-400/90 leading-relaxed">
                Created {lastHandoffResult.task_ids.length} invariant-governed task(s) in workspace.
                Target branch <code className="font-mono px-1 py-0.5 rounded bg-emerald-950/60">{lastHandoffResult.target_branch || 'synthesis/handoff-v1'}</code> bound to safe worktree.
              </p>
              <div className="flex flex-wrap gap-1.5 pt-1">
                {lastHandoffResult.task_ids.map((tid) => (
                  <span
                    key={tid}
                    className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-[var(--color-surface-1)] border border-emerald-500/30 text-xs font-mono text-[var(--color-editor-fg)]"
                  >
                    <Code2 size={12} className="text-emerald-400" />
                    <span>{tid}</span>
                    <button
                      onClick={() => copyText(tid, tid)}
                      className="text-[var(--color-fg-muted)] hover:text-white"
                      title="Copy Task ID"
                    >
                      {copiedId === tid ? <Check size={11} className="text-emerald-400" /> : <Copy size={11} />}
                    </button>
                    {onNavigateToTask && (
                      <button
                        onClick={() => onNavigateToTask(tid)}
                        className="text-emerald-400 hover:text-emerald-300 ml-1"
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
                    disabled={!isComposing && activeProposal?.status === 'handoff_completed'}
                    value={formTitle}
                    onChange={(e) => setFormTitle(e.target.value)}
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
                    disabled={!isComposing && activeProposal?.status === 'handoff_completed'}
                    value={formSummary}
                    onChange={(e) => setFormSummary(e.target.value)}
                    placeholder="Provide architecture rationale, invariants to enforce, and handoff goals..."
                    className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-3 py-2 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60 resize-y"
                  />
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
                  <div>
                    <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                      Execution Workspace / Worktree
                    </label>
                    <select
                      disabled={!isComposing && activeProposal?.status === 'handoff_completed'}
                      value={selectedWorkspaceId}
                      onChange={(e) => setSelectedWorkspaceId(e.target.value)}
                      className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-3 py-1.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60"
                    >
                      <option value="">Default Workspace (Automatic Assignment)</option>
                      {workspaces.map((ws) => (
                        <option key={ws.id} value={ws.id}>
                          {ws.name} ({ws.kind.type}) - {ws.path}
                        </option>
                      ))}
                    </select>
                  </div>

                  <div>
                    <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                      Target Git Branch
                    </label>
                    <div className="relative">
                      <GitBranch size={13} className="absolute left-2.5 top-2.5 text-[var(--color-fg-muted)]" />
                      <input
                        type="text"
                        disabled={!isComposing && activeProposal?.status === 'handoff_completed'}
                        value={targetBranch}
                        onChange={(e) => setTargetBranch(e.target.value)}
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
                    <Search size={11} className="absolute left-2 top-1.5 text-[var(--color-fg-muted)]" />
                    <input
                      type="text"
                      placeholder="Filter claims..."
                      value={claimSearch}
                      onChange={(e) => setClaimSearch(e.target.value)}
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
                          if (isComposing || activeProposal?.status !== 'handoff_completed') {
                            toggleClaim(claim.id);
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
                          onChange={() => toggleClaim(claim.id)}
                          disabled={!isComposing && activeProposal?.status === 'handoff_completed'}
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
                  <Search size={11} className="absolute left-2 top-1.5 text-[var(--color-fg-muted)]" />
                  <input
                    type="text"
                    placeholder="Filter recipes..."
                    value={recipeSearch}
                    onChange={(e) => setRecipeSearch(e.target.value)}
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
                            if (isComposing || activeProposal?.status !== 'handoff_completed') {
                              toggleRecipe(recipe.id);
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
                            onChange={() => toggleRecipe(recipe.id)}
                            disabled={!isComposing && activeProposal?.status === 'handoff_completed'}
                            className="mt-0.5 rounded border-[var(--color-border-default)] accent-[var(--workbench-accent)] cursor-pointer"
                          />
                          <div className="flex-1 space-y-1">
                            <div className="flex items-center justify-between gap-2">
                              <span className="font-semibold text-xs text-[var(--color-editor-fg)]">
                                {recipe.name}
                              </span>
                              <span className="font-mono text-[10px] text-[var(--color-fg-subtle)]">
                                {recipe.inputs?.length || 0} inputs • {recipe.outputs?.length || 0} outputs
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
                <div className="p-3 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-xs flex items-center gap-2">
                  <CheckCircle2 size={14} />
                  <span>All selected claims satisfy Gate 4 verification preconditions. Zero grounding caveats detected.</span>
                </div>
              ) : (
                <div className="space-y-1.5">
                  {computedLiveCaveats.map((cav, idx) => (
                    <div
                      key={idx}
                      className="p-2.5 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-300 text-xs flex items-start gap-2 leading-relaxed"
                    >
                      <AlertCircle size={13} className="text-amber-400 shrink-0 mt-0.5" />
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
                    <ShieldCheck size={14} className={enforceVerification ? 'text-emerald-400' : 'text-amber-400'} />
                    <span>Enforce Verification Pre-conditions (Gate 4 Fail-Closed Posture)</span>
                  </div>
                  <p className="text-[11px] text-[var(--color-fg-muted)] mt-0.5 leading-relaxed">
                    When active, the daemon API strictly rejects handoff of claims lacking L2+ reproducible verifier records or cryptographic proof anchors.
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer shrink-0">
                  <input
                    type="checkbox"
                    checked={enforceVerification}
                    onChange={(e) => setEnforceVerification(e.target.checked)}
                    className="sr-only peer"
                  />
                  <div className="w-10 h-5 bg-[var(--color-surface-3)] peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-emerald-600"></div>
                </label>
              </div>

              {enforceVerification && verificationViolations.length > 0 && (
                <div className="p-3 rounded-lg bg-rose-500/10 border border-rose-500/25 text-rose-300 text-xs flex items-start gap-2">
                  <ShieldAlert size={14} className="text-rose-400 shrink-0 mt-0.5" />
                  <div>
                    <span className="font-semibold">Handoff Blocked by Fail-Closed Gate:</span> {verificationViolations.length} selected claim(s) are ungrounded (L0) and have no verifier record. Uncheck ungrounded claims or supply verification evidence to proceed.
                  </div>
                </div>
              )}

              <div className="flex items-center justify-between pt-2 border-t border-[var(--color-border-muted)]">
                <div className="text-[11px] text-[var(--color-fg-subtle)]">
                  Output: Immutable Task Contracts with <code className="font-mono text-[10px] text-zinc-300">Diff</code> and <code className="font-mono text-[10px] text-zinc-300">TestResult</code> evidence requirements.
                </div>
                <button
                  onClick={handleExecuteHandoff}
                  disabled={executing || (enforceVerification && verificationViolations.length > 0)}
                  className="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold transition flex items-center gap-2 shadow-xs disabled:opacity-40 disabled:cursor-not-allowed"
                >
                  <ArrowRight size={14} className={executing ? 'animate-pulse' : ''} />
                  <span>Execute Handoff to Coding</span>
                </button>
              </div>
            </div>
          </div>
        </main>
      </div>
    </div>
  );
};
