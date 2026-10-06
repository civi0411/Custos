import React, { useState } from 'react';
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
  GitMerge
} from 'lucide-react';

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

export const WorktreeManagerModal: React.FC<WorktreeManagerModalProps> = ({
  isOpen,
  onClose,
  activeWorktreeId,
  onSelectWorktree
}) => {
  const [worktrees, setWorktrees] = useState<ManagedWorktree[]>([
    {
      id: 'wt-main',
      branch: 'main',
      path: '/Users/mac/Project/AgentHub/Custos',
      baseCommit: 'a3f2d1e',
      host: 'local',
      status: 'work',
      modifiedFilesCount: 0,
      assignedAgent: 'Human (Primary)',
      createdAt: 'Main Checkout',
      isMain: true
    },
    {
      id: 'wt-simd',
      branch: 'feat/simd-dispatch',
      path: '.worktrees/feat-simd-dispatch',
      baseCommit: 'a3f2d1e',
      host: 'local',
      status: 'work',
      modifiedFilesCount: 3,
      assignedAgent: 'Claude Code (S2-Worker)',
      createdAt: '45m ago'
    },
    {
      id: 'wt-permits',
      branch: 'fix/permits-race',
      path: '.worktrees/fix-permits-race',
      baseCommit: 'a3f2d1e',
      host: 'local',
      status: 'review',
      modifiedFilesCount: 1,
      assignedAgent: 'Claude 3.7 Sonnet',
      createdAt: '2h ago'
    },
    {
      id: 'wt-claim',
      branch: 'refactor/claim-matrix',
      path: '.worktrees/refactor-claim-matrix',
      baseCommit: 'a3f2d1e',
      host: 'remote-ssh',
      status: 'ship',
      modifiedFilesCount: 4,
      assignedAgent: 'Custos OI Estimator',
      createdAt: '3h ago'
    }
  ]);

  const [isCreating, setIsCreating] = useState(false);
  const [newBranch, setNewBranch] = useState('');
  const [newBaseCommit, setNewBaseCommit] = useState('a3f2d1e');
  const [newHost, setNewHost] = useState<'local' | 'remote-ssh'>('local');
  const [runSetupScript, setRunSetupScript] = useState(true);

  if (!isOpen) return null;

  const handleCreateWorktree = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newBranch.trim()) return;

    const sanitizedBranch = newBranch.trim().toLowerCase().replace(/\s+/g, '-');
    const created: ManagedWorktree = {
      id: `wt-${Date.now().toString().slice(-4)}`,
      branch: sanitizedBranch,
      path: `.worktrees/${sanitizedBranch.replace(/\//g, '-')}`,
      baseCommit: newBaseCommit.trim() || 'a3f2d1e',
      host: newHost,
      status: 'work',
      modifiedFilesCount: 0,
      assignedAgent: 'Claude Code',
      createdAt: 'Just now'
    };

    setWorktrees(prev => [created, ...prev]);
    onSelectWorktree(created);
    setIsCreating(false);
    setNewBranch('');
  };

  const handlePruneWorktree = (id: string) => {
    setWorktrees(prev => prev.filter(w => w.id !== id));
  };

  const getStatusBadge = (status: ManagedWorktree['status']) => {
    switch (status) {
      case 'create':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-blue-500/10 text-blue-400 border border-blue-500/20">1. Setup</span>;
      case 'work':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center gap-1"><span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>2. Work (Active)</span>;
      case 'review':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-amber-500/10 text-amber-400 border border-amber-500/20">3. Review Diff</span>;
      case 'ship':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-purple-500/10 text-purple-400 border border-purple-500/20">4. Ready to Ship</span>;
      case 'cleanup':
        return <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-neutral-800 text-neutral-400">5. Cleanup</span>;
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm animate-in fade-in duration-150">
      <div 
        className="w-full max-w-2xl rounded-2xl overflow-hidden flex flex-col font-sans max-h-[85vh] shadow-2xl"
        style={{
          background: 'var(--color-surface-1, #161b22)',
          border: '1px solid var(--color-border-default, #30363d)'
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div 
          className="px-6 py-4 flex items-center justify-between"
          style={{
            background: 'var(--color-surface-2, #1c2128)',
            borderBottom: '1px solid var(--color-border-default, #30363d)'
          }}
        >
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg flex items-center justify-center text-[#3fb950]" style={{ background: 'rgba(63,185,80,0.15)', border: '1px solid rgba(63,185,80,0.3)' }}>
              <GitBranch className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-semibold text-white text-sm">Orca Worktree Lifecycle Engine</span>
                <span className="text-[10px] px-2 py-0.5 rounded-full font-mono text-[#3fb950]" style={{ background: 'rgba(63,185,80,0.1)' }}>
                  Isolated Execution Workspaces
                </span>
              </div>
              <p className="text-[11px] text-[#8b949e]">
                Managed worktrees create isolated checkout directories preserving your main workspace & dirty changes.
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-white/5 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-5 text-xs text-neutral-300">
          {/* Worktree Lifecycle Progress Indicator */}
          <div 
            className="p-3.5 rounded-xl flex items-center justify-between text-[11px] font-mono"
            style={{
              background: 'var(--color-canvas, #0d1117)',
              border: '1px solid var(--color-border-muted, #21262d)'
            }}
          >
            <div className="flex items-center gap-1.5 text-neutral-400">
              <span className="w-5 h-5 rounded-full bg-blue-500/20 text-blue-400 flex items-center justify-center text-[10px] font-bold">1</span>
              <span>Create Worktree</span>
            </div>
            <ArrowRight className="w-3 h-3 text-[#6e7681]" />
            <div className="flex items-center gap-1.5 text-[#3fb950] font-semibold">
              <span className="w-5 h-5 rounded-full bg-emerald-500/20 text-[#3fb950] flex items-center justify-center text-[10px] font-bold">2</span>
              <span>Agent Work</span>
            </div>
            <ArrowRight className="w-3 h-3 text-[#6e7681]" />
            <div className="flex items-center gap-1.5 text-neutral-400">
              <span className="w-5 h-5 rounded-full bg-amber-500/20 text-amber-400 flex items-center justify-center text-[10px] font-bold">3</span>
              <span>Review Diff</span>
            </div>
            <ArrowRight className="w-3 h-3 text-[#6e7681]" />
            <div className="flex items-center gap-1.5 text-neutral-400">
              <span className="w-5 h-5 rounded-full bg-purple-500/20 text-purple-400 flex items-center justify-center text-[10px] font-bold">4</span>
              <span>Ship / Integrate</span>
            </div>
            <ArrowRight className="w-3 h-3 text-[#6e7681]" />
            <div className="flex items-center gap-1.5 text-neutral-500">
              <span className="w-5 h-5 rounded-full bg-neutral-800 text-neutral-400 flex items-center justify-center text-[10px] font-bold">5</span>
              <span>Cleanup</span>
            </div>
          </div>

          {/* Create New Worktree Section */}
          {isCreating ? (
            <form 
              onSubmit={handleCreateWorktree}
              className="p-4 rounded-xl space-y-3"
              style={{
                background: 'var(--color-surface-2, #1c2128)',
                border: '1px solid var(--color-coding, #3fb950)'
              }}
            >
              <div className="flex items-center justify-between">
                <span className="font-semibold text-white flex items-center gap-1.5">
                  <FolderPlus className="w-4 h-4 text-[#3fb950]" />
                  Create Managed Worktree (Orca O2)
                </span>
                <button
                  type="button"
                  onClick={() => setIsCreating(false)}
                  className="text-[11px] text-[#8b949e] hover:text-white"
                >
                  Cancel
                </button>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-[11px] font-medium text-[#8b949e] mb-1">
                    Branch Name
                  </label>
                  <input
                    type="text"
                    value={newBranch}
                    onChange={(e) => setNewBranch(e.target.value)}
                    placeholder="feat/ast-invariant-cache"
                    required
                    className="w-full rounded-md px-3 py-1.5 font-mono text-xs text-white placeholder-[#6e7681] focus:outline-none"
                    style={{
                      background: 'var(--color-canvas, #0d1117)',
                      border: '1px solid var(--color-border-default, #30363d)'
                    }}
                  />
                </div>

                <div>
                  <label className="block text-[11px] font-medium text-[#8b949e] mb-1">
                    Base Commit Hash
                  </label>
                  <input
                    type="text"
                    value={newBaseCommit}
                    onChange={(e) => setNewBaseCommit(e.target.value)}
                    placeholder="a3f2d1e"
                    className="w-full rounded-md px-3 py-1.5 font-mono text-xs text-white placeholder-[#6e7681] focus:outline-none"
                    style={{
                      background: 'var(--color-canvas, #0d1117)',
                      border: '1px solid var(--color-border-default, #30363d)'
                    }}
                  />
                </div>
              </div>

              <div className="flex items-center justify-between pt-1">
                <div className="flex items-center gap-3">
                  <label className="flex items-center gap-1.5 cursor-pointer text-[11px] text-neutral-300">
                    <input
                      type="radio"
                      name="host"
                      checked={newHost === 'local'}
                      onChange={() => setNewHost('local')}
                      className="accent-[#3fb950]"
                    />
                    <span>Local Host (macOS)</span>
                  </label>
                  <label className="flex items-center gap-1.5 cursor-pointer text-[11px] text-neutral-300">
                    <input
                      type="radio"
                      name="host"
                      checked={newHost === 'remote-ssh'}
                      onChange={() => setNewHost('remote-ssh')}
                      className="accent-[#3fb950]"
                    />
                    <span>Remote SSH (orcad)</span>
                  </label>
                </div>

                <label className="flex items-center gap-1.5 cursor-pointer text-[11px] text-[#8b949e]">
                  <input
                    type="checkbox"
                    checked={runSetupScript}
                    onChange={(e) => setRunSetupScript(e.target.checked)}
                    className="accent-[#3fb950]"
                  />
                  <span>Run setup script</span>
                </label>
              </div>

              <div className="flex justify-end pt-2">
                <button
                  type="submit"
                  className="px-4 py-1.5 rounded-md text-white font-medium text-xs flex items-center gap-1.5 transition"
                  style={{ background: 'var(--color-coding, #3fb950)' }}
                >
                  <Check className="w-3.5 h-3.5" />
                  <span>Provision Worktree & Terminal</span>
                </button>
              </div>
            </form>
          ) : (
            <div className="flex items-center justify-between">
              <span className="font-semibold text-white text-[12px] flex items-center gap-1.5">
                <GitBranch className="w-4 h-4 text-[#3fb950]" />
                Active Managed Worktrees ({worktrees.length})
              </span>
              <button
                onClick={() => setIsCreating(true)}
                className="px-2.5 py-1 rounded-md text-[11px] font-medium flex items-center gap-1.5 transition"
                style={{
                  background: 'var(--color-surface-2, #1c2128)',
                  border: '1px solid var(--color-border-default, #30363d)',
                  color: 'var(--color-editor-fg, #e6edf3)'
                }}
              >
                <FolderPlus className="w-3.5 h-3.5 text-[#3fb950]" />
                <span>+ New Worktree</span>
              </button>
            </div>
          )}

          {/* Worktree Cards List */}
          <div className="space-y-2.5">
            {worktrees.map((wt) => {
              const isActive = wt.id === activeWorktreeId || (wt.isMain && !activeWorktreeId);
              return (
                <div
                  key={wt.id}
                  className="p-3.5 rounded-xl transition flex flex-col space-y-2"
                  style={{
                    background: isActive ? 'var(--color-surface-2, #1c2128)' : 'var(--color-canvas, #0d1117)',
                    border: `1px solid ${isActive ? 'var(--color-coding, #3fb950)' : 'var(--color-border-default, #30363d)'}`
                  }}
                >
                  {/* Top row */}
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 font-mono">
                      <GitBranch className="w-4 h-4" style={{ color: isActive ? 'var(--color-coding, #3fb950)' : '#8b949e' }} />
                      <span className="font-semibold text-white text-[13px]">{wt.branch}</span>
                      {wt.isMain && (
                        <span className="px-1.5 py-0.2 rounded text-[9.5px] font-mono bg-blue-500/10 text-blue-400 border border-blue-500/20">
                          MAIN CHECKOUT
                        </span>
                      )}
                      {isActive && (
                        <span className="px-1.5 py-0.2 rounded text-[9.5px] font-mono bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-bold">
                          ACTIVE WORKTREE
                        </span>
                      )}
                    </div>
                    {getStatusBadge(wt.status)}
                  </div>

                  {/* Details row */}
                  <div className="flex items-center justify-between text-[11px] text-[#8b949e] font-mono">
                    <div className="flex items-center gap-3">
                      <span>Path: <code className="text-neutral-300">{wt.path}</code></span>
                      <span>Base: <code className="text-[#3fb950]">#{wt.baseCommit}</code></span>
                      <span className="flex items-center gap-1">
                        <Server className="w-3 h-3" />
                        {wt.host === 'local' ? 'Local host' : 'Remote SSH'}
                      </span>
                    </div>
                    <span>{wt.createdAt}</span>
                  </div>

                  {/* Agent and Actions row */}
                  <div 
                    className="flex items-center justify-between pt-2 text-[11px]"
                    style={{ borderTop: '1px solid var(--color-border-muted, #21262d)' }}
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-[#8b949e]">Assignee:</span>
                      <span className="text-white font-medium flex items-center gap-1">
                        <Terminal className="w-3 h-3 text-[#3fb950]" />
                        {wt.assignedAgent}
                      </span>
                      {wt.modifiedFilesCount > 0 && (
                        <span className="text-[10px] px-1.5 py-0.2 rounded bg-amber-500/10 text-amber-400 font-mono">
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
                          className="px-2.5 py-1 rounded text-white text-xs font-medium transition flex items-center gap-1"
                          style={{ background: 'var(--color-coding, #3fb950)' }}
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
                          <span>Ship / Merge Preview</span>
                        </button>
                      )}

                      {!wt.isMain && (
                        <button
                          onClick={() => handlePruneWorktree(wt.id)}
                          className="p-1 rounded text-[#8b949e] hover:text-[#f85149] transition hover:bg-red-500/10"
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
        </div>

        {/* Footer */}
        <div 
          className="px-6 py-3.5 flex items-center justify-between"
          style={{
            background: 'var(--color-surface-2, #1c2128)',
            borderTop: '1px solid var(--color-border-default, #30363d)'
          }}
        >
          <div className="flex items-center gap-2 text-[11px] text-[#8b949e]">
            <ShieldCheck className="w-3.5 h-3.5 text-[#3fb950]" />
            <span>Git worktrees provide isolated checkouts; baseline verifiers ensure 0 dirty leaks.</span>
          </div>

          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-lg text-white text-xs font-medium transition"
            style={{
              background: 'var(--color-surface-1, #161b22)',
              border: '1px solid var(--color-border-default, #30363d)'
            }}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};

export default WorktreeManagerModal;
