import { TrendingUp } from 'lucide-react';
import type { WorkbenchContribution } from '@/types/workbench';

export const quantContribution: WorkbenchContribution = {
  id: 'quant',
  canonicalId: 'quant',
  label: 'Quant',
  icon: TrendingUp,
  shortcut: '⌥4',
  builtin: false,
  color: '#fbbf24',
  description: 'Algorithmic alpha discovery, backtesting simulations, and financial risk audits.',
  supportedTabTypes: [
    'alpha',
    'alphastudio',
    'tearsheet',
    'risk_audit',
    'riskaudit',
    'execution',
  ],
  panes: [
    { id: 'alphastudio', title: 'Alpha Studio', position: 'main', defaultWidth: 'half' },
    { id: 'tearsheet', title: 'Tearsheet', position: 'main', defaultWidth: 'half' },
    { id: 'risk_audit', title: 'Risk Audit', position: 'bottom', defaultWidth: 'half' },
    { id: 'execution', title: 'Execution', position: 'bottom', defaultWidth: 'half' },
  ],
};
