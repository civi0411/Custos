import { OperationalMode, ExecutionPermit, Task } from '../../types';

export interface AvailableModeItem {
  mode: OperationalMode;
  name: string;
  slug: string;
  color: string;
  tagline: string;
  capabilities: string[];
}

export const AVAILABLE_MODES: AvailableModeItem[] = [
  {
    mode: 'Code',
    name: 'Code Engine',
    slug: 'custos-code',
    color: '#38bdf8',
    tagline: 'Lập trình, sửa lỗi & tái cấu trúc mã nguồn trong Sandbox',
    capabilities: ['Worktree Isolation & Sandboxing', 'AST Patch Synthesis', 'Automated Test Runner'],
  },
  {
    mode: 'Research',
    name: 'Research Engine',
    slug: 'custos-research',
    color: '#f59e0b',
    tagline: 'Điều tra kiến trúc, topology & suy luận chuyên sâu',
    capabilities: ['Repository Topology Mapping', 'Concurrency Invariants', 'Security Audit'],
  },
  {
    mode: 'Assitant',
    name: 'Assistant Engine',
    slug: 'custos-assistant',
    color: '#10b981',
    tagline: 'Điều phối quy trình tác vụ tự động & quản trị runtime',
    capabilities: ['Task Lifecycle Supervision', 'Epoch Optimistic Locking', 'SQLite Audit Spans'],
  },
];

export const getModeSlug = (m?: OperationalMode | string | null): string => {
  if (!m || m === 'standard' || m === 'custos') return 'custos';
  const lower = m.toLowerCase();
  if (lower === 'assitant' || lower === 'assistant') return 'custos-assistant';
  if (lower === 'code') return 'custos-code';
  if (lower === 'research') return 'custos-research';
  return `custos-${lower}`;
};

export interface SlashItem {
  id: string;
  name: string;
  icon: 'code' | 'research' | 'assistant' | 'custos';
  color: string;
  type: 'mode';
  mode: OperationalMode | 'custos';
}

export const SLASH_ITEMS: SlashItem[] = [
  {
    id: 'mode-code',
    name: 'custos-code',
    icon: 'code',
    color: '#38bdf8',
    type: 'mode',
    mode: 'Code',
  },
  {
    id: 'mode-research',
    name: 'custos-research',
    icon: 'research',
    color: '#f59e0b',
    type: 'mode',
    mode: 'Research',
  },
  {
    id: 'mode-assistant',
    name: 'custos-assistant',
    icon: 'assistant',
    color: '#10b981',
    type: 'mode',
    mode: 'Assitant',
  },
  {
    id: 'mode-custos',
    name: 'custos',
    icon: 'custos',
    color: '#e2e8f0',
    type: 'mode',
    mode: 'custos',
  },
];

export interface VibeSessionState {
  step: 'none' | 'mode' | 'goal' | 'permit';
  mode?: OperationalMode;
  goal?: string;
  task?: Task;
  permit?: ExecutionPermit;
}
