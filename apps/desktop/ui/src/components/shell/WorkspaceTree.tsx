import React, { useState } from 'react';
import {
  FolderGit2,
  FolderOpen,
  GitBranch,
  Plus,
  ChevronRight,
  ChevronDown,
  Check,
  MoreVertical,
  Trash2,
  Terminal,
} from 'lucide-react';
import type { WorkspaceNode, WorkspaceDetectionResult } from '@/types/domain';

export interface WorkspaceTreeProps {
  nodes: WorkspaceNode[];
  activeId: string;
  onSelectNode: (id: string) => void;
  onAddFolder: () => void;
  onAddWorktree?: (parentId: string, branchName: string) => void | Promise<void>;
  onRemoveNode?: (id: string) => void;
  activeDetection?: WorkspaceDetectionResult | null;
}

export const WorkspaceTree: React.FC<WorkspaceTreeProps> = ({
  nodes,
  activeId,
  onSelectNode,
  onAddFolder,
  onAddWorktree,
  onRemoveNode,
  activeDetection: _activeDetection,
}) => {
  const [expandedNodes, setExpandedNodes] = useState<Record<string, boolean>>({});
  const [creatingWorktreeFor, setCreatingWorktreeFor] = useState<string | null>(null);
  const [worktreeBranchName, setWorktreeBranchName] = useState('');
  const [activeMenuNodeId, setActiveMenuNodeId] = useState<string | null>(null);

  const toggleExpand = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setExpandedNodes((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const handleStartWorktree = (nodeId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setCreatingWorktreeFor(nodeId);
    setWorktreeBranchName('');
    setExpandedNodes((prev) => ({ ...prev, [nodeId]: true }));
  };

  const handleCreateWorktreeSubmit = async (e: React.FormEvent, parentId: string) => {
    e.preventDefault();
    const branch = worktreeBranchName.trim();
    if (!branch || !onAddWorktree) return;
    await onAddWorktree(parentId, branch);
    setCreatingWorktreeFor(null);
    setWorktreeBranchName('');
  };

  return (
    <div className="mb-2.5">
      {/* Header */}
      <div className="flex items-center justify-between px-1 py-1 text-[10px] font-semibold text-fg-subtle uppercase tracking-wider mb-1">
        <span className="flex items-center gap-1.5">
          <FolderGit2 className="w-3.5 h-3.5 text-fg-muted" />
          <span>Workspaces</span>
        </span>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={onAddFolder}
            className="p-1 rounded text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition flex items-center gap-0.5 normal-case font-medium"
            title="Open Workspace / Add Folder..."
            aria-label="Add Workspace"
          >
            <Plus className="w-3 h-3 text-fg-muted" />
            <span className="text-[10px] text-fg-muted">Add</span>
          </button>
        </div>
      </div>

      {/* Nodes list */}
      <div className="space-y-1 max-h-56 overflow-y-auto no-scrollbar pr-0.5">
        {nodes.length === 0 ? (
          <button
            type="button"
            onClick={onAddFolder}
            className="w-full flex items-center justify-center gap-2 p-2 rounded-lg border border-dashed border-border-default hover:border-border-muted text-xs text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
          >
            <FolderOpen className="w-3.5 h-3.5 text-fg-muted" />
            <span>Open Workspace</span>
          </button>
        ) : (
          nodes.map((node) => {
            const isNodeActive = node.id === activeId || node.isActive;
            const hasWorktrees = node.worktrees && node.worktrees.length > 0;
            const isExpanded = expandedNodes[node.id] ?? true;
            const isCreating = creatingWorktreeFor === node.id;
            const isGit = node.kind === 'git';

            return (
              <div key={node.id} className="group/node rounded-lg overflow-hidden">
                {/* Workspace Row */}
                <div
                  onClick={() => onSelectNode(node.id)}
                  className={`w-full flex items-center justify-between px-2 py-1.5 rounded-lg text-xs transition cursor-pointer ${
                    isNodeActive
                      ? 'bg-surface-2 text-fg-editor font-medium border border-border-default shadow-xs'
                      : 'text-fg-muted hover:text-fg-editor hover:bg-surface-2 border border-transparent'
                  }`}
                  title={`${node.name} (${node.path})`}
                >
                  <div className="flex items-center gap-1.5 truncate min-w-0 flex-1">
                    {/* Expand/Collapse Toggle */}
                    {hasWorktrees ? (
                      <button
                        type="button"
                        onClick={(e) => toggleExpand(node.id, e)}
                        className="p-0.5 text-fg-subtle hover:text-fg-editor transition shrink-0"
                        title={isExpanded ? 'Collapse worktrees' : 'Expand worktrees'}
                      >
                        {isExpanded ? (
                          <ChevronDown className="w-3 h-3" />
                        ) : (
                          <ChevronRight className="w-3 h-3" />
                        )}
                      </button>
                    ) : (
                      <span className="w-3 shrink-0" />
                    )}

                    {/* Kind Icon */}
                    {node.kind === 'ssh' ? (
                      <Terminal className="w-3.5 h-3.5 text-fg-muted shrink-0" />
                    ) : isGit ? (
                      <FolderGit2 className="w-3.5 h-3.5 text-fg-muted shrink-0" />
                    ) : (
                      <FolderOpen className="w-3.5 h-3.5 text-fg-muted shrink-0" />
                    )}

                    {/* Workspace Name & Git Info */}
                    <div className="flex flex-col min-w-0">
                      <span className="truncate leading-tight text-[12px]">{node.name}</span>
                      {node.branch && (
                        <div className="flex items-center gap-1 text-[9.5px] font-mono text-fg-subtle leading-tight mt-0.5">
                          <GitBranch className="w-2.5 h-2.5 text-fg-subtle shrink-0" />
                          <span className="truncate max-w-[80px] text-fg-muted">{node.branch}</span>
                          {node.isDirty && (
                            <span className="text-fg-subtle font-medium shrink-0">
                              • {node.dirtyCount ? `${node.dirtyCount}*` : 'dirty'}
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                  </div>

                  {/* Actions & Status */}
                  <div className="flex items-center gap-1 shrink-0 ml-1">
                    {isNodeActive && (
                      <Check className="w-3.5 h-3.5 workbench-accent shrink-0" />
                    )}

                    {/* Worktree + button */}
                    {isGit && onAddWorktree && (
                      <button
                        type="button"
                        onClick={(e) => handleStartWorktree(node.id, e)}
                        className="p-1 rounded text-fg-subtle hover:text-fg-editor hover:bg-surface-3 transition hidden group-hover/node:flex"
                        title="New isolated Git worktree"
                      >
                        <Plus className="w-3 h-3" />
                      </button>
                    )}

                    {/* Options Menu */}
                    {onRemoveNode && (
                      <div className="relative">
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            setActiveMenuNodeId((curr) => (curr === node.id ? null : node.id));
                          }}
                          className={`p-1 rounded text-fg-subtle hover:text-fg-editor hover:bg-surface-3 transition ${
                            activeMenuNodeId === node.id ? 'flex bg-surface-3 text-fg-editor' : 'hidden group-hover/node:flex'
                          }`}
                          title="Workspace actions"
                        >
                          <MoreVertical className="w-3 h-3" />
                        </button>
                        {activeMenuNodeId === node.id && (
                          <div
                            onClick={(e) => e.stopPropagation()}
                            className="absolute right-0 top-full mt-1 w-36 rounded-lg bg-surface-1 border border-border-default shadow-xl p-1 z-50 text-xs animate-in fade-in zoom-in-95 duration-100"
                          >
                            <button
                              type="button"
                              onClick={() => {
                                setActiveMenuNodeId(null);
                                onRemoveNode(node.id);
                              }}
                              className="w-full flex items-center gap-1.5 px-2 py-1.5 rounded text-[var(--color-task-failed)] hover:bg-[rgba(248,81,73,0.12)] transition text-left text-[11px]"
                            >
                              <Trash2 className="w-3 h-3" />
                              <span>Remove from list</span>
                            </button>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                </div>

                {/* Inline form to create Worktree */}
                {isCreating && (
                  <form
                    onSubmit={(e) => handleCreateWorktreeSubmit(e, node.id)}
                    className="mt-1 ml-4 p-1.5 rounded-lg bg-surface-2 border border-border-default space-y-1.5"
                  >
                    <input
                      type="text"
                      value={worktreeBranchName}
                      onChange={(e) => setWorktreeBranchName(e.target.value)}
                      placeholder="branch-name (e.g. feat-auth)"
                      autoFocus
                      className="w-full bg-surface-1 border border-border-default rounded px-2 py-1 text-xs text-fg-editor placeholder:text-fg-subtle outline-none font-mono"
                    />
                    <div className="flex justify-end gap-1.5">
                      <button
                        type="button"
                        onClick={() => {
                          setCreatingWorktreeFor(null);
                          setWorktreeBranchName('');
                        }}
                        className="px-2 py-0.5 text-[10.5px] text-fg-muted hover:text-fg-editor"
                      >
                        Cancel
                      </button>
                      <button
                        type="submit"
                        disabled={!worktreeBranchName.trim()}
                        className="px-2 py-0.5 text-[10.5px] bg-surface-3 hover:bg-surface-0 border border-border-default text-fg-editor rounded font-medium disabled:opacity-50"
                      >
                        Create
                      </button>
                    </div>
                  </form>
                )}

                {/* Indented Worktrees Children */}
                {isExpanded && hasWorktrees && (
                  <div className="pl-3 border-l border-border-muted/60 ml-3 space-y-0.5 mt-0.5">
                    {node.worktrees!.map((wt) => {
                      const isWtActive = wt.id === activeId || wt.isActive;
                      return (
                        <div
                          key={wt.id}
                          onClick={() => onSelectNode(wt.id)}
                          className={`flex items-center justify-between px-2 py-1 rounded-md text-[11px] transition cursor-pointer ${
                            isWtActive
                              ? 'bg-surface-2 text-fg-editor font-medium border border-border-default'
                              : 'text-fg-muted hover:text-fg-editor hover:bg-surface-2 border border-transparent'
                          }`}
                          title={`${wt.name} (${wt.path})`}
                        >
                          <div className="flex items-center gap-1.5 truncate min-w-0">
                            <GitBranch className="w-3 h-3 text-fg-subtle shrink-0" />
                            <span className="truncate">{wt.branch || wt.name}</span>
                            {wt.isDirty && (
                              <span className="text-fg-subtle font-mono text-[9px]">• dirty</span>
                            )}
                          </div>
                          {isWtActive && <Check className="w-3 h-3 workbench-accent shrink-0 ml-1" />}
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
