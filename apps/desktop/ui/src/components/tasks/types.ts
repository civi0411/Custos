import type { TaskStatus } from '@/types/domain';

export type TaskViewMode = 'board' | 'list';

export type TaskBucket = 'draft' | 'running' | 'attention' | 'done';

export interface TaskFilterOptions {
  query: string;
  packId: 'all' | 'code' | 'research' | 'copilot';
  status: 'all' | 'draft' | 'running' | 'blocked' | 'done';
}

export function getTaskBucket(status: TaskStatus): TaskBucket {
  const s = String(status).toLowerCase();
  if (s === 'running' || s === 'active' || s === 'completing') return 'running';
  if (s === 'blocked' || s === 'attention' || s === 'failed') return 'attention';
  if (s === 'done' || s === 'succeeded') return 'done';
  return 'draft';
}

export function getPackInfo(packId?: string): { label: string; badgeClass: string; domain: string } {
  const p = (packId || '').toLowerCase();
  if (p.includes('code') || p.includes('eng')) {
    return {
      label: 'Engineering',
      badgeClass: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30',
      domain: 'code',
    };
  }
  if (p.includes('research') || p.includes('science')) {
    return {
      label: 'Research',
      badgeClass: 'bg-purple-500/10 text-purple-400 border-purple-500/30',
      domain: 'research',
    };
  }
  return {
    label: 'Assistant',
    badgeClass: 'bg-sky-500/10 text-sky-400 border-sky-500/30',
    domain: 'copilot',
  };
}
