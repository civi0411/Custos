import React from 'react';
import {
  ChevronDown,
  ChevronUp,
  Plus,
  Minus,
  Trash2,
  Copy,
  Check,
  PanelLeft,
  PanelLeftClose,
} from 'lucide-react';
import { getFileIcon } from '@/utils/fileIcons';
import type {
  WorkspaceFileDiff,
  WorkspaceFileContent,
  WorkspaceDiffEntry,
} from '@/types/domain';

export interface DiffViewerProps {
  selectedPath: string;
  fileDiff: WorkspaceFileDiff;
  fileContent: WorkspaceFileContent | null;
  currentDiffEntry?: WorkspaceDiffEntry;
  diffStats: { added: number; deleted: number } | null;
  isExplorerOpen: boolean;
  onToggleExplorer: () => void;
  onViewModeChange: (mode: 'content' | 'diff') => void;
  changedFileList: WorkspaceDiffEntry[];
  currentDiffIndex: number;
  onPrevDiff: () => void;
  onNextDiff: () => void;
  onPrimaryAction: () => void;
  onDiscardCurrent: () => void;
  onCopyPath: () => void;
  copied: boolean;
  actionInProgress: string | null;
}

export const DiffViewer: React.FC<DiffViewerProps> = ({
  selectedPath,
  fileDiff,
  fileContent,
  currentDiffEntry,
  diffStats,
  isExplorerOpen,
  onToggleExplorer,
  onViewModeChange,
  changedFileList,
  currentDiffIndex,
  onPrevDiff,
  onNextDiff,
  onPrimaryAction,
  onDiscardCurrent,
  onCopyPath,
  copied,
  actionInProgress,
}) => {
  return (
    <>
      {/* Diff Header */}
      <div className="flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-3 text-xs bg-[var(--color-surface-1)]/40 font-mono">
        <div className="flex items-center gap-2 truncate text-[11px] text-[var(--color-fg-muted)]">
          <button
            type="button"
            onClick={onToggleExplorer}
            className={`p-1 rounded transition mr-0.5 ${
              isExplorerOpen
                ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
                : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
            }`}
            title={isExplorerOpen ? 'Hide Files Navigator' : 'Show Files Navigator'}
          >
            {isExplorerOpen ? <PanelLeftClose className="h-3.5 w-3.5" /> : <PanelLeft className="h-3.5 w-3.5" />}
          </button>
          <span className="text-[var(--color-border-subtle)]">|</span>
          {getFileIcon(selectedPath)}
          <span className="text-[var(--color-editor-fg)] font-semibold truncate">{selectedPath}</span>
          {currentDiffEntry && (
            <span className="text-[10px] font-mono text-white/70">
              {currentDiffEntry.is_staged ? '• Staged' : '• Modified'}
            </span>
          )}
        </div>

        <div className="flex items-center gap-1.5">
          {/* View Mode Toggle */}
          {fileContent && (
            <div className="flex items-center rounded bg-[var(--color-surface-2)] p-0.5 border border-[var(--color-border-subtle)] text-[10px] mr-1">
              <button
                type="button"
                onClick={() => onViewModeChange('diff')}
                className="px-2 py-0.5 rounded transition bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-xs font-semibold"
              >
                Git Diff
              </button>
              <button
                type="button"
                onClick={() => onViewModeChange('content')}
                className="px-2 py-0.5 rounded transition text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                Code Details
              </button>
            </div>
          )}

          {diffStats && (
            <span className="flex items-center gap-1 font-mono text-[10px] px-1.5 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] mr-0.5">
              <span className="text-white/90 font-semibold font-mono">+{diffStats.added}</span>
              <span className="text-white/50 font-semibold font-mono">-{diffStats.deleted}</span>
            </span>
          )}

          {changedFileList.length > 1 && (
            <div className="flex items-center gap-0.5 px-1 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] text-[10px] text-[var(--color-fg-muted)] mr-0.5">
              <button
                type="button"
                onClick={onPrevDiff}
                className="hover:text-[var(--color-editor-fg)] transition p-0.5"
                title="Previous changed file"
              >
                <ChevronUp className="h-3 w-3" />
              </button>
              <span className="font-mono text-[9px] px-1 select-none">
                {currentDiffIndex >= 0 ? currentDiffIndex + 1 : 1}/{changedFileList.length}
              </span>
              <button
                type="button"
                onClick={onNextDiff}
                className="hover:text-[var(--color-editor-fg)] transition p-0.5"
                title="Next changed file"
              >
                <ChevronDown className="h-3 w-3" />
              </button>
            </div>
          )}

          {currentDiffEntry && (
            <>
              <button
                type="button"
                disabled={!!actionInProgress}
                onClick={onPrimaryAction}
                className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] transition disabled:opacity-50 ${
                  currentDiffEntry.is_staged
                    ? 'border border-white/15 bg-white/5 text-white/80 hover:bg-white/10 hover:text-white font-medium'
                    : 'bg-white text-black hover:bg-white/90 font-semibold shadow-sm'
                }`}
              >
                {currentDiffEntry.is_staged ? (
                  <>
                    <Minus className="h-2.5 w-2.5" />
                    <span>Unstage</span>
                  </>
                ) : (
                  <>
                    <Plus className="h-2.5 w-2.5" />
                    <span>Stage</span>
                  </>
                )}
              </button>
              <button
                type="button"
                disabled={!!actionInProgress}
                onClick={onDiscardCurrent}
                className="flex items-center gap-1 px-2 py-0.5 rounded text-[10px] text-white/60 hover:text-white hover:bg-white/10 transition"
                title="Discard changes"
              >
                <Trash2 className="h-2.5 w-2.5" />
                <span>Discard</span>
              </button>
            </>
          )}

          <button
            type="button"
            onClick={onCopyPath}
            className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
            title="Copy file path"
          >
            {copied ? <Check className="h-3 w-3 text-white" /> : <Copy className="h-3 w-3" />}
          </button>
        </div>
      </div>

      {/* Diff Viewer Body */}
      <div className="flex-1 overflow-auto p-3 font-mono text-xs">
        <div className="space-y-0.5">
          {fileDiff.diff ? (
            fileDiff.diff.split('\n').map((line: string, idx: number) => {
              const isAdd = line.startsWith('+') && !line.startsWith('+++');
              const isDel = line.startsWith('-') && !line.startsWith('---');
              const isHunk = line.startsWith('@@');

              let lineClass = 'text-[var(--color-fg-muted)]';
              let bgClass = '';
              if (isAdd) {
                lineClass = 'text-emerald-400';
                bgClass = 'bg-emerald-500/10';
              } else if (isDel) {
                lineClass = 'text-rose-400';
                bgClass = 'bg-rose-500/10';
              } else if (isHunk) {
                lineClass = 'text-blue-400 font-semibold';
                bgClass = 'bg-blue-500/5';
              }

              return (
                <div
                  key={idx}
                  className={`flex px-2 py-0.5 whitespace-pre rounded-xs ${lineClass} ${bgClass}`}
                >
                  <span className="w-8 shrink-0 select-none text-[10px] text-[var(--color-fg-subtle)] text-right pr-2">
                    {idx + 1}
                  </span>
                  <span>{line}</span>
                </div>
              );
            })
          ) : (
            <div className="text-[var(--color-fg-subtle)]">No diff changes detected in file.</div>
          )}
        </div>
      </div>
    </>
  );
};
