import React from 'react';
import { Download, AlertCircle } from 'lucide-react';

import type { WorkbenchContribution, CanonicalWorkbenchId, WorkbenchId } from '@/types/workbench';
export type { WorkbenchContribution, CanonicalWorkbenchId, WorkbenchId };

import { engineeringContribution } from './coding/contribution';
import { researchContribution } from './research/contribution';
import { copilotContribution } from './assistant/contribution';
import { quantContribution } from './quant/contribution';

export {
  engineeringContribution,
  researchContribution,
  copilotContribution,
  quantContribution,
};

export const BUILTIN_CONTRIBUTIONS: WorkbenchContribution[] = [
  engineeringContribution,
  researchContribution,
  copilotContribution,
  quantContribution,
];

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
