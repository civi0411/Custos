import React, { useState } from 'react';
import {
  ChevronUp,
  ChevronDown,
  Plus,
  Minus,
  Trash2,
  Copy,
  Check,
  PanelLeft,
  PanelLeftClose,
  MessageSquarePlus,
  Send,
} from 'lucide-react';
import type { WorkspaceDiffEntry, WorkspaceFileDiff, WorkspaceFileContent } from '@/types/domain';
import { GhostDismissButton } from '@/components/common/SecuritySurface';
import { getFileIcon } from './types';

export interface DiffLineAnnotation {
  id: string;
  filePath: string;
  startLine: number;
  endLine: number;
  linePreview: string;
  comment: string;
}

export function formatDiffAnnotationBatch(annotations: DiffLineAnnotation[]): string {
  return [
    `Inline Diff Review Batch (${annotations.length} notes):`,
    ...annotations.map(
      (a, i) =>
        `${i + 1}. ${a.filePath}:${a.startLine}${a.endLine !== a.startLine ? `-${a.endLine}` : ''} (${a.linePreview.trim().slice(0, 80)})\n   Note: ${a.comment}`
    ),
  ].join('\n');
}

export interface DiffViewerCanvasProps {
  selectedPath: string;
  isExplorerOpen: boolean;
  onToggleExplorer: () => void;
  currentDiffEntry?: WorkspaceDiffEntry;
  fileContent: WorkspaceFileContent | null;
  fileDiff: WorkspaceFileDiff;
  fileViewMode: 'content' | 'diff';
  onSetFileViewMode: (mode: 'content' | 'diff') => void;
  diffStats: { added: number; deleted: number } | null;
  changedFileList: WorkspaceDiffEntry[];
  currentDiffIndex: number;
  onPrevDiff: () => void;
  onNextDiff: () => void;
  actionInProgress: string | null;
  onPrimaryAction: () => void;
  onDiscardCurrent: () => void;
  onCopyPath: () => void;
  copied: boolean;
  annotations?: DiffLineAnnotation[];
  onAddAnnotation?: (annotation: Omit<DiffLineAnnotation, 'id'>) => void;
  onRemoveAnnotation?: (id: string) => void;
  onSendAnnotationBatch?: () => void;
  onClearAnnotations?: () => void;
  sendingAnnotations?: boolean;
}

