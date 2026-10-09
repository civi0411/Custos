import React, { useState, useEffect, useCallback, useMemo } from 'react';
import {
  ShieldCheck,
  ShieldAlert,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Clock,
  RefreshCw,
  Search,
  Plus,
  FileCode,
  GitBranch,
  Terminal,
  Cpu,
  UserCheck,
  Hash,
  AlertCircle,
  Copy,
  Check,
  FileText,
  Trash2,
  CheckSquare,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type {
  ReviewerRecord,
  RecordReviewParams,
  ReviewTargetType,
  ReviewMethod,
  ReviewStatus,
  FindingSeverity,
  ReviewFinding,
} from '@/types/domain';

interface EvidenceWorkbenchPaneProps {
  onShowToast?: (msg: string) => void;
}

const TARGET_TYPES: { label: string; value: ReviewTargetType }[] = [
  { label: 'Artifact', value: 'artifact' },
  { label: 'Diff', value: 'diff' },
  { label: 'Task', value: 'task' },
  { label: 'Run', value: 'run' },
  { label: 'Note', value: 'note' },
  { label: 'Workspace', value: 'workspace' },
];

const REVIEW_METHODS: { label: string; value: ReviewMethod }[] = [
  { label: 'Automated Verifier', value: 'automated_verifier' },
  { label: 'Contract Proof', value: 'contract_proof' },
  { label: 'Model Evaluation', value: 'model_evaluation' },
  { label: 'Peer Review', value: 'peer_review' },
  { label: 'Runtime Inspection', value: 'runtime_inspection' },
];

const REVIEW_STATUSES: { label: string; value: ReviewStatus }[] = [
  { label: 'Approved', value: 'approved' },
  { label: 'Rejected', value: 'rejected' },
  { label: 'Degraded', value: 'degraded' },
  { label: 'Pending', value: 'pending' },
];

const SEVERITIES: { label: string; value: FindingSeverity }[] = [
  { label: 'Info', value: 'info' },
  { label: 'Warning', value: 'warning' },
  { label: 'Error', value: 'error' },
  { label: 'Blocker', value: 'blocker' },
];

export const EvidenceWorkbenchPane: React.FC<EvidenceWorkbenchPaneProps> = ({ onShowToast }) => {
  const [reviews, setReviews] = useState<ReviewerRecord[]>([]);
  const [selectedReviewId, setSelectedReviewId] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [markingStale, setMarkingStale] = useState<boolean>(false);
  const [copiedDigest, setCopiedDigest] = useState<boolean>(false);

  // Filters
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [targetTypeFilter, setTargetTypeFilter] = useState<string>('all');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [methodFilter, setMethodFilter] = useState<string>('all');
  const [freshOnly, setFreshOnly] = useState<boolean>(false);

  // Modal for new review
  const [showSubmitModal, setShowSubmitModal] = useState<boolean>(false);
  const [submitting, setSubmitting] = useState<boolean>(false);
  const [formTargetType, setFormTargetType] = useState<ReviewTargetType>('artifact');
  const [formTargetId, setFormTargetId] = useState<string>('');
  const [formReviewer, setFormReviewer] = useState<string>('custos-verifier');
  const [formMethod, setFormMethod] = useState<ReviewMethod>('automated_verifier');
  const [formStatus, setFormStatus] = useState<ReviewStatus>('approved');
  const [formSummary, setFormSummary] = useState<string>('');
  const [formDigest, setFormDigest] = useState<string>('');
  const [formFindings, setFormFindings] = useState<ReviewFinding[]>([]);

  // Findings draft within modal
  const [findingSeverity, setFindingSeverity] = useState<FindingSeverity>('info');
  const [findingCriterion, setFindingCriterion] = useState<string>('');
  const [findingMessage, setFindingMessage] = useState<string>('');
  const [findingFilePath, setFindingFilePath] = useState<string>('');
  const [findingLineNumber, setFindingLineNumber] = useState<string>('');

  const fetchReviews = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const list = await daemonClient.listReviews();
      setReviews(list);
      if (list.length > 0 && !selectedReviewId) {
        setSelectedReviewId(list[0].id);
      }
    } catch (err: any) {
      setError(err?.message || 'Failed to list reviewer records from daemon');
    } finally {
      setLoading(false);
    }
  }, [selectedReviewId]);

  useEffect(() => {
    fetchReviews();
  }, [fetchReviews]);

  const selectedReview = useMemo(() => {
    return reviews.find((r) => r.id === selectedReviewId) || null;
  }, [reviews, selectedReviewId]);

  const filteredReviews = useMemo(() => {
    return reviews.filter((r) => {
      if (targetTypeFilter !== 'all' && (r.target_type || r.targetType) !== targetTypeFilter) {
        return false;
      }
      if (statusFilter !== 'all' && r.status !== statusFilter) {
        return false;
      }
      if (methodFilter !== 'all' && r.method !== methodFilter) {
        return false;
      }
      if (freshOnly && !(r.is_fresh ?? r.isFresh)) {
        return false;
      }
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const targetId = (r.target_id || r.targetId || '').toLowerCase();
        const reviewer = r.reviewer.toLowerCase();
        const summary = (r.evidence_summary || r.evidenceSummary || '').toLowerCase();
        const digest = (r.evidence_digest || r.evidenceDigest || '').toLowerCase();
        return (
          targetId.includes(q) ||
          reviewer.includes(q) ||
          summary.includes(q) ||
          digest.includes(q)
        );
      }
      return true;
    });
  }, [reviews, targetTypeFilter, statusFilter, methodFilter, freshOnly, searchQuery]);

  const stats = useMemo(() => {
    const total = reviews.length;
    const fresh = reviews.filter((r) => r.is_fresh ?? r.isFresh).length;
    const stale = total - fresh;
    const approved = reviews.filter((r) => r.status === 'approved').length;
    const rejected = reviews.filter((r) => r.status === 'rejected').length;
    return { total, fresh, stale, approved, rejected };
  }, [reviews]);

  const handleMarkStale = async (id: string) => {
    try {
      setMarkingStale(true);
      await daemonClient.markReviewStale(id);
      onShowToast?.(`Review record marked stale`);
      await fetchReviews();
    } catch (err: any) {
      onShowToast?.(`Failed to mark review stale: ${err?.message || err}`);
    } finally {
      setMarkingStale(false);
    }
  };

  const handleCopyDigest = (digest: string) => {
    navigator.clipboard.writeText(digest);
    setCopiedDigest(true);
    setTimeout(() => setCopiedDigest(false), 2000);
    onShowToast?.('Evidence digest copied to clipboard');
  };

  const handleAddFinding = () => {
    if (!findingCriterion.trim() || !findingMessage.trim()) {
      onShowToast?.('Criterion and finding message are required');
      return;
    }
    const lineNum = findingLineNumber.trim() ? parseInt(findingLineNumber.trim(), 10) : undefined;
    const newFinding: ReviewFinding = {
      severity: findingSeverity,
      criterion: findingCriterion.trim(),
      message: findingMessage.trim(),
      file_path: findingFilePath.trim() || undefined,
      line_number: Number.isNaN(lineNum) ? undefined : lineNum,
    };
    setFormFindings([...formFindings, newFinding]);
    setFindingCriterion('');
    setFindingMessage('');
    setFindingFilePath('');
    setFindingLineNumber('');
  };

  const handleRemoveFinding = (index: number) => {
    setFormFindings(formFindings.filter((_, i) => i !== index));
  };

  const handleSubmitReview = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formTargetId.trim()) {
      onShowToast?.('Target identifier is required');
      return;
    }
    if (!formReviewer.trim()) {
      onShowToast?.('Reviewer identifier is required');
      return;
    }
    if (!formSummary.trim()) {
      onShowToast?.('Evidence summary is required');
      return;
    }

    try {
      setSubmitting(true);
      const params: RecordReviewParams = {
        target_type: formTargetType,
        target_id: formTargetId.trim(),
        reviewer: formReviewer.trim(),
        method: formMethod,
        status: formStatus,
        evidence_summary: formSummary.trim(),
        evidence_digest: formDigest.trim() || undefined,
        findings: formFindings.length > 0 ? formFindings : undefined,
      };
      const created = await daemonClient.recordReview(params);
      onShowToast?.(`Recorded review for ${created.target_id || created.targetId}`);
      setShowSubmitModal(false);
      // Reset form
      setFormTargetId('');
      setFormSummary('');
      setFormDigest('');
      setFormFindings([]);
      await fetchReviews();
      setSelectedReviewId(created.id);
    } catch (err: any) {
      onShowToast?.(`Failed to record review: ${err?.message || err}`);
    } finally {
      setSubmitting(false);
    }
  };

  const renderStatusBadge = (status: ReviewStatus) => {
    switch (status) {
      case 'approved':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
            <CheckCircle2 className="w-3 h-3" /> Approved
          </span>
        );
      case 'rejected':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-rose-500/10 text-rose-400 border border-rose-500/20">
            <XCircle className="w-3 h-3" /> Rejected
          </span>
        );
      case 'degraded':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20">
            <AlertTriangle className="w-3 h-3" /> Degraded
          </span>
        );
      case 'pending':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-sky-500/10 text-sky-400 border border-sky-500/20">
            <Clock className="w-3 h-3" /> Pending
          </span>
        );
      default:
        return null;
    }
  };

  const renderSeverityBadge = (severity: FindingSeverity) => {
    switch (severity) {
      case 'info':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20">
            info
          </span>
        );
      case 'warning':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20">
            warning
          </span>
        );
      case 'error':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-rose-500/10 text-rose-400 border border-rose-500/20">
            error
          </span>
        );
      case 'blocker':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-red-600/20 text-red-300 border border-red-500/30">
            blocker
          </span>
        );
      default:
        return null;
    }
  };

  const renderTargetIcon = (targetType: ReviewTargetType) => {
    switch (targetType) {
      case 'artifact':
        return <FileCode className="w-3.5 h-3.5 text-blue-400" />;
      case 'diff':
        return <GitBranch className="w-3.5 h-3.5 text-emerald-400" />;
      case 'task':
        return <CheckSquare className="w-3.5 h-3.5 text-purple-400" />;
      case 'run':
        return <Terminal className="w-3.5 h-3.5 text-amber-400" />;
      case 'note':
        return <FileText className="w-3.5 h-3.5 text-teal-400" />;
      case 'workspace':
        return <Cpu className="w-3.5 h-3.5 text-rose-400" />;
      default:
        return <ShieldCheck className="w-3.5 h-3.5 text-fg-muted" />;
    }
  };

  return (
    <div className="flex flex-col h-full bg-bg-canvas text-fg-default overflow-hidden">
      {/* Top Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border-subtle bg-bg-surface/50 backdrop-blur-sm">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
            <ShieldCheck className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-base font-semibold tracking-tight text-fg-default">
              Evidence & Verifier Records
            </h1>
            <p className="text-xs text-fg-subtle">
              Cryptographically anchored criteria verifications, verifier lineages, status pills, and freshness audit trail.
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={fetchReviews}
            disabled={loading}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md border border-border-subtle bg-bg-surface hover:bg-bg-subtle text-fg-muted hover:text-fg-default transition-colors disabled:opacity-50"
            title="Refresh review records"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
          <button
            onClick={() => setShowSubmitModal(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-emerald-600 hover:bg-emerald-500 text-white transition-colors shadow-sm"
          >
            <Plus className="w-3.5 h-3.5" />
            Record Review
          </button>
        </div>
      </div>

      {/* Telemetry Stats Bar */}
      <div className="grid grid-cols-5 gap-3 px-6 py-3 border-b border-border-subtle bg-bg-canvas text-xs font-mono">
        <div className="flex items-center justify-between p-2 rounded border border-border-subtle bg-bg-surface/30">
          <span className="text-fg-subtle">Total Records</span>
          <span className="font-semibold text-fg-default">{stats.total}</span>
        </div>
        <div className="flex items-center justify-between p-2 rounded border border-emerald-500/20 bg-emerald-500/5">
          <span className="text-emerald-400">Fresh (Active)</span>
          <span className="font-semibold text-emerald-300">{stats.fresh}</span>
        </div>
        <div className="flex items-center justify-between p-2 rounded border border-amber-500/20 bg-amber-500/5">
          <span className="text-amber-400">Stale</span>
          <span className="font-semibold text-amber-300">{stats.stale}</span>
        </div>
        <div className="flex items-center justify-between p-2 rounded border border-blue-500/20 bg-blue-500/5">
          <span className="text-blue-400">Approved</span>
          <span className="font-semibold text-blue-300">{stats.approved}</span>
        </div>
        <div className="flex items-center justify-between p-2 rounded border border-rose-500/20 bg-rose-500/5">
          <span className="text-rose-400">Rejected</span>
          <span className="font-semibold text-rose-300">{stats.rejected}</span>
        </div>
      </div>

      {/* Error Alert */}
      {error && (
        <div className="mx-6 my-2 p-3 rounded-lg border border-rose-500/30 bg-rose-500/10 text-rose-300 text-xs flex items-center justify-between">
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-rose-400" />
            <span>{error}</span>
          </div>
          <button
            onClick={fetchReviews}
            className="px-2 py-0.5 rounded border border-rose-500/30 hover:bg-rose-500/20 text-[11px]"
          >
            Retry
          </button>
        </div>
      )}

      {/* Main Split Body */}
      <div className="flex flex-1 min-h-0 overflow-hidden">
        {/* Left Pane: Records Ledger */}
        <div className="w-96 flex flex-col border-r border-border-subtle bg-bg-surface/20 min-h-0">
          {/* Search & Filters */}
          <div className="p-3 border-b border-border-subtle space-y-2">
            <div className="relative">
              <Search className="w-3.5 h-3.5 absolute left-2.5 top-2.5 text-fg-subtle" />
              <input
                type="text"
                placeholder="Search target, reviewer, hash..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-8 pr-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default"
              />
            </div>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <select
                value={targetTypeFilter}
                onChange={(e) => setTargetTypeFilter(e.target.value)}
                className="px-2 py-1 bg-bg-canvas border border-border-subtle rounded text-fg-muted focus:outline-none"
              >
                <option value="all">All Targets</option>
                {TARGET_TYPES.map((t) => (
                  <option key={t.value} value={t.value}>
                    {t.label}
                  </option>
                ))}
              </select>
              <select
                value={statusFilter}
                onChange={(e) => setStatusFilter(e.target.value)}
                className="px-2 py-1 bg-bg-canvas border border-border-subtle rounded text-fg-muted focus:outline-none"
              >
                <option value="all">All Statuses</option>
                {REVIEW_STATUSES.map((s) => (
                  <option key={s.value} value={s.value}>
                    {s.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="flex items-center justify-between pt-1">
              <select
                value={methodFilter}
                onChange={(e) => setMethodFilter(e.target.value)}
                className="px-2 py-1 bg-bg-canvas border border-border-subtle rounded text-xs text-fg-muted focus:outline-none"
              >
                <option value="all">All Methods</option>
                {REVIEW_METHODS.map((m) => (
                  <option key={m.value} value={m.value}>
                    {m.label}
                  </option>
                ))}
              </select>
              <label className="flex items-center gap-1.5 text-[11px] text-fg-subtle cursor-pointer">
                <input
                  type="checkbox"
                  checked={freshOnly}
                  onChange={(e) => setFreshOnly(e.target.checked)}
                  className="rounded border-border-subtle bg-bg-canvas text-emerald-500 focus:ring-0"
                />
                Fresh Only
              </label>
            </div>
          </div>

          {/* Records List */}
          <div className="flex-1 overflow-y-auto p-2 space-y-2 min-h-0">
            {loading && reviews.length === 0 ? (
              <div className="flex items-center justify-center p-8 text-xs text-fg-subtle">
                <RefreshCw className="w-4 h-4 animate-spin mr-2" />
                Loading review records...
              </div>
            ) : filteredReviews.length === 0 ? (
              <div className="p-8 text-center text-xs text-fg-subtle space-y-2">
                <ShieldAlert className="w-8 h-8 mx-auto text-fg-subtle opacity-50" />
                <p>No reviewer records match the selected filter.</p>
                <button
                  onClick={() => setShowSubmitModal(true)}
                  className="inline-flex items-center gap-1 px-3 py-1 text-xs text-emerald-400 hover:underline"
                >
                  <Plus className="w-3 h-3" /> Record first review
                </button>
              </div>
            ) : (
              filteredReviews.map((rev) => {
                const isSelected = rev.id === selectedReviewId;
                const isFresh = rev.is_fresh ?? rev.isFresh;
                const targetType = rev.target_type || rev.targetType;
                const targetId = rev.target_id || rev.targetId;
                const summary = rev.evidence_summary || rev.evidenceSummary;
                const findingsCount = rev.findings?.length || 0;

                return (
                  <div
                    key={rev.id}
                    onClick={() => setSelectedReviewId(rev.id)}
                    className={`p-3 rounded-lg border transition-all cursor-pointer ${
                      isSelected
                        ? 'border-emerald-500/40 bg-emerald-500/5 shadow-sm'
                        : 'border-border-subtle bg-bg-surface/40 hover:border-border-default hover:bg-bg-surface/70'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1.5">
                      <div className="flex items-center gap-1.5">
                        {renderTargetIcon(targetType)}
                        <span className="text-[11px] font-mono font-medium text-fg-subtle uppercase">
                          {targetType}
                        </span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        {renderStatusBadge(rev.status)}
                        <span
                          className={`inline-block w-2 h-2 rounded-full ${
                            isFresh ? 'bg-emerald-400' : 'bg-amber-400'
                          }`}
                          title={isFresh ? 'Fresh verification' : 'Stale verification'}
                        />
                      </div>
                    </div>

                    <div className="text-xs font-mono font-medium text-fg-default truncate mb-1">
                      {targetId}
                    </div>

                    <p className="text-[11px] text-fg-subtle line-clamp-2 mb-2 leading-relaxed">
                      {summary}
                    </p>

                    <div className="flex items-center justify-between text-[10px] text-fg-subtle border-t border-border-subtle/50 pt-2 font-mono">
                      <span className="flex items-center gap-1 truncate max-w-[120px]">
                        <UserCheck className="w-3 h-3 text-fg-muted" />
                        {rev.reviewer}
                      </span>
                      <span>
                        {findingsCount} finding{findingsCount === 1 ? '' : 's'}
                      </span>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Right Pane: Inspector / Detail */}
        <div className="flex-1 flex flex-col min-h-0 bg-bg-surface/10 overflow-y-auto">
          {selectedReview ? (
            <div className="p-6 space-y-6 max-w-4xl">
              {/* Review Header Banner */}
              <div className="p-5 rounded-xl border border-border-subtle bg-bg-surface/60 backdrop-blur-sm space-y-4">
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      {renderTargetIcon(selectedReview.target_type || selectedReview.targetType)}
                      <span className="text-xs font-mono uppercase text-fg-subtle">
                        {selectedReview.target_type || selectedReview.targetType}
                      </span>
                      <span className="text-fg-subtle">/</span>
                      <span className="text-sm font-semibold text-fg-default font-mono">
                        {selectedReview.target_id || selectedReview.targetId}
                      </span>
                    </div>
                    <div className="text-xs text-fg-subtle font-mono">
                      Record ID: {selectedReview.id}
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    {renderStatusBadge(selectedReview.status)}
                    <span
                      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-mono border ${
                        (selectedReview.is_fresh ?? selectedReview.isFresh)
                          ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                          : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                      }`}
                    >
                      <span
                        className={`w-1.5 h-1.5 rounded-full ${
                          (selectedReview.is_fresh ?? selectedReview.isFresh)
                            ? 'bg-emerald-400'
                            : 'bg-amber-400'
                        }`}
                      />
                      {(selectedReview.is_fresh ?? selectedReview.isFresh) ? 'Fresh' : 'Stale'}
                    </span>
                  </div>
                </div>

                {/* Toolbar */}
                <div className="flex items-center justify-between pt-3 border-t border-border-subtle/50 text-xs">
                  <div className="flex items-center gap-4 text-fg-subtle font-mono text-[11px]">
                    <span className="flex items-center gap-1.5">
                      <Cpu className="w-3.5 h-3.5 text-fg-muted" />
                      Reviewer: <strong className="text-fg-default">{selectedReview.reviewer}</strong>
                    </span>
                    <span className="flex items-center gap-1.5">
                      <ShieldCheck className="w-3.5 h-3.5 text-fg-muted" />
                      Method: <strong className="text-fg-default">{selectedReview.method}</strong>
                    </span>
                  </div>

                  <div className="flex items-center gap-2">
                    {(selectedReview.is_fresh ?? selectedReview.isFresh) && (
                      <button
                        onClick={() => handleMarkStale(selectedReview.id)}
                        disabled={markingStale}
                        className="px-2.5 py-1 text-xs rounded border border-amber-500/30 text-amber-400 hover:bg-amber-500/10 transition-colors disabled:opacity-50"
                      >
                        {markingStale ? 'Marking...' : 'Mark Stale'}
                      </button>
                    )}
                  </div>
                </div>
              </div>

              {/* Summary & Digest Card */}
              <div className="p-5 rounded-xl border border-border-subtle bg-bg-surface/40 space-y-4">
                <h2 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle flex items-center gap-2">
                  <FileText className="w-3.5 h-3.5 text-fg-muted" />
                  Evidence Summary
                </h2>
                <div className="text-xs text-fg-default leading-relaxed whitespace-pre-wrap p-3 rounded-lg bg-bg-canvas/50 border border-border-subtle font-sans">
                  {selectedReview.evidence_summary || selectedReview.evidenceSummary}
                </div>

                {(selectedReview.evidence_digest || selectedReview.evidenceDigest) && (
                  <div className="space-y-1.5">
                    <span className="text-[11px] font-mono text-fg-subtle flex items-center gap-1.5">
                      <Hash className="w-3 h-3" /> Cryptographic Evidence Digest (CAS)
                    </span>
                    <div className="flex items-center justify-between p-2 rounded-lg bg-bg-canvas border border-border-subtle font-mono text-xs">
                      <span className="text-fg-muted truncate">
                        {selectedReview.evidence_digest || selectedReview.evidenceDigest}
                      </span>
                      <button
                        onClick={() =>
                          handleCopyDigest(
                            selectedReview.evidence_digest || selectedReview.evidenceDigest || ''
                          )
                        }
                        className="p-1 text-fg-subtle hover:text-fg-default transition-colors ml-2"
                        title="Copy digest"
                      >
                        {copiedDigest ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                      </button>
                    </div>
                  </div>
                )}
              </div>

              {/* Findings Section */}
              <div className="p-5 rounded-xl border border-border-subtle bg-bg-surface/40 space-y-4">
                <div className="flex items-center justify-between">
                  <h2 className="text-xs font-semibold uppercase tracking-wider text-fg-subtle flex items-center gap-2">
                    <AlertCircle className="w-3.5 h-3.5 text-fg-muted" />
                    Review Findings ({selectedReview.findings?.length || 0})
                  </h2>
                </div>

                {(!selectedReview.findings || selectedReview.findings.length === 0) ? (
                  <div className="p-6 text-center text-xs text-fg-subtle rounded-lg border border-dashed border-border-subtle">
                    No findings recorded. Verification completed without issues.
                  </div>
                ) : (
                  <div className="space-y-2">
                    {selectedReview.findings.map((f, idx) => (
                      <div
                        key={idx}
                        className="p-3 rounded-lg border border-border-subtle bg-bg-canvas/60 space-y-2"
                      >
                        <div className="flex items-center justify-between text-xs">
                          <div className="flex items-center gap-2">
                            {renderSeverityBadge(f.severity)}
                            <span className="font-mono font-medium text-fg-default">
                              {f.criterion}
                            </span>
                          </div>
                          {(f.file_path || f.filePath) && (
                            <span className="font-mono text-[11px] text-fg-subtle flex items-center gap-1">
                              <FileCode className="w-3 h-3" />
                              {f.file_path || f.filePath}
                              {(f.line_number ?? f.lineNumber) !== undefined &&
                                `:${f.line_number ?? f.lineNumber}`}
                            </span>
                          )}
                        </div>
                        <p className="text-xs text-fg-muted pl-1 leading-relaxed">
                          {f.message}
                        </p>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Gate 4 Integrity Audit Card */}
              <div className="p-4 rounded-xl border border-emerald-500/20 bg-emerald-500/5 space-y-2">
                <div className="flex items-center gap-2 text-xs font-semibold text-emerald-400">
                  <ShieldCheck className="w-4 h-4" />
                  Gate 4 Verifier Contract Enforcement
                </div>
                <p className="text-xs text-fg-subtle leading-relaxed">
                  Under Custos fail-closed posture, execution workspaces, diffs, and synthesis artifacts must possess active, fresh verification records before promotion to production branch or coding handoff.
                </p>
              </div>
            </div>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center p-8 text-center text-fg-subtle space-y-3">
              <ShieldCheck className="w-12 h-12 text-fg-subtle opacity-40" />
              <div className="space-y-1">
                <h3 className="text-sm font-medium text-fg-default">No Record Selected</h3>
                <p className="text-xs max-w-sm">
                  Select a verifier record from the ledger on the left to inspect criteria findings, method lineage, and freshness status.
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Record Review Modal */}
      {showSubmitModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
          <div className="w-full max-w-2xl bg-bg-surface border border-border-default rounded-xl shadow-2xl flex flex-col max-h-[90vh] overflow-hidden">
            {/* Modal Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-border-subtle bg-bg-surface/80">
              <div className="flex items-center gap-2.5">
                <ShieldCheck className="w-5 h-5 text-emerald-400" />
                <h2 className="text-sm font-semibold text-fg-default">
                  Record Review & Criteria Verification
                </h2>
              </div>
              <button
                onClick={() => setShowSubmitModal(false)}
                className="text-fg-subtle hover:text-fg-default text-lg leading-none p-1"
              >
                &times;
              </button>
            </div>

            {/* Modal Body */}
            <form onSubmit={handleSubmitReview} className="flex-1 overflow-y-auto p-6 space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-medium text-fg-subtle mb-1">
                    Target Type
                  </label>
                  <select
                    value={formTargetType}
                    onChange={(e) => setFormTargetType(e.target.value as ReviewTargetType)}
                    className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default focus:outline-none focus:border-border-default"
                  >
                    {TARGET_TYPES.map((t) => (
                      <option key={t.value} value={t.value}>
                        {t.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label className="block text-xs font-medium text-fg-subtle mb-1">
                    Target Identifier
                  </label>
                  <input
                    type="text"
                    placeholder="e.g. artifacts/report.md or git-sha"
                    value={formTargetId}
                    onChange={(e) => setFormTargetId(e.target.value)}
                    required
                    className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default font-mono"
                  />
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div>
                  <label className="block text-xs font-medium text-fg-subtle mb-1">
                    Reviewer ID
                  </label>
                  <input
                    type="text"
                    placeholder="e.g. custos-verifier, cargo-clippy"
                    value={formReviewer}
                    onChange={(e) => setFormReviewer(e.target.value)}
                    required
                    className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default font-mono"
                  />
                </div>

                <div>
                  <label className="block text-xs font-medium text-fg-subtle mb-1">
                    Review Method
                  </label>
                  <select
                    value={formMethod}
                    onChange={(e) => setFormMethod(e.target.value as ReviewMethod)}
                    className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default focus:outline-none focus:border-border-default"
                  >
                    {REVIEW_METHODS.map((m) => (
                      <option key={m.value} value={m.value}>
                        {m.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label className="block text-xs font-medium text-fg-subtle mb-1">
                    Status
                  </label>
                  <select
                    value={formStatus}
                    onChange={(e) => setFormStatus(e.target.value as ReviewStatus)}
                    className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default focus:outline-none focus:border-border-default"
                  >
                    {REVIEW_STATUSES.map((s) => (
                      <option key={s.value} value={s.value}>
                        {s.label}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div>
                <label className="block text-xs font-medium text-fg-subtle mb-1">
                  Evidence Summary
                </label>
                <textarea
                  rows={3}
                  placeholder="Describe the verification findings, test results, or audit rationale..."
                  value={formSummary}
                  onChange={(e) => setFormSummary(e.target.value)}
                  required
                  className="w-full px-3 py-2 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-fg-subtle mb-1">
                  Evidence Digest (Optional SHA256 / Hash)
                </label>
                <input
                  type="text"
                  placeholder="e.g. sha256:4a3b8c..."
                  value={formDigest}
                  onChange={(e) => setFormDigest(e.target.value)}
                  className="w-full px-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default font-mono"
                />
              </div>

              {/* Findings Builder */}
              <div className="pt-2 border-t border-border-subtle space-y-3">
                <div className="flex items-center justify-between">
                  <label className="text-xs font-semibold text-fg-subtle uppercase tracking-wider">
                    Findings ({formFindings.length})
                  </label>
                </div>

                {/* Mini Form for Finding */}
                <div className="p-3 rounded-lg border border-border-subtle bg-bg-canvas/50 space-y-2">
                  <div className="grid grid-cols-4 gap-2">
                    <div>
                      <select
                        value={findingSeverity}
                        onChange={(e) => setFindingSeverity(e.target.value as FindingSeverity)}
                        className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default"
                      >
                        {SEVERITIES.map((s) => (
                          <option key={s.value} value={s.value}>
                            {s.label}
                          </option>
                        ))}
                      </select>
                    </div>
                    <div className="col-span-3">
                      <input
                        type="text"
                        placeholder="Criterion (e.g. zero_clippy_warnings)"
                        value={findingCriterion}
                        onChange={(e) => setFindingCriterion(e.target.value)}
                        className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default font-mono"
                      />
                    </div>
                  </div>

                  <input
                    type="text"
                    placeholder="Finding message / observation"
                    value={findingMessage}
                    onChange={(e) => setFindingMessage(e.target.value)}
                    className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default"
                  />

                  <div className="grid grid-cols-3 gap-2">
                    <div className="col-span-2">
                      <input
                        type="text"
                        placeholder="File path (optional)"
                        value={findingFilePath}
                        onChange={(e) => setFindingFilePath(e.target.value)}
                        className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default font-mono"
                      />
                    </div>
                    <div>
                      <input
                        type="number"
                        placeholder="Line # (opt)"
                        value={findingLineNumber}
                        onChange={(e) => setFindingLineNumber(e.target.value)}
                        className="w-full px-2 py-1 text-xs bg-bg-surface border border-border-subtle rounded text-fg-default font-mono"
                      />
                    </div>
                  </div>

                  <div className="flex justify-end pt-1">
                    <button
                      type="button"
                      onClick={handleAddFinding}
                      className="px-2.5 py-1 text-xs rounded bg-bg-surface hover:bg-bg-subtle border border-border-subtle text-fg-default font-medium transition-colors"
                    >
                      + Add Finding
                    </button>
                  </div>
                </div>

                {/* Findings List in Modal */}
                {formFindings.length > 0 && (
                  <div className="space-y-1.5 max-h-36 overflow-y-auto">
                    {formFindings.map((f, i) => (
                      <div
                        key={i}
                        className="flex items-center justify-between p-2 rounded border border-border-subtle bg-bg-surface text-xs"
                      >
                        <div className="flex items-center gap-2 truncate">
                          {renderSeverityBadge(f.severity)}
                          <span className="font-mono text-fg-default truncate">
                            {f.criterion}:
                          </span>
                          <span className="text-fg-subtle truncate">{f.message}</span>
                        </div>
                        <button
                          type="button"
                          onClick={() => handleRemoveFinding(i)}
                          className="text-fg-subtle hover:text-rose-400 p-1 transition-colors"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Modal Footer */}
              <div className="flex items-center justify-end gap-2 pt-4 border-t border-border-subtle">
                <button
                  type="button"
                  onClick={() => setShowSubmitModal(false)}
                  className="px-4 py-2 text-xs font-medium rounded-md border border-border-subtle hover:bg-bg-subtle text-fg-muted hover:text-fg-default transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={submitting}
                  className="px-4 py-2 text-xs font-medium rounded-md bg-emerald-600 hover:bg-emerald-500 text-white transition-colors disabled:opacity-50 shadow-sm"
                >
                  {submitting ? 'Recording...' : 'Record Review'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
