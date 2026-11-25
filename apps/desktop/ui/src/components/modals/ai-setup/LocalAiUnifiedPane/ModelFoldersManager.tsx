import React from 'react';
import { RefreshCw, FolderPlus, Folder, X } from 'lucide-react';
import type { LocalModelFolder } from '@/types/domain';

export interface ModelFoldersManagerProps {
  filterTab: 'all' | 'loaded' | 'gguf' | 'lmstudio';
  setFilterTab: (tab: 'all' | 'loaded' | 'gguf' | 'lmstudio') => void;
  counts: { all: number; loaded: number; gguf: number; lmstudio: number };
  scanningFolders: boolean;
  onScanFolders: () => void;
  showAddFolder: boolean;
  setShowAddFolder: (val: boolean | ((prev: boolean) => boolean)) => void;
  newFolderPath: string;
  setNewFolderPath: (val: string) => void;
  onPickFolder: () => void;
  onAddFolder: () => void;
  folders: LocalModelFolder[];
  onRemoveFolder: (id: string) => void;
}

export const ModelFoldersManager: React.FC<ModelFoldersManagerProps> = ({
  filterTab,
  setFilterTab,
  counts,
  scanningFolders,
  onScanFolders,
  showAddFolder,
  setShowAddFolder,
  newFolderPath,
  setNewFolderPath,
  onPickFolder,
  onAddFolder,
  folders,
  onRemoveFolder,
}) => {
  return (
    <>
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2.5">
        {/* Filter Tabs */}
        <div className="flex items-center rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-1 text-[11px] gap-1">
          {[
            { id: 'all' as const, label: 'All', count: counts.all },
            { id: 'loaded' as const, label: 'In VRAM', count: counts.loaded },
            { id: 'gguf' as const, label: 'Metal GGUF', count: counts.gguf },
            { id: 'lmstudio' as const, label: 'LM Studio', count: counts.lmstudio },
          ].map((f) => (
            <button
              key={f.id}
              type="button"
              onClick={() => setFilterTab(f.id)}
              className={`flex items-center gap-1.5 rounded-lg px-2.5 py-1 transition cursor-pointer ${
                filterTab === f.id
                  ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium shadow-xs border border-[var(--color-border-emphasis)]'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-1)] border border-transparent'
              }`}
            >
              <span>{f.label}</span>
              <span
                className={`text-[10px] font-mono px-1.5 py-0.2 rounded-full ${
                  filterTab === f.id
                    ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                    : 'bg-[var(--color-surface-2)] text-[var(--color-fg-subtle)]'
                }`}
              >
                {f.count}
              </span>
            </button>
          ))}
        </div>

        <div className="flex items-center gap-2 self-end sm:self-auto">
          <button
            type="button"
            onClick={onScanFolders}
            disabled={scanningFolders}
            className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] transition disabled:opacity-50 cursor-pointer"
          >
            <RefreshCw className={`size-3 ${scanningFolders ? 'animate-spin' : ''}`} />
            {scanningFolders ? 'Scanning…' : 'Scan'}
          </button>
          <button
            type="button"
            onClick={() => setShowAddFolder((prev) => !prev)}
            className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] transition cursor-pointer"
          >
            <FolderPlus className="size-3" /> Add Folder
          </button>
        </div>
      </div>

      {/* Add Folder Form */}
      {showAddFolder && (
        <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3.5 space-y-2.5 text-xs shadow-2xs">
          <span className="font-semibold text-[var(--color-editor-fg)]">Register Model Search Directory</span>
          <div className="flex gap-2">
            <input
              type="text"
              placeholder="/path/to/gguf/models"
              value={newFolderPath}
              onChange={(e) => setNewFolderPath(e.target.value)}
              className="flex-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
            />
            <button
              type="button"
              onClick={onPickFolder}
              className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] cursor-pointer"
            >
              Browse…
            </button>
          </div>
          <div className="flex justify-end gap-2 pt-1">
            <button
              type="button"
              onClick={() => setShowAddFolder(false)}
              className="rounded-lg px-3 py-1 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={onAddFolder}
              disabled={!newFolderPath.trim()}
              className="rounded-lg bg-[var(--color-editor-fg)] px-3.5 py-1 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] disabled:opacity-50 cursor-pointer shadow-xs"
            >
              Save Directory
            </button>
          </div>
        </div>
      )}

      {/* Folders List */}
      {folders.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {folders.map((f) => (
            <span
              key={f.id}
              className="inline-flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1 text-[10px] text-[var(--color-fg-muted)] font-mono shadow-2xs"
            >
              <Folder className="size-3 text-[var(--color-fg-subtle)]" />
              <span className="truncate max-w-[200px]" title={f.folder_path}>{f.folder_path}</span>
              <button
                type="button"
                onClick={() => onRemoveFolder(f.id)}
                className="p-0.5 rounded hover:bg-[var(--color-surface-2)] hover:text-rose-400 transition cursor-pointer"
                title="Remove directory"
              >
                <X className="size-2.5" />
              </button>
            </span>
          ))}
        </div>
      )}
    </>
  );
};
