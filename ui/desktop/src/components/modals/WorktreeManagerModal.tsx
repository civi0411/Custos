import React, { useState, useEffect } from 'react';
import { 
  X, 
  GitBranch, 
  FolderPlus, 
  Check, 
  Trash2, 
  ArrowRight, 
  ShieldCheck, 
  Server, 
  Terminal, 
  GitMerge, 
  Loader2 
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { ExecutionWorkspace } from '@/types/domain';

export interface ManagedWorktree {
  id: string;
  branch: string;
  path: string;
  baseCommit: string;
  host: 'local' | 'remote-ssh';
  status: 'create' | 'work' | 'review' | 'ship' | 'cleanup';
  modifiedFilesCount: number;
  assignedAgent: string;
  createdAt: string;
  isMain?: boolean;
}

interface WorktreeManagerModalProps {
  isOpen: boolean;
  onClose: () => void;
  activeWorktreeId: string;
  onSelectWorktree: (worktree: ManagedWorktree) => void;
}

const mapToManagedWorktree = (ws: ExecutionWorkspace): ManagedWorktree => {
  const branch = ws.kind.type === 'git' ? ws.kind.branch : ws.name;
  const isMain = branch === 'main';
  const host = ws.kind.type === 'remote_ssh' ? 'remote-ssh' : 'local';

  let status: ManagedWorktree['status'] = 'work';
  if (ws.status === 'initializing') status = 'create';
  else if (ws.status === 'archived') status = 'cleanup';

  return {
    id: ws.id,
    branch,
    path: ws.path,
    baseCommit: ws.lineage?.base_commit || 'a3f2d1e',
    host,
    status,
    modifiedFilesCount: ws.metadata?.modifiedFilesCount ?? 0,
    assignedAgent: ws.metadata?.assignedAgent ?? 'Claude Code',
    createdAt: isMain
      ? 'Main Checkout'
      : ws.created_at
      ? new Date(ws.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
      : 'Just now',
    isMain,
  };
};

export const WorktreeManagerModal: React.FC<WorktreeManagerModalProps> = ({
  isOpen,
  onClose,
  activeWorktreeId,
  onSelectWorktree,
}) => {
  const [worktrees, setWorktrees] = useState<ManagedWorktree[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Form State
  const [newBranch, setNewBranch] = useState('');
  const [newBaseCommit, setNewBaseCommit] = useState('');
  const [newHost, setNewHost] = useState<'local' | 'remote-ssh'>('local');
  const [runSetupScript, setRunSetupScript] = useState(true);

  useEffect(() => {
    if (!isOpen) return;
    setIsLoading(true);
    setErrorMsg(null);

    daemonClient
      .listWorkspaces()
      .then((records) => {
        const mapped = records.map(mapToManagedWorktree);
        if (!mapped.some((w) => w.isMain)) {
          mapped.unshift({
            id: 'ws-main',
            branch: 'main',
            path: '/Users/mac/Project/AgentHub/Custos',
            baseCommit: 'a3f2d1e',
            host: 'local',
            status: 'work',
            modifiedFilesCount: 0,
            assignedAgent: 'Custos Host',
            createdAt: 'Main Checkout',
            isMain: true,
          });
        }
        setWorktrees(mapped);
      })
      .catch((err) => {
        console.warn('Could not fetch workspaces from daemon:', err);
        setWorktrees([
          {
            id: 'ws-main',
            branch: 'main',
            path: '/Users/mac/Project/AgentHub/Custos',
            baseCommit: 'a3f2d1e',
            host: 'local',
            status: 'work',
            modifiedFilesCount: 0,
            assignedAgent: 'Custos Host',
            createdAt: 'Main Checkout',
            isMain: true,
          },
        ]);
      })
      .finally(() => setIsLoading(false));
  }, [isOpen]);

  if (!isOpen) return null;

  const handleCreateWorktree = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newBranch.trim()) return;

    setIsSubmitting(true);
    setErrorMsg(null);

    try {
      const sanitizedBranch = newBranch.trim();
      const created = await daemonClient.createWorkspace({
        name: sanitizedBranch,
        kind:
          newHost === 'remote-ssh'
            ? {
                type: 'remote_ssh',
                host: 'ssh.server',
                remote_path: `.worktrees/${sanitizedBranch}`,
              }
            : {
                type: 'git',
                repo_path: '/Users/mac/Project/AgentHub/Custos',
                branch: sanitizedBranch,
                base_commit: newBaseCommit.trim() || undefined,
              },
        path: `.worktrees/${sanitizedBranch.replace(/\//g, '-')}`,
        lineage: {
          base_commit: newBaseCommit.trim() || undefined,
          target_branch: sanitizedBranch,
        },
        metadata: {
          domain: 'engineering',
          assignedAgent: 'Claude Code',
        },
        setup_script: runSetupScript ? 'echo "Provisioned worktree"' : undefined,
      });

      const managed = mapToManagedWorktree(created);
      setWorktrees((prev) => [managed, ...prev]);
      onSelectWorktree(managed);
      setIsCreating(false);
      setNewBranch('');
    } catch (err: any) {
      console.error('Failed to create execution workspace:', err);
      setErrorMsg(err.message || 'Failed to create execution workspace');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handlePruneWorktree = async (id: string) => {
    try {
      await daemonClient.archiveWorkspace(id, true);
    } catch (err) {
      console.warn('Archive workspace error:', err);
    }
    setWorktrees((prev) => prev.filter((w) => w.id !== id));
  };

  const getStatusBadge = (status: ManagedWorktree['status']) => {
    switch (status) {
      case 'create':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-blue-500/10 text-blue-400 border border-blue-500/20">1. Setup</span>;
      case 'work':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 flex items-center gap-1"><span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>2. Work (Active)</span>;
      case 'review':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-amber-500/10 text-amber-500 border border-amber-500/20">3. Review Diff</span>;
      case 'ship':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-purple-500/10 text-purple-400 border border-purple-500/20">4. Ready to Ship</span>;
      case 'cleanup':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-surface-2 text-fg-subtle border border-border-muted">5. Cleanup</span>;
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150">
      <div 
        className="w-full max-w-2xl bg-surface-1 border border-border-default rounded-2xl overflow-hidden flex flex-col font-sans max-h-[85vh] shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 flex items-center justify-between border-b border-border-muted bg-surface-1">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg flex items-center justify-center bg-surface-2 border border-border-default text-emerald-500">
              <GitBranch className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-semibold text-fg-editor text-sm">Execution Workspaces</span>
                <span className="text-[10px] px-2 py-0.5 rounded-full font-mono text-emerald-500 bg-emerald-500/10 border border-emerald-500/20">
                  Isolated Worktrees
                </span>
              </div>
              <p className="text-xs text-fg-muted mt-0.5">
                Managed worktrees create isolated checkout directories preserving your main workspace.
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-5 text-xs text-fg-editor">
          {/* Worktree Lifecycle Progress Indicator */}
          <div className="p-3.5 rounded-xl flex items-center justify-between text-[11px] font-mono bg-surface-0 border border-border-muted">
            <div className="flex items-center gap-1.5 text-fg-muted">
              <span className="w-4 h-4 rounded-full bg-surface-2 text-fg-editor flex items-center justify-center text-[10px] font-bold">1</span>
              <span>Create</span>
            </div>
            <ArrowRight className="w-3 h-3 text-fg-subtle" />
            <div className="flex items-center gap-1.5 text-emerald-500 font-semibold">
              <span className="w-4 h-4 rounded-full bg-emerald-500/10 text-emerald-500 flex items-center justify-center text-[10px] font-bold">2</span>
              <span>Agent Work</span>
            </div>
            <ArrowRight className="w-3 h-3 text-fg-subtle" />
            <div className="flex items-center gap-1.5 text-fg-muted">
              <span className="w-4 h-4 rounded-full bg-surface-2 text-fg-editor flex items-center justify-center text-[10px] font-bold">3</span>
              <span>Review Diff</span>
            </div>
            <ArrowRight className="w-3 h-3 text-fg-subtle" />
            <div className="flex items-center gap-1.5 text-fg-muted">
              <span className="w-4 h-4 rounded-full bg-surface-2 text-fg-editor flex items-center justify-center text-[10px] font-bold">4</span>
              <span>Ship</span>
            </div>
            <ArrowRight className="w-3 h-3 text-fg-subtle" />
            <div className="flex items-center gap-1.5 text-fg-subtle">
              <span className="w-4 h-4 rounded-full bg-surface-2 text-fg-subtle flex items-center justify-center text-[10px] font-bold">5</span>
              <span>Cleanup</span>
            </div>
          </div>

          {/* Create New Worktree Section */}
          {isCreating ? (
            <form 
              onSubmit={handleCreateWorktree}
              className="p-4 rounded-xl space-y-3 bg-surface-0 border border-border-default"
            >
              <div className="flex items-center justify-between">
                <span className="font-semibold text-fg-editor flex items-center gap-1.5">
                  <FolderPlus className="w-4 h-4 workbench-accent" />
                  Create Managed Worktree
                </span>
                <button
                  type="button"
                  onClick={() => setIsCreating(false)}
                  className="text-xs text-fg-muted hover:text-fg-editor"
                >
                  Cancel
                </button>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-medium text-fg-muted mb-1">
                    Branch Name
                  </label>
                  <input
                    type="text"
                    value={newBranch}
                    onChange={(e) => setNewBranch(e.target.value)}
                    placeholder="feat/ast-invariant-cache"
                    required
                    className="w-full rounded-md px-3 py-1.5 font-mono text-xs text-fg-editor placeholder-fg-subtle bg-surface-1 border border-border-default focus:border-border-emphasis focus:outline-none"
                  />
                </div>

                <div>
                  <label className="block text-xs font-medium text-fg-muted mb-1">
                    Base Commit Hash
                  </label>
                  <input
                    type="text"
                    value={newBaseCommit}
                    onChange={(e) => setNewBaseCommit(e.target.value)}
                    placeholder="a3f2d1e"
                    className="w-full rounded-md px-3 py-1.5 font-mono text-xs text-fg-editor placeholder-fg-subtle bg-surface-1 border border-border-default focus:border-border-emphasis focus:outline-none"
                  />
                </div>
              </div>

              <div className="flex items-center justify-between pt-1">
                <div className="flex items-center gap-3">
                  <label className="flex items-center gap-1.5 cursor-pointer text-xs text-fg-muted">
                    <input
                      type="radio"
                      name="host"
                      checked={newHost === 'local'}
                      onChange={() => setNewHost('local')}
                      className="accent-[var(--workbench-accent)]"
                    />
                    <span>Local Host (macOS)</span>
                  </label>
                  <label className="flex items-center gap-1.5 cursor-pointer text-xs text-fg-muted">
                    <input
                      type="radio"
                      name="host"
                      checked={newHost === 'remote-ssh'}
                      onChange={() => setNewHost('remote-ssh')}
                      className="accent-[var(--workbench-accent)]"
                    />
                    <span>Remote SSH</span>
                  </label>
                </div>

                <label className="flex items-center gap-1.5 cursor-pointer text-xs text-fg-muted">
                  <input
                    type="checkbox"
                    checked={runSetupScript}
                    onChange={(e) => setRunSetupScript(e.target.checked)}
                    className="accent-[var(--workbench-accent)]"
                  />
                  <span>Run setup script</span>
                </label>
              </div>

              {errorMsg && (
                <div className="mt-2 text-xs px-2.5 py-1.5 rounded bg-rose-500/10 text-rose-500 border border-rose-500/20">
                  {errorMsg}
                </div>
              )}

              <div className="flex justify-end pt-2">
                <button
                  type="submit"
                  disabled={isSubmitting}
                  className="workbench-primary-action px-4 py-1.5 rounded-lg font-medium text-xs flex items-center gap-1.5 transition disabled:opacity-50"
                >
                  {isSubmitting ? (
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  ) : (
                    <Check className="w-3.5 h-3.5" />
                  )}
                  <span>{isSubmitting ? 'Provisioning...' : 'Provision Worktree'}</span>
                </button>
              </div>
            </form>
          ) : (
            <div className="flex items-center justify-between">
              <span className="font-semibold text-fg-editor text-xs flex items-center gap-1.5">
                <GitBranch className="w-4 h-4 workbench-accent" />
                Active Managed Worktrees ({worktrees.length})
              </span>
              <button
                onClick={() => setIsCreating(true)}
                className="px-2.5 py-1 rounded-md text-xs font-medium flex items-center gap-1.5 transition bg-surface-2 border border-border-default text-fg-editor hover:bg-surface-3"
              >
                <FolderPlus className="w-3.5 h-3.5 text-fg-muted" />
                <span>+ New Worktree</span>
              </button>
            </div>
          )}

          {/* Worktree Cards List */}
          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-12 text-fg-muted gap-2.5 text-xs">
              <Loader2 className="w-5 h-5 animate-spin workbench-accent" />
              <span className="font-mono text-xs">Syncing workspaces with custos-daemon...</span>
            </div>
          ) : (
            <div className="space-y-2.5">
              {worktrees.map((wt) => {
              const isActive = wt.id === activeWorktreeId || (wt.isMain && !activeWorktreeId);
              return (
                <div
                  key={wt.id}
                  className={`p-3.5 rounded-xl transition flex flex-col space-y-2 border ${
                    isActive
                      ? 'bg-surface-2 border-border-emphasis shadow-xs'
                      : 'bg-surface-0 border-border-muted hover:border-border-default'
                  }`}
                >
                  {/* Top row */}
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 font-mono">
                      <GitBranch className={`w-4 h-4 ${isActive ? 'workbench-accent' : 'text-fg-muted'}`} />
                      <span className="font-semibold text-fg-editor text-xs">{wt.branch}</span>
                      {wt.isMain && (
                        <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-blue-500/10 text-blue-500 border border-blue-500/20">
                          MAIN CHECKOUT
                        </span>
                      )}
                      {isActive && (
                        <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 font-semibold">
                          ACTIVE WORKTREE
                        </span>
                      )}
                    </div>
                    {getStatusBadge(wt.status)}
                  </div>

                  {/* Details row */}
                  <div className="flex items-center justify-between text-xs text-fg-muted font-mono">
                    <div className="flex items-center gap-3">
                      <span>Path: <code className="text-fg-editor">{wt.path}</code></span>
                      <span>Base: <code className="text-emerald-500">#{wt.baseCommit}</code></span>
                      <span className="flex items-center gap-1">
                        <Server className="w-3 h-3" />
                        {wt.host === 'local' ? 'Local host' : 'Remote SSH'}
                      </span>
                    </div>
                    <span>{wt.createdAt}</span>
                  </div>

                  {/* Agent and Actions row */}
                  <div className="flex items-center justify-between pt-2 text-xs border-t border-border-muted">
                    <div className="flex items-center gap-2">
                      <span className="text-fg-muted">Assignee:</span>
                      <span className="text-fg-editor font-medium flex items-center gap-1">
                        <Terminal className="w-3 h-3 text-emerald-500" />
                        {wt.assignedAgent}
                      </span>
                      {wt.modifiedFilesCount > 0 && (
                        <span className="text-[10px] px-1.5 py-0.2 rounded bg-amber-500/10 text-amber-500 font-mono">
                          {wt.modifiedFilesCount} files modified
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-1.5">
                      {!isActive && (
                        <button
                          onClick={() => {
                            onSelectWorktree(wt);
                            onClose();
                          }}
                          className="px-2.5 py-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-editor border border-border-default text-xs font-medium transition flex items-center gap-1"
                        >
                          <Check className="w-3 h-3" />
                          <span>Switch to Worktree</span>
                        </button>
                      )}

                      {wt.status === 'ship' && (
                        <button
                          className="px-2.5 py-1 rounded bg-purple-600 hover:bg-purple-500 text-white text-xs font-medium transition flex items-center gap-1"
                        >
                          <GitMerge className="w-3 h-3" />
                          <span>Ship Preview</span>
                        </button>
                      )}

                      {!wt.isMain && (
                        <button
                          onClick={() => handlePruneWorktree(wt.id)}
                          className="p-1 rounded text-fg-muted hover:text-rose-500 transition hover:bg-rose-500/10"
                          title="Prune / Clean Worktree"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-3.5 flex items-center justify-between bg-surface-1 border-t border-border-muted">
          <div className="flex items-center gap-2 text-xs text-fg-muted">
            <ShieldCheck className="w-3.5 h-3.5 workbench-accent" />
            <span>Git worktrees provide isolated checkouts; baseline verifiers ensure 0 dirty leaks.</span>
          </div>

          <button
            onClick={onClose}
            className="px-3.5 py-1.5 rounded-lg text-fg-editor bg-surface-2 hover:bg-surface-3 border border-border-default text-xs font-medium transition"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};

export default WorktreeManagerModal;
