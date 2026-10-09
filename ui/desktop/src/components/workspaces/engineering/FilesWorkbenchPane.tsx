import React, { useState, useEffect, useMemo, useCallback } from 'react';
import {
  FolderTree,
  Folder,
  FolderOpen,
  File,
  FileCode,
  FileText,
  RefreshCw,
  Save,
  Search,
  AlertCircle,
  CheckCircle2,
  ChevronRight,
  ChevronDown,
  GitBranch,
  Lock,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { ExecutionWorkspace, WorkspaceFileEntry, WorkspaceFileContent } from '@/types/domain';
import { formatKeyCombo } from '@/lib/utils';

function getFileIcon(name: string) {
  const ext = name.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'ts':
    case 'tsx':
    case 'js':
    case 'jsx':
    case 'rs':
    case 'py':
    case 'go':
    case 'c':
    case 'cpp':
    case 'html':
    case 'css':
      return <FileCode className="h-3.5 w-3.5 text-blue-400 shrink-0" />;
    case 'json':
    case 'yaml':
    case 'yml':
    case 'toml':
      return <FileText className="h-3.5 w-3.5 text-amber-400 shrink-0" />;
    case 'md':
    case 'txt':
      return <FileText className="h-3.5 w-3.5 text-emerald-400 shrink-0" />;
    default:
      return <File className="h-3.5 w-3.5 text-[var(--color-fg-muted)] shrink-0" />;
  }
}

interface TreeNode {
  entry?: WorkspaceFileEntry;
  name: string;
  path: string;
  isDir: boolean;
  children: Map<string, TreeNode>;
}

function buildFileTree(entries: WorkspaceFileEntry[]): TreeNode {
  const root: TreeNode = {
    name: 'root',
    path: '',
    isDir: true,
    children: new Map(),
  };

  for (const entry of entries) {
    const parts = entry.path.split('/').filter(Boolean);
    let current = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      const nodePath = parts.slice(0, i + 1).join('/');

      if (!current.children.has(part)) {
        current.children.set(part, {
          entry: isLast ? entry : undefined,
          name: part,
          path: nodePath,
          isDir: isLast ? entry.is_dir : true,
          children: new Map(),
        });
      }
      current = current.children.get(part)!;
    }
  }

  return root;
}

