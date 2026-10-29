import React, { useState, useMemo, useEffect } from 'react';
import {
  Folder,
  FolderOpen,
  FolderTree,
  Filter,
  RefreshCw,
  PanelLeftClose,
  Search,
  ChevronRight,
  ChevronDown,
} from 'lucide-react';
import { getFileIcon } from '@/utils/fileIcons';
import type { WorkspaceFileEntry, WorkspaceDiffEntry } from '@/types/domain';

export interface TreeNode {
  entry?: WorkspaceFileEntry;
  name: string;
  path: string;
  isDir: boolean;
  children: Map<string, TreeNode>;
}

export function checkIsDir(entry?: WorkspaceFileEntry): boolean {
  if (!entry) return false;
  return Boolean(
    (entry as any).isDir ??
    (entry as any).is_dir
  );
}

export function buildFileTree(entries: WorkspaceFileEntry[]): TreeNode {
  const root: TreeNode = {
    name: 'root',
    path: '',
    isDir: true,
    children: new Map(),
  };

  for (const entry of entries) {
    const parts = entry.path.split('/').filter(Boolean);
    let current = root;
    let currentPath = '';

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      currentPath = currentPath ? `${currentPath}/${part}` : part;
      const isLast = i === parts.length - 1;
      const isDir = !isLast || checkIsDir(entry);

      if (!current.children.has(part)) {
        current.children.set(part, {
          entry: isLast ? entry : undefined,
          name: part,
          path: currentPath,
          isDir,
          children: new Map(),
        });
      }

      current = current.children.get(part)!;
    }
  }

  return root;
}

export interface FileTreeSidebarProps {
  fileEntries: WorkspaceFileEntry[];
  changedFilesMap: Map<string, WorkspaceDiffEntry>;
  folderChangeCounts: Map<string, number>;
  selectedPath: string | null;
  onSelectPath: (path: string) => void;
  onRefresh: () => void;
  onClose: () => void;
  loading: boolean;
}

