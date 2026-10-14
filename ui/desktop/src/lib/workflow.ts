import { Session, WorkflowCard } from '@/types';
import type { Task } from '@/types/domain';

export interface WorkflowProjectionContext {
  tasksById?: Record<string, Task>;
  attentionCountByTask?: Record<string, number>;
}

/**
 * Projects a canonical WorkflowCard from real backend session/task state.
 * Avoids transcript-based heuristics and reflects daemon-owned execution states.
 */
export function sessionToWorkflowCard(
  s: Session,
  context?: WorkflowProjectionContext
): WorkflowCard {
  const boundTask = s.taskId && context?.tasksById ? context.tasksById[s.taskId] : undefined;

  // Determine Workbench lens
  let workbench: 'copilot' | 'engineering' | 'research' = 'engineering';
  if (s.pack === 'research') {
    workbench = 'research';
  } else if (s.pack === 'assistant' || s.pack === 'copilot') {
    workbench = 'copilot';
  }

  // Kind determination
  const hasTask = !!boundTask || !!s.taskId;
  const hasChat = Array.isArray(s.messages) && s.messages.length > 0;
  const kind = hasTask && hasChat ? 'hybrid' : hasTask ? 'task' : 'session';

  // Attention & Status determination
  const rawStatus = boundTask
    ? String(boundTask.status).toLowerCase()
    : s.taskStatus
    ? String(s.taskStatus).toLowerCase()
    : 'idle';

  const needsAttention =
    (boundTask && context?.attentionCountByTask && (context.attentionCountByTask[boundTask.id] || 0) > 0) ||
    Boolean((s as any).needsAttention) ||
    rawStatus === 'blocked';

  let normalizedStatus: WorkflowCard['status'] = 'idle';
  if (needsAttention) {
    normalizedStatus = 'blocked';
  } else if (rawStatus === 'running' || rawStatus === 'active' || (s as any).isRunning) {
    normalizedStatus = 'running';
  } else if (rawStatus === 'done' || rawStatus === 'succeeded' || rawStatus === 'completed') {
    normalizedStatus = 'done';
  } else if (rawStatus === 'failed') {
    normalizedStatus = 'failed';
  } else if (rawStatus === 'draft') {
    normalizedStatus = 'draft';
  }

  return {
    id: s.id,
    title: s.title || 'Untitled Session',
    preview: s.preview,
    pack: s.pack,
    workbench,
    kind,
    status: normalizedStatus,
    hasChat,
    hasTask,
    taskId: s.taskId,
    parentId: (s as any).parentSessionId,
    costTokens: (s as any).costTokens || 0,
    needsAttention,
    createdAt: (s as any).createdAt || s.time,
    updatedAt: (s as any).updatedAt || s.time,
    session: s,
  };
}
