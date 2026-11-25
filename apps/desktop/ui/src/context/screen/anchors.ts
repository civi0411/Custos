import type { Screen } from '@/types/screen';
import { createCanvas } from '@/types/canvas';
import type { OrcaTab } from '@/components/workbenches/OrcaTabbedContainer';

export interface WorktreeDescriptor {
  id: string;
  name: string;
  branch?: string;
  path?: string;
  repoName?: string;
}

export interface TaskDescriptor {
  id: string;
  title: string;
  packId?: string;
}

export function buildWorktreeScreenTabs(worktree: WorktreeDescriptor, screenLabel: string): OrcaTab[] {
  return [
    {
      id: `conv_${worktree.id}`,
      type: 'conversation',
      title: `${screenLabel}: Chat`,
      url: `custos://chat?workspace=${worktree.id}`,
      workspaceId: worktree.id,
    },
    {
      id: `terminal_${worktree.id}`,
      type: 'terminal',
      title: `${screenLabel}: Terminal`,
      url: `custos://terminal?workspace=${worktree.id}`,
      workspaceId: worktree.id,
    },
  ];
}

export function createWorktreeScreen(
  worktree: WorktreeDescriptor,
  projectId: string,
  initialTabsList: OrcaTab[]
): Screen {
  const screenLabel = worktree.branch
    ? `${worktree.repoName ? `${worktree.repoName} @ ` : ''}${worktree.branch}`
    : worktree.name;
  const initialTabIds = initialTabsList.map((t) => t.id);

  return {
    id: `screen-wt-${worktree.id}`,
    name: screenLabel,
    icon: 'GitBranch',
    projectId,
    homeWorkbench: 'engineering',
    defaultWorkspaceId: worktree.id,
    anchor: {
      kind: 'worktree',
      id: worktree.id,
      label: screenLabel,
      path: worktree.path,
      repoName: worktree.repoName,
      branch: worktree.branch,
    },
    canvasState: createCanvas(initialTabIds),
    tabRegistry: {},
    tabs: initialTabsList,
    activeTabId: initialTabIds[0] || '',
    isPinned: false,
    createdAt: Date.now(),
    lastUsedAt: Date.now(),
  } as unknown as Screen;
}

export function buildTaskScreenTabs(task: TaskDescriptor, screenLabel: string): OrcaTab[] {
  return [
    {
      id: `task_${task.id}`,
      type: 'tasks',
      title: screenLabel,
      url: `custos://task?id=${task.id}`,
      taskId: task.id,
    },
    {
      id: `conv_task_${task.id}`,
      type: 'conversation',
      title: `${task.title} - Chat`,
      url: `custos://chat?taskId=${task.id}`,
      taskId: task.id,
    },
  ];
}

export function createTaskScreen(
  task: TaskDescriptor,
  projectId: string,
  initialTabsList: OrcaTab[]
): Screen {
  const screenLabel = `Task: ${task.title}`;
  const initialTabIds = initialTabsList.map((t) => t.id);

  return {
    id: `screen-task-${task.id}`,
    name: screenLabel,
    icon: 'CheckSquare',
    projectId,
    homeWorkbench:
      task.packId === 'code' ? 'engineering' : task.packId === 'research' ? 'research' : 'copilot',
    anchor: {
      kind: 'task',
      id: task.id,
      label: screenLabel,
    },
    canvasState: createCanvas(initialTabIds),
    tabRegistry: {},
    tabs: initialTabsList,
    activeTabId: initialTabIds[0] || '',
    isPinned: false,
    createdAt: Date.now(),
    lastUsedAt: Date.now(),
  };
}
