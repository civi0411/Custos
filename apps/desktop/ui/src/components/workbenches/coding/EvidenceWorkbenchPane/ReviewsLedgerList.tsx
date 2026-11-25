import React from 'react';
import {
  Search,
  RefreshCw,
  ShieldAlert,
  Plus,
  UserCheck,
} from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import type { ReviewerRecord } from '@/types/domain';
import { TARGET_TYPES, REVIEW_METHODS, REVIEW_STATUSES } from './constants';
import { renderStatusBadge, renderTargetIcon } from './badges';

export interface ReviewsLedgerListProps {
  reviews: ReviewerRecord[];
  filteredReviews: ReviewerRecord[];
  selectedReviewId: string | null;
  loading: boolean;
  searchQuery: string;
  targetTypeFilter: string;
  statusFilter: string;
  methodFilter: string;
  freshOnly: boolean;
  onSearchChange: (v: string) => void;
  onTargetTypeFilterChange: (v: string) => void;
  onStatusFilterChange: (v: string) => void;
  onMethodFilterChange: (v: string) => void;
  onFreshOnlyChange: (v: boolean) => void;
  onSelectReview: (id: string) => void;
  onOpenSubmitModal: () => void;
}

export const ReviewsLedgerList: React.FC<ReviewsLedgerListProps> = ({
  reviews,
  filteredReviews,
  selectedReviewId,
  loading,
  searchQuery,
  targetTypeFilter,
  statusFilter,
  methodFilter,
  freshOnly,
  onSearchChange,
  onTargetTypeFilterChange,
  onStatusFilterChange,
  onMethodFilterChange,
  onFreshOnlyChange,
  onSelectReview,
  onOpenSubmitModal,
}) => {
  return (
    <div className="w-96 flex flex-col border-r border-border-subtle bg-bg-surface/20 min-h-0">
      {/* Search & Filters */}
      <div className="p-3 border-b border-border-subtle space-y-2">
        <div className="relative">
          <Search className="w-3.5 h-3.5 absolute left-2.5 top-2.5 text-fg-subtle" />
          <input
            type="text"
            placeholder="Search target, reviewer, hash..."
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            className="w-full pl-8 pr-3 py-1.5 text-xs bg-bg-canvas border border-border-subtle rounded-md text-fg-default placeholder:text-fg-subtle focus:outline-none focus:border-border-default"
          />
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs">
          <NativeSelect
            value={targetTypeFilter}
            onChange={(e) => onTargetTypeFilterChange(e.target.value)}
            className="px-2 py-1 bg-bg-canvas border border-border-subtle rounded text-fg-muted focus:outline-none"
          >
            <option value="all">All Targets</option>
            {TARGET_TYPES.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </NativeSelect>
          <NativeSelect
            value={statusFilter}
            onChange={(e) => onStatusFilterChange(e.target.value)}
            className="px-2 py-1 bg-bg-canvas border border-border-subtle rounded text-fg-muted focus:outline-none"
          >
            <option value="all">All Statuses</option>
            {REVIEW_STATUSES.map((s) => (
              <option key={s.value} value={s.value}>
                {s.label}
              </option>
            ))}
          </NativeSelect>
        </div>
        <div className="flex items-center justify-between pt-1">
          <NativeSelect
            value={methodFilter}
            onChange={(e) => onMethodFilterChange(e.target.value)}
            className="px-2 py-1 bg-bg-canvas border border-border-subtle rounded text-xs text-fg-muted focus:outline-none"
          >
            <option value="all">All Methods</option>
            {REVIEW_METHODS.map((m) => (
              <option key={m.value} value={m.value}>
                {m.label}
              </option>
            ))}
          </NativeSelect>
          <label className="flex items-center gap-1.5 text-[11px] text-fg-subtle cursor-pointer">
            <input
              type="checkbox"
              checked={freshOnly}
              onChange={(e) => onFreshOnlyChange(e.target.checked)}
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
              onClick={onOpenSubmitModal}
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
                onClick={() => onSelectReview(rev.id)}
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
  );
};