export const FileTreeSidebar: React.FC<FileTreeSidebarProps> = ({
  fileEntries,
  changedFilesMap,
  folderChangeCounts,
  selectedPath,
  onSelectPath,
  onRefresh,
  onClose,
  loading,
}) => {
  const [filterQuery, setFilterQuery] = useState('');
  const [onlyChanged, setOnlyChanged] = useState(false);
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(() => new Set());

  // Keep first two directory levels visible on fileEntries update
  useEffect(() => {
    const initialExpanded = new Set<string>();
    fileEntries.forEach((f) => {
      const parts = f.path.split('/').filter(Boolean);
      const directoryParts = checkIsDir(f) ? parts : parts.slice(0, -1);
      for (let depth = 1; depth <= Math.min(2, directoryParts.length); depth++) {
        initialExpanded.add(directoryParts.slice(0, depth).join('/'));
      }
    });
    setExpandedDirs(initialExpanded);
  }, [fileEntries]);

  const toggleDir = (dirPath: string) => {
    setExpandedDirs((prev) => {
      const next = new Set(prev);
      if (next.has(dirPath)) next.delete(dirPath);
      else next.add(dirPath);
      return next;
    });
  };

  const filteredEntries = useMemo(() => {
    return fileEntries.filter((entry) => {
      if (onlyChanged) {
        if (checkIsDir(entry)) {
          const count = folderChangeCounts.get(entry.path) || 0;
          if (count === 0) return false;
        } else if (!changedFilesMap.has(entry.path)) {
          return false;
        }
      }
      if (!filterQuery) return true;
      return entry.path.toLowerCase().includes(filterQuery.toLowerCase());
    });
  }, [fileEntries, onlyChanged, changedFilesMap, folderChangeCounts, filterQuery]);

  const treeRoot = useMemo(() => buildFileTree(filteredEntries), [filteredEntries]);

  const renderTree = (node: TreeNode, depth = 0): React.ReactNode => {
    const isExpanded = expandedDirs.has(node.path);
    const hasDiff = changedFilesMap.get(node.path);
    const isSelected = selectedPath === node.path;
    const folderChangeCount = node.isDir ? folderChangeCounts.get(node.path) || 0 : 0;

    const sortedChildren = Array.from(node.children.values()).sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    });

    if (node.path === '') {
      return (
        <div className="space-y-0.5">
          {sortedChildren.map((child) => renderTree(child, 0))}
        </div>
      );
    }

    if (node.isDir) {
      if (onlyChanged && folderChangeCount === 0) {
        return null;
      }

      return (
        <div key={node.path}>
          <button
            type="button"
            aria-expanded={isExpanded}
            onClick={() => toggleDir(node.path)}
            style={{ paddingLeft: `${depth * 12 + 6}px` }}
            className="flex w-full items-center justify-between py-1 px-1.5 text-left text-xs transition-colors rounded cursor-pointer select-none text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent group"
          >
            <div className="flex items-center gap-1.5 min-w-0 pr-2">
              <span className="p-0.5 text-[var(--color-fg-subtle)]">
                {isExpanded ? (
                  <ChevronDown className="h-3 w-3" />
                ) : (
                  <ChevronRight className="h-3 w-3" />
                )}
              </span>
              {isExpanded ? (
                <FolderOpen className="h-3.5 w-3.5 shrink-0 text-white/60" />
              ) : (
                <Folder className="h-3.5 w-3.5 shrink-0 text-white/40" />
              )}
              <span className="truncate">{node.name}</span>
            </div>
            {folderChangeCount > 0 && (
              <span className="text-[10px] font-mono font-medium text-white/60 mr-1">
                {folderChangeCount}
              </span>
            )}
          </button>
          {isExpanded && (
            <div>
              {sortedChildren.map((child) => renderTree(child, depth + 1))}
            </div>
          )}
        </div>
      );
    }

    let nameColorClass = 'text-[var(--color-editor-fg)]';
    let statusLetter = '';
    let statusColor = '';

    if (hasDiff) {
      if (hasDiff.status === 'added' || hasDiff.status === '??') {
        nameColorClass = 'text-white font-medium';
        statusLetter = hasDiff.status === '??' ? 'U' : 'A';
        statusColor = 'text-white/70';
      } else if (hasDiff.status === 'deleted') {
        nameColorClass = 'text-white/40 line-through';
        statusLetter = 'D';
        statusColor = 'text-white/40';
      } else {
        nameColorClass = 'text-white font-medium';
        statusLetter = 'M';
        statusColor = 'text-white/70';
      }
    }

    return (
      <div
        key={node.path}
        role="button"
        tabIndex={0}
        onClick={() => onSelectPath(node.path)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onSelectPath(node.path);
          }
        }}
        style={{ paddingLeft: `${depth * 12 + 18}px` }}
        className={`flex w-full items-center justify-between py-1 px-1.5 text-left text-xs transition-colors rounded cursor-pointer select-none group ${
          isSelected
            ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
            : 'hover:bg-[var(--color-surface-1)]'
        }`}
      >
        <div className="flex items-center gap-1.5 min-w-0 pr-2">
          {getFileIcon(node.name)}
          <span className={`truncate ${nameColorClass}`}>{node.name}</span>
        </div>
        {statusLetter && (
          <span className={`mr-1 shrink-0 font-mono text-[10px] font-bold ${statusColor}`}>
            {statusLetter}
          </span>
        )}
      </div>
    );
  };

  return (
    <div className="wb-master w-60 shrink-0 border-r border-[var(--color-border-subtle)] flex flex-col bg-[var(--color-surface-1)]/30 z-10 transition-all">
      {/* Explorer Header */}
      <div className="flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-2.5 bg-[var(--color-surface-1)]/50">
        <div className="flex items-center gap-1.5 truncate">
          <FolderTree className="h-3.5 w-3.5 text-white/60 shrink-0" />
          <span className="text-xs font-semibold text-[var(--color-editor-fg)] truncate">
            Files Navigator
          </span>
          {changedFilesMap.size > 0 && (
            <span className="text-[10px] font-mono text-white/60 font-medium">
              ({changedFilesMap.size})
            </span>
          )}
        </div>

        <div className="flex items-center gap-0.5">
          <button
            type="button"
            onClick={() => setOnlyChanged(!onlyChanged)}
            className={`px-1.5 py-0.5 rounded text-[10px] font-medium transition flex items-center gap-1 ${
              onlyChanged
                ? 'bg-white/10 text-white border border-white/15'
                : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
            }`}
            title={onlyChanged ? 'Show all files' : 'Show only changed files'}
          >
            <Filter className="h-3 w-3" />
            <span>{onlyChanged ? 'Changed' : 'All'}</span>
          </button>
          <button
            type="button"
            onClick={onRefresh}
            className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
            title="Refresh files"
          >
            <RefreshCw className="h-3 w-3" />
          </button>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
            title="Close files navigator"
          >
            <PanelLeftClose className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {/* Filter Box */}
      <div className="p-1.5 border-b border-[var(--color-border-subtle)]">
        <div className="relative">
          <Search className="absolute left-2 top-1.5 h-3 w-3 text-[var(--color-fg-subtle)]" />
          <input
            type="text"
            placeholder="Filter files..."
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            className="w-full rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] pl-6 pr-2 py-0.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-white/20"
          />
        </div>
      </div>

      {/* Tree */}
      <div className="flex-1 overflow-y-auto p-1 font-mono text-xs">
        {renderTree(treeRoot)}
        {fileEntries.length === 0 && !loading && (
          <div className="p-4 text-center text-xs text-[var(--color-fg-subtle)]">
            No files found.
          </div>
        )}
      </div>
    </div>
  );
};
