import { useMemo } from 'react';
import type { Session } from '@/types';
import type { ExecutionWorkspace, WorkspaceDetectionResult } from '@/types/domain';
import {
  normPath,
  matchWorkspacePath,
  type ProjectGroupItem,
  type WorkspaceItem,
  type WorktreeItem,
} from './types';

export interface UseProjectGroupsParams {
  currentProject: string;
  effectiveDetection: WorkspaceDetectionResult | null | undefined;
  currentProjectPath: string;
  workspaces: ExecutionWorkspace[];
  activeWorkspaceId?: string;
  activeWorkspace?: ExecutionWorkspace | null;
  allSessions: Session[];
  filterQuery: string;
  filterActiveOnly: boolean;
  getAgentActivity: (wsId: string, wsPath: string) => WorktreeItem['agentActivity'];
}

export function useProjectGroups({
  currentProject,
  effectiveDetection,
  currentProjectPath,
  workspaces,
  activeWorkspaceId,
  activeWorkspace,
  allSessions,
  filterQuery,
  filterActiveOnly,
  getAgentActivity,
}: UseProjectGroupsParams): ProjectGroupItem[] {
  return useMemo<ProjectGroupItem[]>(() => {
    const projectName = currentProject || effectiveDetection?.name || 'AgentHub';
    const projectPath = currentProjectPath;
    const isMultiRepo = Boolean(
      effectiveDetection?.sub_repositories && effectiveDetection.sub_repositories.length > 0
    );

    const projectWorkspaces: WorkspaceItem[] = [];
    const otherWorkspaces: WorkspaceItem[] = [];

    const wsByPath = new Map<string, ExecutionWorkspace>();
    workspaces.forEach((ws) => {
      if (ws.status !== 'archived') {
        if (ws.path) wsByPath.set(normPath(ws.path), ws);
        if (ws.kind?.type === 'folder' && ws.kind.path) wsByPath.set(normPath(ws.kind.path), ws);
        if (ws.kind?.type === 'git' && ws.kind.repo_path) wsByPath.set(normPath(ws.kind.repo_path), ws);
      }
    });

    if (effectiveDetection?.sub_repositories && effectiveDetection.sub_repositories.length > 0) {
      effectiveDetection.sub_repositories.forEach((sub: any) => {
        const subNorm = normPath(sub.path);
        const matchedWs =
          wsByPath.get(subNorm) ||
          workspaces.find((w) => w.status !== 'archived' && matchWorkspacePath(w, subNorm));
        const wsId = matchedWs?.id || `sub_${sub.name}`;
        const mainBranch = sub.branch || matchedWs?.active_branch || 'main';
        const daemonDirtyCount = matchedWs?.dirty_manifest?.modified_files?.length ?? 0;
        const dirtyCount = Math.max(sub.dirty_count ?? 0, daemonDirtyCount);
        const isDirty = sub.is_dirty || Boolean(matchedWs?.dirty_manifest?.is_dirty) || dirtyCount > 0;

        const worktrees: WorktreeItem[] = [];

        const primarySessions = allSessions.filter(
          (s) => s.workspaceId === wsId || (matchedWs && s.workspaceId === matchedWs.id) || s.workspaceId === sub.path
        );
        worktrees.push({
          id: wsId,
          name: mainBranch,
          branch: mainBranch,
          path: sub.path,
          isMain: true,
          isDirty,
          dirtyCount,
          isRunning: primarySessions.some((s) => s.taskStatus === 'running'),
          isBlocked: primarySessions.some((s) => s.taskStatus === 'blocked'),
          sessionCount: primarySessions.length,
          agentActivity: getAgentActivity(wsId, sub.path),
        });

        if (sub.worktrees) {
          sub.worktrees.forEach((wt: any) => {
            if (!wt.is_main) {
              const wtNorm = normPath(wt.path);
              const matchedWtWs =
                wsByPath.get(wtNorm) ||
                workspaces.find((w) => w.status !== 'archived' && matchWorkspacePath(w, wtNorm));
              const wtId = matchedWtWs?.id || `wt_${wt.path}`;
              const wtBranch = wt.branch || matchedWtWs?.active_branch || 'worktree';
              const wtDirtyCount = matchedWtWs?.dirty_manifest?.modified_files?.length ?? 0;
              const wtSessions = allSessions.filter(
                (s) => s.workspaceId === wtId || (matchedWtWs && s.workspaceId === matchedWtWs.id) || s.workspaceId === wt.path
              );
              worktrees.push({
                id: wtId,
                name: wtBranch,
                branch: wtBranch,
                path: wt.path,
                isMain: false,
                isDirty: wt.is_dirty || wtDirtyCount > 0,
                dirtyCount: Math.max(wt.dirty_count ?? 0, wtDirtyCount),
                isRunning: wtSessions.some((s) => s.taskStatus === 'running'),
                isBlocked: wtSessions.some((s) => s.taskStatus === 'blocked'),
                sessionCount: wtSessions.length,
                agentActivity: getAgentActivity(wtId, wt.path),
              });
            }
          });
        }

        projectWorkspaces.push({
          id: wsId,
          name: sub.name,
          path: sub.path,
          kind: 'git',
          worktrees,
        });
      });
    } else {
      const activeBranch = effectiveDetection?.branch || activeWorkspace?.active_branch || 'main';
      const daemonDirtyCount = activeWorkspace?.dirty_manifest?.modified_files?.length ?? 0;
      const dirtyCount = Math.max(effectiveDetection?.dirty_count ?? 0, daemonDirtyCount);
      const isDirty = (effectiveDetection?.is_dirty ?? false) || Boolean(activeWorkspace?.dirty_manifest?.is_dirty) || dirtyCount > 0;

      const worktrees: WorktreeItem[] = [];
      const primarySessions = allSessions.filter(
        (s) => s.workspaceId === activeWorkspaceId || s.workspaceId === currentProjectPath
      );
      worktrees.push({
        id: activeWorkspaceId || 'ws_root',
        name: activeBranch,
        branch: activeBranch,
        path: currentProjectPath,
        isMain: true,
        isDirty,
        dirtyCount,
        isRunning: primarySessions.some((s) => s.taskStatus === 'running'),
        isBlocked: primarySessions.some((s) => s.taskStatus === 'blocked'),
        sessionCount: primarySessions.length,
        agentActivity: getAgentActivity(activeWorkspaceId || 'ws_root', currentProjectPath),
      });

      if (effectiveDetection?.worktrees) {
        effectiveDetection.worktrees.forEach((wt: any) => {
          if (!wt.is_main) {
            const wtSessions = allSessions.filter((s) => s.workspaceId === wt.path);
            const branchName = wt.branch || 'worktree';
            worktrees.push({
              id: `wt_${wt.path}`,
              name: branchName,
              branch: branchName,
              path: wt.path,
              isMain: false,
              isDirty: Boolean(wt.is_dirty),
              dirtyCount: wt.dirty_count ?? 0,
              isRunning: wtSessions.some((s) => s.taskStatus === 'running'),
              isBlocked: wtSessions.some((s) => s.taskStatus === 'blocked'),
              sessionCount: wtSessions.length,
              agentActivity: getAgentActivity(`wt_${wt.path}`, wt.path),
            });
          }
        });
      }

      projectWorkspaces.push({
        id: activeWorkspaceId || 'proj_root',
        name: effectiveDetection?.name || projectName,
        path: currentProjectPath,
        kind: effectiveDetection?.is_git ? 'git' : 'folder',
        worktrees,
      });
    }

    const groups: ProjectGroupItem[] = [];
    if (projectWorkspaces.length > 0) {
      groups.push({
        id: 'proj_agenthub',
        name: projectName,
        path: projectPath,
        isMultiRepo,
        workspaces: projectWorkspaces,
      });
    }
    if (otherWorkspaces.length > 0) {
      groups.push({
        id: 'proj_other',
        name: 'Other Workspaces',
        path: '',
        isMultiRepo: false,
        workspaces: otherWorkspaces,
      });
    }

    if (!filterQuery.trim() && !filterActiveOnly) {
      return groups;
    }

    const query = filterQuery.toLowerCase().trim();
    return groups
      .map((g) => ({
        ...g,
        workspaces: g.workspaces
          .map((ws) => ({
            ...ws,
            worktrees: ws.worktrees.filter((wt) => {
              if (filterActiveOnly && !wt.isRunning && !wt.isBlocked && !wt.isDirty) return false;
              if (query && !wt.name.toLowerCase().includes(query) && !wt.path.toLowerCase().includes(query)) return false;
              return true;
            }),
          }))
          .filter((ws) => ws.worktrees.length > 0 || (query && ws.name.toLowerCase().includes(query))),
      }))
      .filter((g) => g.workspaces.length > 0);
  }, [
    currentProject,
    effectiveDetection,
    currentProjectPath,
    workspaces,
    activeWorkspaceId,
    activeWorkspace,
    allSessions,
    filterQuery,
    filterActiveOnly,
    getAgentActivity,
  ]);
}
