/**
 * Multi-Persona `@mention` Group Room client for Custos Mobile.
 */

import type { StableLogicalRpcClient } from '../rpc/StableLogicalRpcClient';

export interface PersonaRoomSummary {
  readonly id: string;
  readonly title: string;
  readonly member_persona_ids: readonly string[];
  readonly active_session_id?: string | null;
}

export interface PersonaRoomMessageDto {
  readonly id: string;
  readonly room_id: string;
  readonly sender: string;
  readonly sender_kind: 'owner' | 'persona' | 'system';
  readonly target_persona_id?: string | null;
  readonly root_request_id?: string | null;
  readonly content: string;
  readonly created_at: string;
}

export function isRoomHistoryPayload(
  v: unknown
): v is { messages: PersonaRoomMessageDto[] } {
  if (typeof v !== 'object' || v === null) return false;
  return Array.isArray((v as Record<string, unknown>)['messages']);
}

export class PersonaGroupRoomClient {
  constructor(private readonly rpc: StableLogicalRpcClient) {}

  createRoom(title: string, memberPersonaIds: string[], defaultWorkbench?: string) {
    return this.rpc.request<{ room: PersonaRoomSummary }>('v1.personas.rooms.create', {
      title,
      member_persona_ids: memberPersonaIds,
      default_workbench: defaultWorkbench,
    });
  }

  postMessage(roomId: string, content: string, rootRequestId?: string) {
    return this.rpc.request('v1.personas.rooms.post_message', {
      room_id: roomId,
      content,
      sender_kind: 'owner',
      root_request_id: rootRequestId,
    });
  }

  history(roomId: string, limit = 50) {
    return this.rpc.request<{ messages: PersonaRoomMessageDto[] }>(
      'v1.personas.rooms.history',
      { room_id: roomId, limit },
      isRoomHistoryPayload
    );
  }
}

