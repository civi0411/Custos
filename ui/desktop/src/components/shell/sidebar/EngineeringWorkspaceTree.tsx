import React from 'react';
import {
  FolderOpen,
  GitFork,
  FolderGit2,
  GitBranch,
  Plus,
  ChevronDown,
  ChevronRight,
  Check,
  Shield,
} from 'lucide-react';
import type { ExecutionWorkspace } from '@/types/domain';
import type { ProjectGroupItem, WorktreeItem } from './types';

export interface EngineeringWorkspaceTreeProps {
  projectGroups: ProjectGroupItem[];
  expandedProjects: Record<string, boolean>;
  setExpandedProjects: React.Dispatch<React.SetStateAction<Record<string, boolean>>>;
  expandedWorkspaces: Record<string, boolean>;
  setExpandedWorkspaces: React.Dispatch<React.SetStateAction<Record<string, boolean>>>;
  activeWorkspaceId?: string;
  activeWorkspace?: ExecutionWorkspace | null;
  creatingWorktreeFor: string | null;
  setCreatingWorktreeFor: (id: string | null) => void;
  worktreeBranchInput: string;
  setWorktreeBranchInput: (v: string) => void;
  isSubmittingWorktree: boolean;
  handleOpenFolderDirect: () => void | Promise<void>;
  handleAddRepoToProject: () => void | Promise<void>;
  handleSelectWorkspaceItem: (wsId: string, wsPath: string) => void | Promise<void>;
  handleAddWorktree: (wsId: string, branch: string) => void | Promise<void>;
  normPath: (p?: string) => string;
}

