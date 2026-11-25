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
  packageState?: 'disabled' | 'not_installed';
  activating?: boolean;
  error?: string | null;
  onActivate?: () => void;
  onOpenLibrary?: () => void;
}

export const WorkbenchNotInstalledPlaceholder: React.FC<WorkbenchNotInstalledPlaceholderProps> = ({
  workbenchId,
  packageState = 'not_installed',
  activating = false,
  error,
  onActivate,
  onOpenLibrary,
}) => {
  const canonical = canonicalWorkbenchId(workbenchId);
  const contribution = BUILTIN_CONTRIBUTIONS.find((c) => c.canonicalId === canonical);
  const displayName = contribution?.label || workbenchId;
  const isDisabled = packageState === 'disabled';

  return (
    <div className="h-full w-full flex flex-col items-center justify-center p-8 text-center bg-[#121214] text-neutral-300 select-none">
      <div className="size-12 rounded-2xl bg-neutral-800/80 border border-neutral-700/60 flex items-center justify-center mb-4 text-neutral-400">
        <AlertCircle className={`size-6 ${isDisabled ? 'text-sky-400' : 'text-amber-400'}`} />
      </div>
      <h3 className="text-base font-medium text-white mb-1">
        {isDisabled ? `${displayName} Workbench Disabled` : `${displayName} Workbench Not Installed`}
      </h3>
      <p className="text-xs text-neutral-400 max-w-sm mb-5 leading-relaxed">
        {isDisabled
          ? `The ${displayName} workbench is installed on this profile but currently disabled. Re-enable it to restore this view.`
          : contribution?.description ||
            `The ${displayName} domain workbench is an on-demand add-on that must be installed and enabled before use.`}
      </p>
      {error && (
        <div className="mb-4 max-w-sm rounded-md border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-[11px] text-red-200">
          {error}
        </div>
      )}
      <div className="flex items-center gap-2">
        {onActivate && (
          <button
            type="button"
            disabled={activating}
            onClick={onActivate}
            className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-md bg-emerald-600/85 hover:bg-emerald-500 disabled:opacity-50 text-xs font-medium text-white transition cursor-pointer"
          >
            <Download className="size-3.5" />
            {activating
              ? 'Activating...'
              : isDisabled
              ? `Enable ${displayName}`
              : `Install & Enable ${displayName}`}
          </button>
        )}
        {onOpenLibrary && (
          <button
            type="button"
            onClick={onOpenLibrary}
            className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-md bg-neutral-800 hover:bg-neutral-700 text-xs font-medium text-white border border-neutral-700 transition cursor-pointer"
          >
            Open Workbench Library
          </button>
        )}
      </div>
    </div>
  );
};
