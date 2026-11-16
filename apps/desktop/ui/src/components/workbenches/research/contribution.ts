import { FlaskConical } from 'lucide-react';
import type { WorkbenchContribution } from '@/types/workbench';

export const researchContribution: WorkbenchContribution = {
  id: 'research',
  canonicalId: 'research',
  label: 'Research',
  icon: FlaskConical,
  shortcut: '⌥2',
  builtin: false,
  color: '#a78bfa',
  description: 'Literature synthesis, reproducible experiments, notebooks, and citation verification.',
  supportedTabTypes: [
    'literature',
    'findings',
    'claims',
    'lab',
    'runs',
    'experiments',
    'training',
    'knowledge',
    'synthesis',
    'artifacts',
  ],
  panes: [
    { id: 'notebook', title: 'Notebook', position: 'main', defaultWidth: 'full' },
    { id: 'literature', title: 'Literature', position: 'main', defaultWidth: 'half' },
    { id: 'findings', title: 'Findings & Claims', position: 'main', defaultWidth: 'half' },
    { id: 'artifacts', title: 'Artifacts', position: 'main', defaultWidth: 'full' },
    { id: 'synthesis', title: 'Synthesis', position: 'main', defaultWidth: 'full' },
    { id: 'runs', title: 'Runs Ledger', position: 'bottom', defaultWidth: 'full' },
    { id: 'training', title: 'Training Runs', position: 'bottom', defaultWidth: 'full' },
  ],
};