export const DiffViewerCanvas: React.FC<DiffViewerCanvasProps> = ({
  selectedPath,
  isExplorerOpen,
  onToggleExplorer,
  currentDiffEntry,
  fileContent,
  fileDiff,
  onSetFileViewMode,
  diffStats,
  changedFileList,
  currentDiffIndex,
  onPrevDiff,
  onNextDiff,
  actionInProgress,
  onPrimaryAction,
  onDiscardCurrent,
  onCopyPath,
  copied,
  annotations = [],
  onAddAnnotation,
  onRemoveAnnotation,
  onSendAnnotationBatch,
  onClearAnnotations,
  sendingAnnotations = false,
}) => {
  const [draftLine, setDraftLine] = useState<{ lineNum: number; preview: string } | null>(null);
  const [draftComment, setDraftComment] = useState('');
  const fileAnnotations = annotations.filter((a) => a.filePath === selectedPath);
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
                onClick={() => onSetFileViewMode('diff')}
                className="px-2 py-0.5 rounded transition bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-xs font-semibold"
              >
                Git Diff
              </button>
              <button
                type="button"
                onClick={() => onSetFileViewMode('content')}
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

      {/* Annotation Batch Bar */}
      {annotations.length > 0 && onSendAnnotationBatch && (
        <div className="flex items-center justify-between px-3 py-1.5 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-[11px]">
          <span className="font-mono text-[var(--color-editor-fg)]">
            {annotations.length} inline diff {annotations.length === 1 ? 'note' : 'notes'} queued for agent
          </span>
          <div className="flex items-center gap-1.5">
            {onClearAnnotations && (
              <GhostDismissButton label="Clear" onClick={onClearAnnotations} disabled={sendingAnnotations} />
            )}
            <button
              type="button"
              disabled={sendingAnnotations}
              onClick={onSendAnnotationBatch}
              className="flex items-center gap-1 px-2.5 py-0.5 rounded bg-white text-black font-semibold hover:bg-white/90 transition disabled:opacity-50"
            >
              <Send className="h-3 w-3" />
              <span>{sendingAnnotations ? 'Sending…' : 'Send to Agent'}</span>
            </button>
          </div>
        </div>
      )}

      {/* Diff Viewer Body */}
      <div className="flex-1 overflow-auto p-3 font-mono text-xs">
        <div className="space-y-0.5">
          {fileDiff.diff ? (
            fileDiff.diff.split('\n').map((line: string, idx: number) => {
              const isAdd = line.startsWith('+') && !line.startsWith('+++');
              const isDel = line.startsWith('-') && !line.startsWith('---');
              const isHunk = line.startsWith('@@');
              const lineNum = idx + 1;
              const lineNotes = fileAnnotations.filter((a) => a.startLine === lineNum);

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
                <div key={idx}>
                  <div
                    className={`group flex items-start px-2 py-0.5 whitespace-pre rounded-xs ${lineClass} ${bgClass}`}
                  >
                    <button
                      type="button"
                      onClick={() => {
                        if (!onAddAnnotation) return;
                        setDraftLine({ lineNum, preview: line });
                        setDraftComment('');
                      }}
                      className="w-8 shrink-0 select-none text-[10px] text-[var(--color-fg-subtle)] hover:text-white text-right pr-2 cursor-pointer"
                      title={onAddAnnotation ? `Annotate line ${lineNum}` : undefined}
                    >
                      <span className="group-hover:hidden">{lineNum}</span>
                      <MessageSquarePlus className="hidden group-hover:inline h-3 w-3 text-white/80" />
                    </button>
                    <span>{line}</span>
                  </div>

                  {lineNotes.map((note) => (
                    <div
                      key={note.id}
                      className="ml-10 my-1 flex items-center justify-between gap-2 px-2.5 py-1 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-default)] text-[11px] font-sans text-[var(--color-editor-fg)]"
                    >
                      <span>{note.comment}</span>
                      {onRemoveAnnotation && (
                        <GhostDismissButton
                          label="Remove"
                          onClick={() => onRemoveAnnotation(note.id)}
                          className="py-0.5! text-[10px]!"
                        />
                      )}
                    </div>
                  ))}

                  {draftLine?.lineNum === lineNum && onAddAnnotation && (
                    <div className="ml-10 my-1 flex items-center gap-1.5 p-1.5 rounded bg-[var(--color-surface-1)] border border-[var(--color-border-default)] font-sans">
                      <input
                        autoFocus
                        value={draftComment}
                        onChange={(e) => setDraftComment(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter' && draftComment.trim()) {
                            onAddAnnotation({
                              filePath: selectedPath,
                              startLine: lineNum,
                              endLine: lineNum,
                              linePreview: draftLine.preview,
                              comment: draftComment.trim(),
                            });
                            setDraftLine(null);
                            setDraftComment('');
                          } else if (e.key === 'Escape') {
                            setDraftLine(null);
                          }
                        }}
                        placeholder={`Review note on line ${lineNum}…`}
                        className="flex-1 bg-transparent text-xs text-[var(--color-editor-fg)] focus:outline-none px-1"
                      />
                      <GhostDismissButton label="Cancel" onClick={() => setDraftLine(null)} />
                      <button
                        type="button"
                        disabled={!draftComment.trim()}
                        onClick={() => {
                          onAddAnnotation({
                            filePath: selectedPath,
                            startLine: lineNum,
                            endLine: lineNum,
                            linePreview: draftLine.preview,
                            comment: draftComment.trim(),
                          });
                          setDraftLine(null);
                          setDraftComment('');
                        }}
                        className="px-2 py-0.5 rounded bg-white text-black text-[11px] font-semibold disabled:opacity-40"
                      >
                        Add
                      </button>
                    </div>
                  )}
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
