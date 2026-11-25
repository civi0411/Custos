import React from 'react';
import { FolderGit2, FolderOpen, FolderTree, Terminal, FileCode } from 'lucide-react';
import type { WorkspaceDiffEntry } from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';
import { getFileIcon } from './types';

export interface EmptyCanvasViewProps {
  type: 'no-workspace' | 'no-file-selected';
  onOpenFolderDirect?: () => void;
  onOpenExplorer?: () => void;
  onOpenTab?: (id: OrcaTabId, customUrl?: string, customTitle?: string) => void;
  changedFileList?: WorkspaceDiffEntry[];
  onSelectPath?: (path: string) => void;
}

export const EmptyCanvasView: React.FC<EmptyCanvasViewProps> = ({
  type,
  onOpenFolderDirect,
  onOpenExplorer,
  onOpenTab,
  changedFileList = [],
  onSelectPath,
}) => {
  if (type === 'no-workspace') {
    return (
      <div className="flex-1 flex items-center justify-center p-6 bg-[var(--color-canvas)]">
        <div className="max-w-md w-full rounded-2xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-6 text-center space-y-4 shadow-xl">
          <div className="w-12 h-12 rounded-xl bg-white/[0.04] border border-white/10 text-white/70 flex items-center justify-center mx-auto">
            <FolderGit2 className="w-6 h-6" />
          </div>
          <div className="space-y-1">
            <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">No Workspace Opened</h3>
            <p className="text-xs text-[var(--color-fg-muted)] leading-relaxed">
              Open a project folder or repository from your machine to browse files and review changes.
            </p>
          </div>
          <button
            onClick={onOpenFolderDirect}
            className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-white text-black font-semibold text-xs hover:bg-white/90 shadow-sm transition"
          >
            <FolderOpen className="w-4 h-4 text-black" />
            <span>Open Project Folder</span>
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col items-center justify-center p-8 text-center text-xs text-[var(--color-fg-subtle)] space-y-4">
      <div className="w-12 h-12 rounded-xl bg-white/[0.04] border border-white/10 text-white/70 flex items-center justify-center mx-auto">
        <FileCode className="w-6 h-6" />
      </div>
      <div className="space-y-1">
        <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">
          Workspace Editor Canvas
        </h3>
      </div>

      <div className="flex items-center gap-2 pt-2">
        <button
          type="button"
          onClick={onOpenExplorer}
          className="px-3 py-1.5 rounded-lg bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-[11px] font-medium text-[var(--color-editor-fg)] border border-[var(--color-border-subtle)] transition flex items-center gap-1.5"
        >
          <FolderTree className="w-3.5 h-3.5 text-white/60" />
          <span>Open In-Canvas Navigator</span>
        </button>
        <button
          type="button"
          onClick={() => onOpenTab?.('terminal')}
          className="px-3 py-1.5 rounded-lg bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-[11px] font-medium text-[var(--color-editor-fg)] border border-[var(--color-border-subtle)] transition flex items-center gap-1.5"
        >
          <Terminal className="w-3.5 h-3.5 text-white/60" />
          <span>Open Terminal</span>
        </button>
      </div>

      {changedFileList.length > 0 && (
        <div className="w-full max-w-md pt-4 text-left border-t border-[var(--color-border-subtle)] mt-4">
          <div className="text-[11px] font-medium text-[var(--color-fg-muted)] mb-2 flex items-center justify-between">
            <span>Modified files ({changedFileList.length}):</span>
          </div>
          <div className="space-y-1 max-h-48 overflow-y-auto">
            {changedFileList.map((f) => (
              <div
                key={f.path}
                onClick={() => onSelectPath?.(f.path)}
                className="flex items-center justify-between px-2.5 py-1.5 rounded bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)] cursor-pointer text-[11px] font-mono transition border border-[var(--color-border-subtle)]/50 group"
              >
                <div className="flex items-center gap-1.5 truncate">
                  {getFileIcon(f.path)}
                  <span className="text-[var(--color-editor-fg)] group-hover:text-white transition truncate">{f.path}</span>
                </div>
                <div className="flex items-center gap-1 shrink-0 font-mono text-[10px]">
                  {f.additions > 0 && <span className="text-white/80 font-medium">+{f.additions}</span>}
                  {f.deletions > 0 && <span className="text-white/40 font-medium">-{f.deletions}</span>}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
