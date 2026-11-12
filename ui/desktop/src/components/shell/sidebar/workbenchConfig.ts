import { Bot, FlaskConical, FolderGit2, TrendingUp } from 'lucide-react';
import type { ExecutionWorkspace, WorkspaceDetectionResult } from '@/types/domain';
import type { ProjectGroupItem } from './types';

export interface WorkbenchSidebarConfig {
  title: string;
  icon: React.ElementType;
  iconColor: string;
  badge: string;
  badgeClass: string;
  searchPlaceholder: string;
  allLabel: string;
  allCount: number;
  activeLabel: string;
  activeCount: number;
  footerStatus: string;
}

export function buildWorkbenchSidebarConfig(params: {
  activeWorkbench: string;
  researchPath: string | null;
  researchDetection: WorkspaceDetectionResult | null;
  tasks: any[];
  projectGroups: ProjectGroupItem[];
  activeWorkspace?: ExecutionWorkspace;
}): WorkbenchSidebarConfig {
  const { activeWorkbench, researchPath, researchDetection, tasks, projectGroups, activeWorkspace } = params;

  switch (activeWorkbench) {
    case 'research': {
      const studiesCount = researchPath ? 1 : 0;
      return {
        title: 'Research Study',
        icon: FlaskConical,
        iconColor: 'text-amber-400',
        badge: 'STUDY',
        badgeClass: 'bg-amber-400/10 text-amber-300 border-amber-400/20',
        searchPlaceholder: 'Search studies, findings, claims...',
        allLabel: 'All Studies',
        allCount: studiesCount,
        activeLabel: 'Active',
        activeCount: studiesCount,
        footerStatus:
          researchDetection?.name || (researchPath ? 'Research Study Active' : 'No Study Folder Selected'),
      };
    }
    case 'copilot':
      return {
        title: 'Autonomous Tasks',
        icon: Bot,
        iconColor: 'text-emerald-400',
        badge: 'SUPERVISOR',
        badgeClass: 'bg-emerald-400/10 text-emerald-300 border-emerald-400/20',
        searchPlaceholder: 'Search tasks, operations...',
        allLabel: 'All Tasks',
        allCount: tasks.length,
        activeLabel: 'Action Needed',
        activeCount: tasks.filter((t) => t.status === 'Blocked' || t.metadata?.approval_required === 'true').length,
        footerStatus: 'Copilot Supervisor Ready',
      };
    case 'quant':
      return {
        title: 'Quant Strategies',
        icon: TrendingUp,
        iconColor: 'text-workbench-accent',
        badge: 'PORTFOLIO',
        badgeClass: 'bg-violet-400/10 text-violet-300 border-violet-400/20',
        searchPlaceholder: 'Search alphas, backtests, risk audits...',
        allLabel: 'All Strategies',
        allCount: 0,
        activeLabel: 'Live Alphas',
        activeCount: 0,
        footerStatus: 'No Active Deployments',
      };
    case 'engineering':
    default: {
      const totalWorktrees = projectGroups.reduce(
        (acc, g) => acc + g.workspaces.reduce((wAcc, ws) => wAcc + ws.worktrees.length, 0),
        0
      );
      const activeWorktrees = projectGroups.reduce(
        (acc, g) =>
          acc +
          g.workspaces.reduce(
            (wAcc, ws) => wAcc + ws.worktrees.filter((wt) => wt.isDirty || wt.isRunning || wt.isBlocked).length,
            0
          ),
        0
      );
      return {
        title: 'Project Workspaces',
        icon: FolderGit2,
        iconColor: 'text-white/80',
        badge: projectGroups[0]?.isMultiRepo ? 'MULTI-REPO' : 'MONOREPO',
        badgeClass: 'bg-white/10 text-white/80 border-white/20',
        searchPlaceholder: 'Search workspaces, branches...',
        allLabel: 'All Worktrees',
        allCount: totalWorktrees,
        activeLabel: 'Active Worktrees',
        activeCount: activeWorktrees,
        footerStatus: activeWorkspace ? `${activeWorkspace.name} · Ready` : 'Ready',
      };
    }
  }
}

export function hasActiveRecentActivity(allSessions: any[], tasks: any[]): boolean {
  return (
    allSessions.some(
      (s) =>
        s.taskStatus === 'running' ||
        s.taskStatus === 'blocked' ||
        s.messages?.some((m: any) => m.runStatus === 'running' || m.runStatus === 'blocked')
    ) ||
    tasks.some(
      (t) =>
        t.status === 'running' ||
        t.status === 'Active' ||
        t.status === 'Blocked' ||
        String(t.status).toLowerCase() === 'running' ||
        String(t.status).toLowerCase() === 'blocked' ||
        t.metadata?.approval_required === 'true'
    )
  );
}
