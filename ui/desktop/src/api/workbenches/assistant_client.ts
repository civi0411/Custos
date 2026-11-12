import type { AssistantDraft, OutboxActionItem } from '@/types/domain';
import { fromDaemon, toDaemon } from '../normalizers';
import type { DaemonTransport } from './quant_client';

export class AssistantClient {
  constructor(private transport: DaemonTransport) {}

  async listDrafts(sessionId?: string, taskId?: string): Promise<AssistantDraft[]> {
    const drafts = await this.transport.request<unknown[]>('v1.assistant.drafts.list', {
      session_id: sessionId,
      task_id: taskId,
    });
    return drafts.map((d) => fromDaemon<AssistantDraft>(d));
  }

  async getDraft(id: string): Promise<AssistantDraft> {
    return fromDaemon<AssistantDraft>(await this.transport.request<unknown>('v1.assistant.drafts.get', { id }));
  }

  async saveDraft(draft: Partial<AssistantDraft> & { id?: string; title: string; kind?: string; content: string }): Promise<AssistantDraft> {
    const payload = {
      ...draft,
      id: draft.id || `draft-${Date.now()}`,
      kind: draft.kind || 'email',
    };
    return fromDaemon<AssistantDraft>(await this.transport.request<unknown>('v1.assistant.drafts.save', toDaemon(payload)));
  }

  async deleteDraft(id: string): Promise<boolean> {
    const res = await this.transport.request<{ deleted: boolean }>('v1.assistant.drafts.delete', { id });
    return Boolean(res?.deleted);
  }

  async listOutbox(taskId?: string): Promise<OutboxActionItem[]> {
    const items = await this.transport.request<unknown[]>('v1.assistant.outbox.list', { task_id: taskId });
    return items.map((i) => {
      const value = fromDaemon<OutboxActionItem>(i);
      return {
        ...value,
        task_id: value.task_id ?? (value as any).taskId ?? '',
        action_type: value.action_type ?? (value as any).actionType ?? '',
        payload_summary: value.payload_summary ?? (value as any).payloadSummary ?? '',
        payload_digest: value.payload_digest ?? (value as any).payloadDigest ?? '',
        permit_id: value.permit_id ?? (value as any).permitId,
        receipt_id: value.receipt_id ?? (value as any).receiptId,
        created_at: value.created_at ?? (value as any).createdAt ?? '',
      };
    });
  }

  async approveOutbox(id: string): Promise<any> {
    return this.transport.request<any>('v1.assistant.outbox.approve', { id });
  }

  async dispatchOutbox(id: string): Promise<any> {
    return this.transport.request<any>('v1.assistant.outbox.dispatch', { id });
  }

  async createOutbox(params: { action_type?: string; task_id?: string; content?: string }): Promise<any> {
    return this.transport.request<any>('v1.assistant.outbox.create', params);
  }
}
