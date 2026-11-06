import React from 'react';
import {
  Code2,
  FlaskConical,
  Bot,
  TrendingUp,
  Download,
  AlertCircle,
} from 'lucide-react';

export type CanonicalWorkbenchId = 'assistant' | 'coding' | 'research' | 'quant' | 'unsupported';
export type WorkbenchId = CanonicalWorkbenchId | 'copilot' | 'engineering';

export interface WorkbenchContribution {
  id: WorkbenchId;
  canonicalId: CanonicalWorkbenchId;
  label: string;
  icon: React.ElementType;
  shortcut: string;
  builtin: boolean;
  description: string;
  supportedTabTypes: string[];
}

export const BUILTIN_CONTRIBUTIONS: WorkbenchContribution[] = [
  {
    id: 'engineering',
    canonicalId: 'coding',
    label: 'Engineering',
    icon: Code2,
    shortcut: '⌥1',
    builtin: true,
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
  },
  {
    id: 'research',
    canonicalId: 'research',
    label: 'Research',
    icon: FlaskConical,
    shortcut: '⌥2',
    builtin: false,
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
  },
  {
    id: 'copilot',
    canonicalId: 'assistant',
    label: 'Copilot',
    icon: Bot,
    shortcut: '⌥3',
    builtin: true,
    description: 'Personal agenda triage, communication drafting, and revocable grant automation.',
    supportedTabTypes: ['drafts', 'outbox', 'contacts', 'calendar', 'notes'],
  },
  {
    id: 'quant',
    canonicalId: 'quant',
    label: 'Quant',
    icon: TrendingUp,
    shortcut: '⌥4',
    builtin: false,
    description: 'Algorithmic alpha discovery, backtesting simulations, and financial risk audits.',
    supportedTabTypes: [
      'alpha',
      'alphastudio',
      'tearsheet',
      'risk_audit',
      'riskaudit',
      'execution',
    ],
  },
];

export const engineeringContribution = BUILTIN_CONTRIBUTIONS[0];
export const researchContribution = BUILTIN_CONTRIBUTIONS[1];
export const copilotContribution = BUILTIN_CONTRIBUTIONS[2];
export const quantContribution = BUILTIN_CONTRIBUTIONS[3];

/**
 * Normalizes aliases into their canonical identity:
 * - 'copilot' -> 'assistant'
 * - 'engineering' -> 'coding'
 */
export function canonicalWorkbenchId(id: string): CanonicalWorkbenchId {
  switch (id.toLowerCase()) {
    case 'copilot':
    case 'assistant':
      return 'assistant';
    case 'engineering':
    case 'coding':
      return 'coding';
    case 'research':
      return 'research';
    case 'quant':
      return 'quant';
    default:
      return 'unsupported';
  }
}

/**
 * Returns contributions filtered by the installed/enabled component inventory projection.
 */
export function getActiveContributions(
  installedIds: string[]
): WorkbenchContribution[] {
  const installedCanonical = new Set(
    installedIds.map((id) => canonicalWorkbenchId(id))
  );

  return BUILTIN_CONTRIBUTIONS.filter(
    (c) => c.builtin || installedCanonical.has(c.canonicalId)
  );
}

/**
 * Maps an Orca tab type to its responsible canonical workbench dynamically from registered contributions,
 * or null if it is a cross-cutting host tab (like chat, conversation, tasks, browser, notes, fleet).
 */
export function getWorkbenchForTabType(tabType: string): CanonicalWorkbenchId | null {
  for (const contribution of BUILTIN_CONTRIBUTIONS) {
    if (contribution.supportedTabTypes.includes(tabType)) {
      return contribution.canonicalId;
    }
  }
  return null;
}

export interface WorkbenchNotInstalledPlaceholderProps {
  workbenchId: string;
  onOpenLibrary?: () => void;
}

export const WorkbenchNotInstalledPlaceholder: React.FC<WorkbenchNotInstalledPlaceholderProps> = ({
  workbenchId,
  onOpenLibrary,
}) => {
  const canonical = canonicalWorkbenchId(workbenchId);
  const contribution = BUILTIN_CONTRIBUTIONS.find((c) => c.canonicalId === canonical);
  const displayName = contribution?.label || workbenchId;

  return (
    <div className="h-full w-full flex flex-col items-center justify-center p-8 text-center bg-[#121214] text-neutral-300 select-none">
      <div className="size-12 rounded-2xl bg-neutral-800/80 border border-neutral-700/60 flex items-center justify-center mb-4 text-neutral-400">
        <AlertCircle className="size-6 text-amber-400" />
      </div>
      <h3 className="text-base font-medium text-white mb-1">
        {displayName} Workbench Not Installed
      </h3>
      <p className="text-xs text-neutral-400 max-w-sm mb-6 leading-relaxed">
        {contribution?.description ||
          `The ${displayName} domain pack is an optional add-on that has not been enabled in this Custos environment.`}
      </p>
      {onOpenLibrary && (
        <button
          type="button"
          onClick={onOpenLibrary}
          className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-md bg-neutral-800 hover:bg-neutral-700 text-xs font-medium text-white border border-neutral-700 transition cursor-pointer"
        >
          <Download className="size-3.5" />
          Open Workbench Library
        </button>
      )}
    </div>
  );
};
