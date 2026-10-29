import React from 'react';
import { X, RotateCcw } from 'lucide-react';
import type { NoteVersionRecord } from '@/types/domain';

export interface NoteVersionHistoryPanelProps {
  versions: NoteVersionRecord[];
  onRestore: (version: NoteVersionRecord) => void;
  onClose: () => void;
}

export const NoteVersionHistoryPanel: React.FC<NoteVersionHistoryPanelProps> = ({
  versions,
  onRestore,
  onClose,
}) => {
  return (
    <div className="w-72 flex-shrink-0 border-l border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 overflow-y-auto">
      <div className="flex items-center justify-between pb-3 border-b border-[var(--color-border-muted)]">
        <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
          Version History
        </span>
        <button
          onClick={onClose}
          className="text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] p-1 rounded hover:bg-[var(--color-surface-2)]"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="mt-3 space-y-3">
        {versions.length === 0 ? (
          <p className="text-xs text-[var(--color-fg-muted)]">
            No previous versions recorded.
          </p>
        ) : (
          versions.map((ver) => (
            <div
              key={ver.id}
              className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] p-2.5 text-xs"
            >
              <div className="flex items-center justify-between">
                <span className="font-semibold text-[var(--color-editor-fg)]">
                  v{ver.version}
                </span>
                <span className="text-[10px] text-[var(--color-fg-muted)]">
                  {new Date(ver.created_at * 1000).toLocaleTimeString()}
                </span>
              </div>

              <button
                onClick={() => onRestore(ver)}
                className="mt-2.5 flex items-center gap-1 rounded bg-[var(--color-surface-3)] px-2 py-1 text-[10px] font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-4)] transition w-full justify-center"
              >
                <RotateCcw className="h-3 w-3" />
                Restore Content
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
};
