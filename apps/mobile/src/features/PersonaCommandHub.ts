/**
 * Sovereign Persona Command Hub:
 * Unifies Needs You / Up Next / Omni-Ask (@mention) over `StableLogicalRpcClient`.
 */

import type { StableLogicalRpcClient } from '../rpc/StableLogicalRpcClient';
import { foldNeedsYouItems, isPersonaItem, type PersonaItem } from './PersonaAskCard';

export interface PersonaDescriptor {
  readonly id: string;
  readonly name: string;
  readonly role: string;
  readonly default_workbench: string;
  readonly enabled?: boolean;
}

export interface PersonaAskResponse {
  readonly status: string;
  readonly task_id?: string;
  readonly run_id?: string | null;
}

export function isPersonaListPayload(v: unknown): v is { personas: PersonaDescriptor[] } {
  if (typeof v !== 'object' || v === null) return false;
  const p = (v as Record<string, unknown>)['personas'];
  return Array.isArray(p);
}

export function isPersonaItemsPayload(v: unknown): v is { items: PersonaItem[] } {
  if (typeof v !== 'object' || v === null) return false;
  const items = (v as Record<string, unknown>)['items'];
  return Array.isArray(items) && items.every(isPersonaItem);
}

export class PersonaCommandHub {
  constructor(private readonly rpc: StableLogicalRpcClient) {}

  listPersonas() {
    return this.rpc.request<{ personas: PersonaDescriptor[] }>(
      'v1.personas.list',
      {},
      isPersonaListPayload
    );
  }

  async loadTriageBuckets(personaId?: string) {
    const res = await this.rpc.request<{ items: PersonaItem[] }>(
      'v1.personas.items.list',
      personaId ? { persona_id: personaId } : {},
      isPersonaItemsPayload
    );
    const needsYou = foldNeedsYouItems(res.items.filter((i) => i.bucket === 'needs_you'));
    const upNext = res.items.filter((i) => i.bucket === 'up_next').slice(0, 5);
    const handled = res.items.filter((i) => i.bucket === 'handled');
    return { needsYou, upNext, handled };
  }

  askPersona(
    personaId: string,
    prompt: string,
    options: { workbench?: string; workspaceId?: string; deviceId?: string } = {}
  ) {
    return this.rpc.request<PersonaAskResponse>('v1.personas.ask', {
      persona_id: personaId,
      prompt,
      caller_scope: 'mobile',
      is_paired_owner: true,
      workbench: options.workbench,
      workspace_id: options.workspaceId,
      device_id: options.deviceId,
    });
  }

  resolveItem(
    itemId: string,
    decision: 'accept' | 'dismiss' | 'start',
    selectedOptions: string[] = []
  ) {
    return this.rpc.request('v1.personas.items.resolve', {
      item_id: itemId,
      decision,
      selected_options: selectedOptions,
    });
  }
}