export const FilesWorkbenchPane: React.FC = () => {
  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>('');
  const [loadingWorkspaces, setLoadingWorkspaces] = useState<boolean>(true);

  // File tree state
  const [entries, setEntries] = useState<WorkspaceFileEntry[]>([]);
  const [loadingTree, setLoadingTree] = useState<boolean>(false);
  const [searchFilter, setSearchFilter] = useState<string>('');
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());

  // Active file editor state
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState<WorkspaceFileContent | null>(null);
  const [editorText, setEditorText] = useState<string>('');
  const [isDirty, setIsDirty] = useState<boolean>(false);
  const [loadingFile, setLoadingFile] = useState<boolean>(false);
  const [savingFile, setSavingFile] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [successToast, setSuccessToast] = useState<string | null>(null);

  const fetchWorkspaces = useCallback(async () => {
    try {
      setLoadingWorkspaces(true);
      const list = await daemonClient.listWorkspaces();
      const activeList = list.filter((w) => w.status !== 'archived');
      setWorkspaces(activeList);
      if (activeList.length > 0 && !selectedWorkspaceId) {
        setSelectedWorkspaceId(activeList[0].id);
      }
    } catch (e: any) {
      setErrorMessage(e?.message || 'Failed to list execution workspaces');
    } finally {
      setLoadingWorkspaces(false);
    }
  }, [selectedWorkspaceId]);

  useEffect(() => {
    fetchWorkspaces();
  }, [fetchWorkspaces]);

  const loadTree = useCallback(async () => {
    if (!selectedWorkspaceId) return;
    try {
      setLoadingTree(true);
      setErrorMessage(null);
      const tree = await daemonClient.getWorkspaceFileTree(selectedWorkspaceId);
      setEntries(tree.entries || []);
      // Auto-expand top-level directories
      const topDirs = new Set<string>();
      (tree.entries || []).forEach((e) => {
        if (e.is_dir && !e.path.includes('/')) {
          topDirs.add(e.path);
        }
      });
      setExpandedDirs((prev) => new Set([...prev, ...topDirs]));
    } catch (e: any) {
      setErrorMessage(e?.message || 'Failed to read workspace file tree');
      setEntries([]);
    } finally {
      setLoadingTree(false);
    }
  }, [selectedWorkspaceId]);

  useEffect(() => {
    if (selectedWorkspaceId) {
      setActiveFilePath(null);
      setFileContent(null);
      setEditorText('');
      setIsDirty(false);
      loadTree();
    }
  }, [selectedWorkspaceId, loadTree]);

  const openFile = async (path: string) => {
    if (!selectedWorkspaceId) return;
    if (isDirty) {
      const confirmDiscard = window.confirm(
        'You have unsaved changes in the current file. Discard changes and open another file?'
      );
      if (!confirmDiscard) return;
    }

    try {
      setLoadingFile(true);
      setErrorMessage(null);
      setActiveFilePath(path);
      const res = await daemonClient.readWorkspaceFile(selectedWorkspaceId, path);
      setFileContent(res);
      setEditorText(res.content);
      setIsDirty(false);
    } catch (e: any) {
      setErrorMessage(e?.message || `Failed to read file ${path}`);
    } finally {
      setLoadingFile(false);
    }
  };

  const handleSaveFile = async () => {
    if (!selectedWorkspaceId || !activeFilePath) return;
    try {
      setSavingFile(true);
      setErrorMessage(null);
      const res = await daemonClient.writeWorkspaceFile({
        workspaceId: selectedWorkspaceId,
        path: activeFilePath,
        content: editorText,
        overwrite: true,
      });
      setFileContent(res);
      setIsDirty(false);
      setSuccessToast(`Saved ${activeFilePath}`);
      setTimeout(() => setSuccessToast(null), 3000);
    } catch (e: any) {
      setErrorMessage(e?.message || `Failed to save file ${activeFilePath}`);
    } finally {
      setSavingFile(false);
    }
  };

  // Keyboard shortcut Ctrl+S / Cmd+S
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        if (activeFilePath && isDirty) {
          handleSaveFile();
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeFilePath, isDirty, editorText, selectedWorkspaceId]);

  const toggleDir = (dirPath: string) => {
    setExpandedDirs((prev) => {
      const next = new Set(prev);
      if (next.has(dirPath)) {
        next.delete(dirPath);
      } else {
        next.add(dirPath);
      }
      return next;
    });
  };

  const filteredEntries = useMemo(() => {
    if (!searchFilter.trim()) return entries;
    const q = searchFilter.toLowerCase();
    return entries.filter(
      (e) => e.path.toLowerCase().includes(q) || e.name.toLowerCase().includes(q)
    );
  }, [entries, searchFilter]);

  const treeRoot = useMemo(() => {
    return buildFileTree(filteredEntries);
  }, [filteredEntries]);

  const renderTreeNode = (node: TreeNode, depth: number) => {
    const isExpanded = expandedDirs.has(node.path);
    const isSelected = activeFilePath === node.path;

    if (node.isDir) {
      return (
        <div key={node.path} className="select-none">
          <div
            onClick={() => toggleDir(node.path)}
            style={{ paddingLeft: `${depth * 12 + 8}px` }}
            className="flex items-center gap-1.5 py-1 pr-2 hover:bg-[var(--color-surface-2)] cursor-pointer rounded text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
          >
            {isExpanded ? (
              <ChevronDown className="h-3 w-3 shrink-0" />
            ) : (
              <ChevronRight className="h-3 w-3 shrink-0" />
            )}
            {isExpanded ? (
              <FolderOpen className="h-3.5 w-3.5 text-amber-400 shrink-0" />
            ) : (
              <Folder className="h-3.5 w-3.5 text-amber-400 shrink-0" />
            )}
            <span className="truncate font-medium">{node.name}</span>
          </div>
          {isExpanded && (
            <div>
              {Array.from(node.children.values())
                .sort((a, b) => b.isDir ? 1 : a.isDir ? -1 : a.name.localeCompare(b.name))
                .map((child) => renderTreeNode(child, depth + 1))}
            </div>
          )}
        </div>
      );
    }

    return (
      <div
        key={node.path}
        onClick={() => openFile(node.path)}
        style={{ paddingLeft: `${depth * 12 + 18}px` }}
        className={`flex items-center gap-1.5 py-1 pr-2 cursor-pointer rounded text-[11px] transition select-none ${
          isSelected
            ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium border-l-2 border-emerald-500'
            : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-1)] hover:text-[var(--color-editor-fg)]'
        }`}
      >
        {getFileIcon(node.name)}
        <span className="truncate flex-1">{node.name}</span>
        {node.entry?.is_readonly && (
          <Lock className="h-2.5 w-2.5 text-[var(--color-fg-subtle)]" />
        )}
      </div>
    );
  };

  const activeWorkspace = workspaces.find((w) => w.id === selectedWorkspaceId);

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Top Bar: Workspace Selector & Stats */}
      <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-4 py-2.5">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <FolderTree className="h-4 w-4 text-[var(--color-fg-muted)]" />
            <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
              Files
            </span>
          </div>

          <div className="h-4 w-[1px] bg-[var(--color-border-subtle)]" />

          {/* Workspace dropdown */}
          <div className="flex items-center gap-2">
            <label className="text-[11px] text-[var(--color-fg-subtle)]">Workspace:</label>
            <select
              value={selectedWorkspaceId}
              onChange={(e) => setSelectedWorkspaceId(e.target.value)}
              disabled={loadingWorkspaces}
              className="rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:ring-1 focus:ring-zinc-600"
            >
              {workspaces.map((ws) => (
                <option key={ws.id} value={ws.id}>
                  {ws.name} ({ws.kind.type === 'git' ? 'Git' : 'Folder'})
                </option>
              ))}
            </select>
          </div>

          {activeWorkspace && (
            <div className="hidden lg:flex items-center gap-1.5 text-[11px] text-[var(--color-fg-subtle)] font-mono">
              <span className="truncate max-w-[280px]">{activeWorkspace.path}</span>
              {activeWorkspace.kind.type === 'git' && (
                <span className="inline-flex items-center gap-1 rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] text-emerald-400">
                  <GitBranch className="h-3 w-3" />
                  {activeWorkspace.kind.branch}
                </span>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center gap-2">
          {successToast && (
            <span className="flex items-center gap-1 text-[11px] text-emerald-400">
              <CheckCircle2 className="h-3.5 w-3.5" />
              {successToast}
            </span>
          )}
          {errorMessage && (
            <span className="flex items-center gap-1 text-[11px] text-red-400">
              <AlertCircle className="h-3.5 w-3.5" />
              {errorMessage}
            </span>
          )}
          <button
            onClick={loadTree}
            disabled={loadingTree || !selectedWorkspaceId}
            title="Refresh File Tree"
            className="flex items-center gap-1 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-1 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition disabled:opacity-50"
          >
            <RefreshCw className={`h-3 w-3 ${loadingTree ? 'animate-spin' : ''}`} />
            <span>Refresh</span>
          </button>
        </div>
      </div>

      {/* Main Container: Split View */}
      <div className="flex flex-1 min-h-0">
        {/* Left Explorer Pane */}
        <div className="w-64 border-r border-[var(--color-border-muted)] bg-[var(--color-surface-1)]/50 flex flex-col">
          {/* Search box */}
          <div className="p-2 border-b border-[var(--color-border-subtle)]">
            <div className="relative">
              <Search className="absolute left-2 top-2 h-3 w-3 text-[var(--color-fg-subtle)]" />
              <input
                type="text"
                placeholder="Search files..."
                value={searchFilter}
                onChange={(e) => setSearchFilter(e.target.value)}
                className="w-full rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] pl-7 pr-2 py-1 text-xs text-[var(--color-editor-fg)] placeholder-[var(--color-fg-subtle)] focus:outline-none focus:ring-1 focus:ring-zinc-600"
              />
            </div>
          </div>

          {/* File Tree List */}
          <div className="flex-1 overflow-y-auto p-1 text-xs">
            {loadingTree ? (
              <div className="flex items-center justify-center p-6 text-[11px] text-[var(--color-fg-muted)]">
                <RefreshCw className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                Scanning repository...
              </div>
            ) : entries.length === 0 ? (
              <div className="p-6 text-center text-[11px] text-[var(--color-fg-muted)]">
                No files found in workspace.
              </div>
            ) : (
              <div>
                {Array.from(treeRoot.children.values())
                  .sort((a, b) => b.isDir ? 1 : a.isDir ? -1 : a.name.localeCompare(b.name))
                  .map((child) => renderTreeNode(child, 0))}
              </div>
            )}
          </div>
        </div>

        {/* Right Editor / Viewer Pane */}
        <div className="flex-1 flex flex-col bg-[var(--color-canvas)]">
          {activeFilePath ? (
            <>
              {/* File Editor Header */}
              <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-4 py-2">
                <div className="flex items-center gap-2">
                  {getFileIcon(activeFilePath)}
                  <span className="font-mono text-xs font-semibold text-[var(--color-editor-fg)]">
                    {activeFilePath}
                  </span>
                  {isDirty && (
                    <span className="h-1.5 w-1.5 rounded-full bg-amber-400" title="Unsaved changes" />
                  )}
                  {fileContent && (
                    <span className="text-[10px] text-[var(--color-fg-subtle)]">
                      ({fileContent.line_count} lines • {(fileContent.size_bytes / 1024).toFixed(1)} KB)
                    </span>
                  )}
                </div>

                <div className="flex items-center gap-2">
                  <button
                    onClick={handleSaveFile}
                    disabled={savingFile || !isDirty}
                    className={`flex items-center gap-1.5 rounded px-2.5 py-1 text-xs font-medium transition ${
                      isDirty
                        ? 'bg-emerald-600 text-white hover:bg-emerald-500 shadow-sm'
                        : 'border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] opacity-50 cursor-not-allowed'
                    }`}
                  >
                    <Save className="h-3 w-3" />
                    <span>{savingFile ? 'Saving...' : `Save ${formatKeyCombo({ ctrlOrCmd: true, key: 'S' })}`}</span>
                  </button>
                </div>
              </div>

              {/* Editor Content Body */}
              <div className="flex-1 relative overflow-hidden flex flex-col">
                {loadingFile ? (
                  <div className="flex flex-1 items-center justify-center text-xs text-[var(--color-fg-muted)]">
                    <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                    Loading {activeFilePath}...
                  </div>
                ) : fileContent?.is_binary ? (
                  <div className="flex flex-1 flex-col items-center justify-center p-8 text-center">
                    <File className="h-10 w-10 text-[var(--color-fg-muted)] mb-3" />
                    <h3 className="text-xs font-semibold">Binary File Detected</h3>
                    <p className="mt-1 text-[11px] text-[var(--color-fg-muted)] max-w-sm">
                      This file cannot be rendered as text in the editor.
                    </p>
                  </div>
                ) : (
                  <textarea
                    value={editorText}
                    onChange={(e) => {
                      setEditorText(e.target.value);
                      setIsDirty(true);
                    }}
                    spellCheck={false}
                    className="flex-1 w-full resize-none bg-[var(--color-canvas)] p-4 font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] focus:outline-none selection:bg-zinc-800"
                  />
                )}
              </div>
            </>
          ) : (
            <div className="flex flex-1 flex-col items-center justify-center p-8 text-center text-[var(--color-fg-muted)]">
              <FolderTree className="h-12 w-12 stroke-[1.2] opacity-40 mb-3" />
              <h3 className="text-xs font-semibold text-[var(--color-editor-fg)]">
                No File Open
              </h3>
              <p className="mt-1 text-[11px] max-w-sm">
                Select a file from the explorer on the left to inspect and edit within the execution workspace.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
