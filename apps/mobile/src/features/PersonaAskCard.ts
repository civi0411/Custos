/**
 * 1-Tap Decision Card & Biometric Gate for Sovereign Personas on Custos Mobile.
 */

export interface PersonaAskOption {
  readonly id: string;
  readonly label: string;
  readonly recommended?: boolean;
}

export interface PersonaAskQuestion {
  readonly id: string;
  readonly prompt: string;
  readonly multi_select?: boolean;
  readonly options: readonly PersonaAskOption[];
}

export interface PersonaItem {
  readonly id: string;
  readonly persona_id: string;
  readonly bucket: 'needs_you' | 'up_next' | 'handled';
  readonly kind:
    | 'permit_approval'
    | 'hook_permission'
    | 'memory_proposal'
    | 'look_around_finding'
    | 'task_result';
  readonly title: string;
  readonly summary: string;
  readonly ask_questions?: readonly PersonaAskQuestion[];
  readonly task_id?: string;
  readonly priority?: number;
}

export const NEEDS_YOU_VISIBLE_CAP = 5;

/** Keeps approvals first (never folded) and caps remaining items at 5 before "Show N more". */
export function foldNeedsYouItems(items: readonly PersonaItem[]): {
  visible: PersonaItem[];
  hiddenCount: number;
} {
  const approvals = items.filter(
    (i) => i.kind === 'permit_approval' || i.kind === 'hook_permission'
  );
  const others = items.filter(
    (i) => i.kind !== 'permit_approval' && i.kind !== 'hook_permission'
  );
  const remainingSlots = Math.max(0, NEEDS_YOU_VISIBLE_CAP - approvals.length);
  const visible = [...approvals, ...others.slice(0, remainingSlots)];
  const hiddenCount = Math.max(0, others.length - remainingSlots);
  return { visible, hiddenCount };
}

export function isPersonaItem(v: unknown): v is PersonaItem {
  if (typeof v !== 'object' || v === null) return false;
  const o = v as Record<string, unknown>;
  return (
    typeof o['id'] === 'string' &&
    typeof o['persona_id'] === 'string' &&
    typeof o['bucket'] === 'string' &&
    typeof o['kind'] === 'string' &&
    typeof o['title'] === 'string'
  );
}

