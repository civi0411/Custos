import React from 'react';
import {
  Folder,
  GitBranch,
  Bot,
  MoreVertical,
  FileDiff,
  Terminal,
  RefreshCw,
  GitMerge,
  Loader2,
} from 'lucide-react';
import type { ExecutionWorkspace, DirtyManifest } from '@/types/domain';

export interface WorktreeCardProps {
  workspace: ExecutionWorkspace;
  isSelected: boolean;
  manifest?: DirtyManifest;
  showSecondaryMenu: boolean;
  integratingId: string | null;
  onSelect: () => void;
  onToggleSecondaryMenu: () => void;
  onCloseSecondaryMenu: () => void;
  onViewDiff: () => void;
  onOpenTerminal: () => void;
  onRecover: () => void;
  onIntegrate: () => void;
  onStartAgent: () => void;
}

export const WorktreeCard: React.FC<WorktreeCardProps> = ({
  workspace: ws,
  isSelected,
  manifest,
  showSecondaryMenu,
  integratingId,
  onSelect,
  onToggleSecondaryMenu,
  onCloseSecondaryMenu,
  onViewDiff,
  onOpenTerminal,
  onRecover,
  onIntegrate,
  onStartAgent,
}) => {
  const isDirty = manifest?.is_dirty || manifest?.isDirty;
  const dirtyCount =
    (manifest?.modified_files?.length || 0) +
    (manifest?.untracked_files?.length || 0) +
    (manifest?.deleted_files?.length || 0);

  const agentName = ws.running_agent_id || 'no agent session';
  const agentStatus = ws.agent_status || 'idle';
  const branchName = ws.active_branch || (ws.kind.type === 'git' ? ws.kind.branch || 'main' : 'default');

  return (
    <div
      onClick={onSelect}
      className={`rounded-xl border p-3.5 shadow-xs transition cursor-pointer ${
        isSelected
          ? 'border-white/30 bg-[var(--color-surface-2)]/60 ring-1 ring-white/20'
          : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] hover:border-[var(--color-border-default)]'
      }`}
    >
      {/* Top Row: Path, Branch, and Agent Badge */}
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <Folder className="h-4 w-4 text-white/60 shrink-0" />
            <span className="font-semibold text-xs truncate">
              {ws.name || ws.path.split('/').pop() || ws.id}
            </span>
            <span className="flex items-center gap-1 rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] font-mono text-white/70 border border-white/10">
              <GitBranch className="h-3 w-3" />
              {branchName}
            </span>
          </div>
          <p className="mt-1 font-mono text-[10px] text-[var(--color-fg-subtle)] truncate">
            {ws.path}
          </p>
        </div>

        {/* Agent Status Badge & Quick Integrate */}
        <div className="flex items-center gap-1.5 shrink-0">
          <span
            className={`flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide border ${
              agentStatus === 'running'
                ? 'bg-white/10 text-white border-white/20'
                : agentStatus === 'review_needed'
                ? 'bg-white/[0.06] text-white/80 border-white/15'
                : 'bg-zinc-500/10 text-[var(--color-fg-subtle)] border-zinc-500/20'
            }`}
          >
            <Bot className="h-3 w-3" />
            <span>{agentName}</span>
            <span className="opacity-60">({agentStatus})</span>
          </span>

          {/* Secondary ··· Menu */}
          <div className="relative">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onToggleSecondaryMenu();
              }}
              className="flex h-6 w-6 items-center justify-center rounded hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]"
            >
              <MoreVertical className="h-3.5 w-3.5" />
            </button>

            {showSecondaryMenu && (
              <div
                className="absolute right-0 top-7 z-50 w-48 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] py-1 shadow-lg text-xs"
                onMouseLeave={onCloseSecondaryMenu}
                onClick={(e) => e.stopPropagation()}
              >
                <button
                  type="button"
                  onClick={onViewDiff}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]"
                >
                  <FileDiff className="h-3.5 w-3.5 text-white/60" />
                  <span>View worktree files</span>
                </button>
                <button
                  type="button"
                  onClick={onOpenTerminal}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]"
                >
                  <Terminal className="h-3.5 w-3.5" />
                  <span>Open worktree terminal</span>
                </button>
                <button
                  type="button"
                  onClick={onRecover}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-white/80 hover:bg-[var(--color-surface-2)]"
                >
                  <RefreshCw className="h-3.5 w-3.5" />
                  <span>Recover Workspace State</span>
                </button>
                <button
                  type="button"
                  disabled={integratingId === ws.id}
                  onClick={onIntegrate}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-white hover:bg-[var(--color-surface-2)] disabled:opacity-40"
                >
                  <GitMerge className="h-3.5 w-3.5" />
                  <span>Merge into Base (Gated)</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Status & Dirty Files Banner */}
      <div className="mt-2.5 flex items-center justify-between text-[11px] text-[var(--color-fg-muted)]">
        <span className="flex items-center gap-1.5 font-mono">
          <span
            className={`h-1.5 w-1.5 rounded-full ${
              isDirty ? 'bg-white/40' : 'bg-white/90'
            }`}
          />
          {isDirty ? `${dirtyCount} uncommitted changes` : 'Clean tree'}
        </span>

        <button
          type="button"
          disabled={integratingId === ws.id}
          onClick={(e) => {
            e.stopPropagation();
            onIntegrate();
          }}
          className="flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] font-medium bg-white/10 hover:bg-white/20 text-white border border-white/20 disabled:opacity-40 transition"
        >
          {integratingId === ws.id ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : (
            <GitMerge className="h-3 w-3" />
          )}
          <span>Integrate</span>
        </button>
      </div>

      <div className="mt-2.5 pt-2 border-t border-white/5 flex items-center justify-between">
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onStartAgent();
          }}
          className="inline-flex items-center gap-1.5 rounded-md border border-white/15 bg-white/5 px-2.5 py-1 text-xs font-medium text-white/90 hover:bg-white/10 hover:text-white transition"
        >
          <Bot className="h-3.5 w-3.5" />
          <span>Start Agent</span>
        </button>
      </div>
    </div>
  );
};
