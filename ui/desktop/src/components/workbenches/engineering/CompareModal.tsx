import React from 'react';
import { GitCompare, X, Loader2, CheckCircle2 } from 'lucide-react';

export interface CandidateComparison {
  workspace_id: string;
  name: string;
  path: string;
  branch: string;
  base_commit_hash?: string;
  is_dirty: boolean;
  modified_count: number;
  untracked_count: number;
  deleted_count: number;
  modified_files: string[];
  untracked_files: string[];
  deleted_files: string[];
  diff: string;
  status: string;
  inspection_error?: string;
  dirty_error?: string;
  diff_error?: string;
  is_ready_for_integration?: boolean;
}

export interface CompareModalProps {
  comparing: boolean;
  candidateComparisons: CandidateComparison[];
  integratingId: string | null;
  onIntegrateWorkspace: (workspaceId: string) => void;
  onClose: () => void;
}

export const CompareModal: React.FC<CompareModalProps> = ({
  comparing,
  candidateComparisons,
  integratingId,
  onIntegrateWorkspace,
  onClose,
}) => {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
      <div className="flex h-[80vh] w-[90vw] max-w-5xl flex-col rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] shadow-2xl">
        {/* Modal Header */}
        <div className="flex items-center justify-between border-b border-[var(--color-border-subtle)] px-4 py-3 bg-[var(--color-surface-1)]">
          <div className="flex items-center gap-2">
            <GitCompare className="h-4 w-4 text-white/70" />
            <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">
              Compare Candidates
            </h3>
            <span className="rounded bg-white/10 px-2 py-0.5 text-[10px] font-mono text-white/80 border border-white/10">
              {candidateComparisons.length} candidates
            </span>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded p-1 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Modal Body */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {comparing ? (
            <div className="flex h-48 items-center justify-center gap-2 text-xs text-[var(--color-fg-subtle)]">
              <Loader2 className="h-4 w-4 animate-spin text-white/60" />
              <span>Computing candidate unified diffs and dirty manifests...</span>
            </div>
          ) : candidateComparisons.length === 0 ? (
            <div className="text-center py-12 text-xs text-[var(--color-fg-muted)]">
              No candidates available for comparison.
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {candidateComparisons.map((c) => (
                <div
                  key={c.workspace_id}
                  className="flex flex-col rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4 shadow-xs"
                >
                  {c.inspection_error && (
                    <p role="alert" className="text-xs text-rose-500 mb-3">
                      {c.dirty_error || c.diff_error || c.inspection_error}
                    </p>
                  )}
                  <div className="flex items-start justify-between">
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-xs">{c.name}</span>
                        <span className="rounded bg-white/10 px-1.5 py-0.5 text-[10px] font-mono text-white/80 border border-white/10">
                          {c.branch}
                        </span>
                      </div>
                      <p className="mt-0.5 text-[10px] font-mono text-[var(--color-fg-subtle)]">
                        Base: {c.base_commit_hash?.slice(0, 8) || 'unknown'}
                      </p>
                    </div>

                    <button
                      type="button"
                      disabled={
                        integratingId === c.workspace_id ||
                        c.is_ready_for_integration !== true
                      }
                      onClick={() => onIntegrateWorkspace(c.workspace_id)}
                      className="flex items-center gap-1.5 rounded-md bg-white text-black hover:bg-white/90 px-2.5 py-1 text-xs font-semibold disabled:opacity-40 transition shadow-sm"
                    >
                      {integratingId === c.workspace_id ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        <CheckCircle2 className="h-3 w-3" />
                      )}
                      <span>Integrate</span>
                    </button>
                  </div>

                  {/* Diff Stats Banner */}
                  <div className="mt-3 flex items-center gap-3 text-[11px] font-mono text-[var(--color-fg-muted)]">
                    <span className="text-white/90">
                      {c.dirty_error ? 'Unknown' : `+${c.modified_count}`} modified
                    </span>
                    <span className="text-white/70">
                      {c.dirty_error ? 'Unknown' : `?${c.untracked_count}`} new
                    </span>
                    <span className="text-white/50">
                      {c.dirty_error ? 'Unknown' : `-${c.deleted_count}`} deleted
                    </span>
                  </div>

                  {/* Diff snippet */}
                  <div className="mt-3 flex-1 overflow-hidden rounded-lg border border-[var(--color-border-subtle)] bg-black/40 p-2.5 font-mono text-[11px] text-[var(--color-editor-fg)]">
                    {c.diff ? (
                      <pre className="max-h-56 overflow-auto whitespace-pre-wrap font-mono text-[10px] leading-relaxed text-[var(--color-fg-muted)]">
                        {c.diff}
                      </pre>
                    ) : (
                      <div className="py-8 text-center text-[10px] text-[var(--color-fg-subtle)] italic">
                        {c.diff_error
                          ? 'Diff is unavailable; retry inspection before integrating.'
                          : 'No uncommitted unified diff changes.'}
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
