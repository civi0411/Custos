import type { ActiveWorkbench } from '@/components/shell/AppHeader';

export interface LensMeta {
  title: string;
  taskLabel: string;
  iconColor: string;
  activeColor: string;
  avatarText: string;
  avatarBg: string;
  avatarColor: string;
  badge: string;
  modelName: string;
  placeholder: string;
}

export const LENS_METADATA: Record<ActiveWorkbench, LensMeta> = {
  copilot: {
    title: 'Copilot Sessions',
    taskLabel: 'New Conversation',
    iconColor: 'workbench-accent',
    activeColor: 'workbench-accent-bg',
    avatarText: 'CL',
    avatarBg: 'workbench-accent-bg',
    avatarColor: 'text-fg-on-emphasis',
    badge: 'Copilot',
    modelName: 'Custos mediated · executor visible per run',
    placeholder: 'Ask, plan, or continue the current task…'
  },
  engineering: {
    title: 'Engineering Sessions',
    taskLabel: 'New Engineering Task',
    iconColor: 'workbench-accent',
    activeColor: 'workbench-accent-bg',
    avatarText: 'CV',
    avatarBg: 'workbench-accent-bg',
    avatarColor: 'text-fg-on-emphasis',
    badge: 'Coding',
    modelName: 'Repository · worktree · verification',
    placeholder: 'Ask about the repository, propose a change, or continue coding…'
  },
  research: {
    title: 'Research Sessions',
    taskLabel: 'New Research Hypothesis',
    iconColor: 'workbench-accent',
    activeColor: 'workbench-accent-bg',
    avatarText: 'RL',
    avatarBg: 'workbench-accent-bg',
    avatarColor: 'text-fg-on-emphasis',
    badge: 'Research',
    modelName: 'Sources · claims · experiments',
    placeholder: 'Ask a question, inspect evidence, or design an experiment…'
  }
};
