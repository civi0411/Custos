import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { ShieldCheck, RefreshCw, Plus, AlertCircle } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { ReviewerRecord, RecordReviewParams } from '@/types/domain';
import type { EvidenceWorkbenchPaneProps } from './types';
import { ReviewsLedgerList } from './ReviewsLedgerList';
import { ReviewInspectorDetail } from './ReviewInspectorDetail';
import { RecordReviewModal } from './RecordReviewModal';

export const EvidenceWorkbenchView: React.FC<EvidenceWorkbenchPaneProps> = ({ onShowToast }) => {
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
      onShowToast?.('Review record marked stale');
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

  const handleSubmitReview = async (params: RecordReviewParams) => {
    try {
      setSubmitting(true);
      const created = await daemonClient.recordReview(params);
      onShowToast?.(`Recorded review for ${created.target_id || created.targetId}`);
      setShowSubmitModal(false);
      await fetchReviews();
      setSelectedReviewId(created.id);
    } catch (err: any) {
      onShowToast?.(`Failed to record review: ${err?.message || err}`);
    } finally {
      setSubmitting(false);
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
        <ReviewsLedgerList
          reviews={reviews}
          filteredReviews={filteredReviews}
          selectedReviewId={selectedReviewId}
          loading={loading}
          searchQuery={searchQuery}
          targetTypeFilter={targetTypeFilter}
          statusFilter={statusFilter}
          methodFilter={methodFilter}
          freshOnly={freshOnly}
          onSearchChange={setSearchQuery}
          onTargetTypeFilterChange={setTargetTypeFilter}
          onStatusFilterChange={setStatusFilter}
          onMethodFilterChange={setMethodFilter}
          onFreshOnlyChange={setFreshOnly}
          onSelectReview={setSelectedReviewId}
          onOpenSubmitModal={() => setShowSubmitModal(true)}
        />

        <ReviewInspectorDetail
          selectedReview={selectedReview}
          markingStale={markingStale}
          copiedDigest={copiedDigest}
          onMarkStale={handleMarkStale}
          onCopyDigest={handleCopyDigest}
        />
      </div>

      <RecordReviewModal
        isOpen={showSubmitModal}
        submitting={submitting}
        onClose={() => setShowSubmitModal(false)}
        onSubmit={handleSubmitReview}
        onShowToast={onShowToast}
      />
    </div>
  );
};