export const EngineeringWorkspaceTree: React.FC<EngineeringWorkspaceTreeProps> = ({
  projectGroups,
  expandedProjects,
  setExpandedProjects,
  expandedWorkspaces,
  setExpandedWorkspaces,
  activeWorkspaceId,
  activeWorkspace,
  creatingWorktreeFor,
  setCreatingWorktreeFor,
  worktreeBranchInput,
  setWorktreeBranchInput,
  isSubmittingWorktree,
  handleOpenFolderDirect,
  handleAddRepoToProject,
  handleSelectWorkspaceItem,
  handleAddWorktree,
  normPath,
}) => {
  if (projectGroups.length === 0) {
    return (
      <div className="p-4 text-center space-y-2.5 border border-dashed border-white/10 rounded-lg my-2">
        <FolderOpen className="size-6 text-white/30 mx-auto" />
        <div className="text-white/50 text-xs">No projects currently open</div>
        <button
          type="button"
          onClick={handleOpenFolderDirect}
          className="px-3 py-1.5 bg-white/8 hover:bg-white/14 text-white text-xs rounded-md transition font-medium cursor-pointer"
        >
          Open Project Folder
        </button>
      </div>
    );
  }

  return (
    <>
      {projectGroups.map((group) => {
        const isGroupExpanded = expandedProjects[group.id] ?? true;

        return (
          <div key={group.id} className="space-y-1">
            {/* ── Level 1: Project Group Header ── */}
            <div
              className="flex items-center justify-between px-2 py-1.5 rounded-lg select-none cursor-pointer group/proj hover:bg-white/[0.04] transition"
              onClick={() =>
                setExpandedProjects((prev) => ({
                  ...prev,
                  [group.id]: !isGroupExpanded,
                }))
              }
            >
              <div className="flex items-center gap-2 min-w-0 flex-1">
                <GitFork className="size-3.5 text-white/70 shrink-0" />
                <span className="font-bold text-[12.5px] text-white tracking-wide truncate">
                  {group.name}
                </span>
                {group.isMultiRepo && (
                  <span className="font-mono text-[9px] px-1.5 py-0.2 rounded bg-white/8 text-white/60 shrink-0 border border-white/8">
                    {group.workspaces.length} repos
                  </span>
                )}
              </div>

              <div className="flex items-center gap-1 shrink-0 text-white/40 group-hover/proj:text-white/70">
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    void handleAddRepoToProject();
                  }}
                  className="p-1 rounded hover:bg-white/10 opacity-0 group-hover/proj:opacity-100 transition cursor-pointer"
                  title="Add repository to project"
                  aria-label="Add repository to project"
                >
                  <Plus className="size-3" />
                </button>
                {isGroupExpanded ? (
                  <ChevronDown className="size-3.5" />
                ) : (
                  <ChevronRight className="size-3.5" />
                )}
              </div>
            </div>

            {/* ── Level 2 & 3: Workspaces & Worktrees ── */}
            {isGroupExpanded && (
              <div className="space-y-2 border-l border-white/8 ml-2.5 pl-2">
                {group.workspaces.map((ws) => {
                  const isWsExpanded = expandedWorkspaces[ws.id] ?? true;
                  const isCreating = creatingWorktreeFor === ws.id;
                  const isRepoActive =
                    ws.id === activeWorkspaceId ||
                    ws.worktrees.some((wt) => wt.id === activeWorkspaceId) ||
                    (activeWorkspace &&
                      (normPath(ws.path) === normPath(activeWorkspace.path) ||
                        ws.worktrees.some((wt) => normPath(wt.path) === normPath(activeWorkspace.path))));

                  return (
                    <div key={ws.id} className="space-y-1">
                      {/* Workspace / Repository Header */}
                      <div
                        className={`flex items-center justify-between px-2 py-1.5 select-none cursor-pointer group/repo rounded-lg transition ${
                          isRepoActive
                            ? 'bg-white/[0.08] text-white'
                            : 'hover:bg-white/[0.04] text-white/80'
                        }`}
                        onClick={() => {
                          if (!isWsExpanded) {
                            setExpandedWorkspaces((prev) => ({
                              ...prev,
                              [ws.id]: true,
                            }));
                          }
                          const target =
                            ws.worktrees.find((wt) => wt.id === activeWorkspaceId) ||
                            ws.worktrees.find((wt) => wt.isMain) ||
                            ws.worktrees[0];
                          if (target) {
                            void handleSelectWorkspaceItem(target.id, target.path);
                          } else {
                            void handleSelectWorkspaceItem(ws.id, ws.path);
                          }
                        }}
                      >
                        <div className="flex items-center gap-1.5 min-w-0 flex-1">
                          {ws.worktrees.length > 1 ? (
                            <span
                              onClick={(e) => {
                                e.stopPropagation();
                                setExpandedWorkspaces((prev) => ({
                                  ...prev,
                                  [ws.id]: !isWsExpanded,
                                }));
                              }}
                              className="text-white/40 hover:text-white transition p-0.5 rounded cursor-pointer shrink-0"
                              title={isWsExpanded ? 'Collapse branches' : 'Expand branches'}
                            >
                              {isWsExpanded ? (
                                <ChevronDown className="size-3" />
                              ) : (
                                <ChevronRight className="size-3" />
                              )}
                            </span>
                          ) : (
                            <span className="w-3 shrink-0" />
                          )}

                          <FolderGit2 className={`size-3.5 shrink-0 ${isRepoActive ? 'text-white' : 'text-white/50'}`} />

                          <span className={`truncate text-[12px] ${isRepoActive ? 'text-white font-semibold' : 'text-white/90 font-medium'}`}>
                            {ws.name}
                          </span>

                          {ws.worktrees.length > 1 && (
                            <span className="font-mono text-[9px] text-white/40">
                              ({ws.worktrees.length})
                            </span>
                          )}
                        </div>

                        <div className="flex items-center gap-1 opacity-0 group-hover/repo:opacity-100 transition shrink-0">
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              setCreatingWorktreeFor(ws.id);
                              setExpandedWorkspaces((prev) => ({ ...prev, [ws.id]: true }));
                            }}
                            className="p-1 rounded text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer"
                            title={`Create new worktree from ${ws.name}`}
                            aria-label={`New worktree for ${ws.name}`}
                          >
                            <Plus className="size-3" />
                          </button>
                        </div>
                      </div>

                      {/* Worktrees List under Workspace */}
                      {isWsExpanded && (
                        <div className="border-l border-white/6 ml-3 pl-2 space-y-1">
                          {ws.worktrees.map((wt: WorktreeItem) => {
                            const isWtActive =
                              wt.id === activeWorkspaceId ||
                              (activeWorkspace &&
                                (normPath(wt.path) === normPath(activeWorkspace.path) || wt.id === activeWorkspace.id));

                            return (
                              <div
                                key={wt.id}
                                onClick={() => void handleSelectWorkspaceItem(wt.id, wt.path)}
                                className={`relative flex flex-col cursor-pointer transition select-none ${
                                  isWtActive
                                    ? 'bg-white/[0.08] border border-white/14 rounded-xl p-2.5 my-1 shadow-sm'
                                    : 'hover:bg-white/[0.04] rounded-lg px-2.5 py-1.5 my-0.5 border border-transparent'
                                }`}
                                title={`${wt.name} (${wt.path})`}
                              >
                                {/* Branch Header Row */}
                                <div className="flex items-center justify-between gap-1.5 min-w-0">
                                  <div className="flex items-center gap-1.5 min-w-0 flex-1">
                                    {/* Status Dot: green = clean, amber = dirty, pulsing = agent active */}
                                    {wt.isRunning ? (
                                      <span className="size-2 rounded-full bg-white animate-pulse shrink-0" />
                                    ) : wt.isBlocked ? (
                                      <Shield className="size-2.5 text-amber-300 shrink-0" />
                                    ) : wt.isDirty ? (
                                      <span className="size-2 rounded-full bg-amber-400 shrink-0" />
                                    ) : (
                                      <span className="size-2 rounded-full bg-emerald-400 shrink-0" />
                                    )}

                                    {/* Branch Name */}
                                    <span
                                      className={`truncate font-mono text-[11.5px] leading-tight ${
                                        isWtActive ? 'text-white font-semibold' : 'text-white/85'
                                      }`}
                                    >
                                      {wt.branch || wt.name}
                                    </span>

                                    {/* Primary / Secondary Tag */}
                                    {wt.isMain ? (
                                      <span className="h-[15px] px-1 text-[9px] font-mono rounded shrink-0 leading-none text-white/60 border border-white/12 bg-white/[0.04]">
                                        primary
                                      </span>
                                    ) : (
                                      <GitBranch className="size-2.5 text-white/35 shrink-0" />
                                    )}
                                  </div>

                                  {/* Status Meta: Dirty Count & Checkmark */}
                                  <div className="flex items-center gap-1 shrink-0 font-mono text-[9.5px]">
                                    {wt.dirtyCount > 0 && (
                                      <span className="text-amber-400/80 font-semibold">
                                        {wt.dirtyCount}*
                                      </span>
                                    )}
                                    {isWtActive && (
                                      <Check className="size-3 text-white/80 ml-0.5" />
                                    )}
                                  </div>
                                </div>

                                {/* Relative Path Directory */}
                                <div className="flex items-center gap-1.5 pl-3.5 text-[9.5px] text-white/35 font-mono truncate leading-tight mt-0.5">
                                  <span className="truncate">
                                    {wt.path.split('/').pop() || wt.branch}/
                                  </span>
                                </div>

                                {/* Agent Activity Line (Real Data from Session) */}
                                {wt.agentActivity && (
                                  <div className="flex items-center gap-1 pl-3.5 mt-1 text-[10px] text-white/50 truncate font-sans">
                                    <Check className="size-2.5 text-emerald-400 shrink-0" />
                                    <span className="truncate max-w-[125px] text-white/70">
                                      {wt.agentActivity.title}
                                    </span>
                                    {wt.agentActivity.model && (
                                      <span className="text-white/35 font-mono truncate max-w-[65px]">
                                        · {wt.agentActivity.model}
                                      </span>
                                    )}
                                  </div>
                                )}
                              </div>
                            );
                          })}

                          {/* Inline Create Worktree Form */}
                          {isCreating && (
                            <form
                              onSubmit={(e) => {
                                e.preventDefault();
                                void handleAddWorktree(ws.id, worktreeBranchInput);
                              }}
                              className="mt-1 p-2 rounded-lg bg-white/6 border border-white/10 space-y-1.5 font-sans"
                            >
                              <div className="text-[10px] text-white/50 font-medium">
                                Create new worktree from {ws.name}:
                              </div>
                              <input
                                type="text"
                                value={worktreeBranchInput}
                                onChange={(e) => setWorktreeBranchInput(e.target.value)}
                                placeholder="new-branch (e.g. feat-auth)"
                                autoFocus
                                disabled={isSubmittingWorktree}
                                className="w-full bg-black/40 border border-white/10 rounded px-2 py-1 text-xs text-white placeholder-white/30 outline-none font-mono"
                              />
                              <div className="flex justify-end gap-1.5 text-[10px]">
                                <button
                                  type="button"
                                  disabled={isSubmittingWorktree}
                                  onClick={() => {
                                    setCreatingWorktreeFor(null);
                                    setWorktreeBranchInput('');
                                  }}
                                  className="px-2 py-0.5 text-white/50 hover:text-white cursor-pointer"
                                >
                                  Cancel
                                </button>
                                <button
                                  type="submit"
                                  disabled={!worktreeBranchInput.trim() || isSubmittingWorktree}
                                  className="px-2.5 py-0.5 bg-white/20 hover:bg-white/30 text-white rounded font-medium disabled:opacity-50 cursor-pointer"
                                >
                                  {isSubmittingWorktree ? 'Creating...' : 'Create Worktree'}
                                </button>
                              </div>
                            </form>
                          )}

                          {/* Inline + Worktree trigger */}
                          {!isCreating && (
                            <button
                              type="button"
                              onClick={() => setCreatingWorktreeFor(ws.id)}
                              className="w-full flex items-center gap-1.5 px-2 py-1 text-[10.5px] text-white/40 hover:text-white transition font-sans cursor-pointer rounded hover:bg-white/[0.02]"
                            >
                              <Plus className="size-2.5 text-white/40" />
                              <span>Create new worktree...</span>
                            </button>
                          )}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        );
      })}
    </>
  );
};
