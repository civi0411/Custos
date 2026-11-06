import { Code2 } from 'lucide-react';
import type { WorkbenchContribution } from '@/types/workbench';

export const engineeringContribution: WorkbenchContribution = {
  id: 'engineering',
  canonicalId: 'coding',
  label: 'Engineering',
  icon: Code2,
  shortcut: '⌥1',
  builtin: true,
  color: '#38bdf8',
  description: 'Autonomous software engineering, Git worktrees, tests, and AST codebase intelligence.',
  supportedTabTypes: [
    'files',
    'changes',
    'activity',
    'kanban',
    'dag',
    'evidence',
    'worktrees',
    'agents',
    'terminal',
    'fleet',
  ],
  panes: [
    { id: 'files', title: 'Files & Changes', position: 'main', defaultWidth: 'full' },
    { id: 'activity', title: 'Activity', position: 'main', defaultWidth: 'half' },
    { id: 'terminal', title: 'Terminal', position: 'bottom', defaultWidth: 'full' },
    { id: 'worktrees', title: 'Worktree Manager', position: 'main', defaultWidth: 'full' },
    { id: 'agents', title: 'Agents & Harnesses', position: 'main', defaultWidth: 'full' },
    { id: 'dag', title: 'Workflow DAG', position: 'main', defaultWidth: 'full' },
    { id: 'evidence', title: 'Evidence & Reviews', position: 'main', defaultWidth: 'full' },
  ],
};
