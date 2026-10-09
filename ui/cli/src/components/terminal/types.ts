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
    mode: 'Assistant',
    name: 'Assistant Engine',
    slug: 'custos-assistant',
    color: '#10b981',
    tagline: 'Điều phối quy trình tác vụ tự động & quản trị runtime',
    capabilities: ['Task Lifecycle Supervision', 'Epoch Optimistic Locking', 'SQLite Audit Spans'],
  },
];

export const getModeSlug = (m?: OperationalMode | string | null): string => {
  if (!m) return 'custos';
  const lower = m.toLowerCase();
  if (lower === 'standard' || lower === 'custos') return 'custos';
  if (lower === 'assitant' || lower === 'assistant') return 'custos-assistant';
  if (lower === 'code') return 'custos-code';
  if (lower === 'research') return 'custos-research';
  return `custos-${lower}`;
};

export interface SlashItem {
  id: string;
  name: string;
  description: string;
  category: 'Chế độ hoạt động' | 'Tác vụ & Quy trình' | 'Hệ thống';
  icon:
    | 'code'
    | 'research'
    | 'assistant'
    | 'custos'
    | 'vibe'
    | 'tasks'
    | 'create'
    | 'advance'
    | 'diff'
    | 'permit'
    | 'status'
    | 'clear'
    | 'help'
    | 'mode';
  color: string;
  type: 'mode' | 'command' | 'submenu';
  mode?: OperationalMode | 'custos';
  command?: string;
}

export const MODE_SLASH_ITEMS: SlashItem[] = [
  {
    id: 'mode-code',
    name: 'code',
    description: 'Lập trình, sửa lỗi & Sandbox Worktree',
    category: 'Chế độ hoạt động',
    icon: 'code',
    color: '#38bdf8',
    type: 'mode',
    mode: 'Code',
  },
  {
    id: 'mode-research',
    name: 'research',
    description: 'Điều tra kiến trúc, topology & đối soát bằng chứng',
    category: 'Chế độ hoạt động',
    icon: 'research',
    color: '#f59e0b',
    type: 'mode',
    mode: 'Research',
  },
  {
    id: 'mode-assistant',
    name: 'assistant',
    description: 'Điều phối quy trình tác vụ & Epoch Lock',
    category: 'Chế độ hoạt động',
    icon: 'assistant',
    color: '#10b981',
    type: 'mode',
    mode: 'Assistant',
  },
];

export const ROOT_SLASH_ITEMS: SlashItem[] = [
  {
    id: 'submenu-mode',
    name: 'mode',
    description: 'Chọn Code, Research hoặc Assistant',
    category: 'Chế độ hoạt động',
    icon: 'mode',
    color: '#38bdf8',
    type: 'submenu',
  },
  {
    id: 'action-vibe',
    name: 'vibe',
    description: 'Khởi chạy quy trình Vibe Coding tương tác từng bước',
    category: 'Tác vụ & Quy trình',
    icon: 'vibe',
    color: '#ec4899',
    type: 'command',
    command: 'vibe',
  },
  {
    id: 'action-tasks',
    name: 'tasks',
    description: 'Hiển thị bảng danh sách nhiệm vụ & SQLite spans',
    category: 'Tác vụ & Quy trình',
    icon: 'tasks',
    color: '#818cf8',
    type: 'command',
    command: 'list',
  },
  {
    id: 'action-create',
    name: 'create',
    description: 'Tạo một nhiệm vụ agentic mới vào hàng đợi',
    category: 'Tác vụ & Quy trình',
    icon: 'create',
    color: '#34d399',
    type: 'command',
    command: 'create',
  },
  {
    id: 'action-advance',
    name: 'advance',
    description: 'Chuyển tiến trình task sang trạng thái tiếp theo',
    category: 'Tác vụ & Quy trình',
    icon: 'advance',
    color: '#fbbf24',
    type: 'command',
    command: 'advance',
  },
  {
    id: 'action-diff',
    name: 'diff',
    description: 'Xem bản so sánh Unified AST Diff Review',
    category: 'Tác vụ & Quy trình',
    icon: 'diff',
    color: '#f97316',
    type: 'command',
    command: 'diff',
  },
  {
    id: 'action-permit',
    name: 'permit',
    description: 'Xem xét & phê duyệt Giấy phép Thực thi (Human Permit)',
    category: 'Tác vụ & Quy trình',
    icon: 'permit',
    color: '#ef4444',
    type: 'command',
    command: 'permit',
  },
  {
    id: 'action-status',
    name: 'status',
    description: 'Kiểm tra trạng thái Daemon, Sandbox & Lock Guard',
    category: 'Tác vụ & Quy trình',
    icon: 'status',
    color: '#06b6d4',
    type: 'command',
    command: 'status',
  },

  // Hệ thống & Trợ giúp (System)
  {
    id: 'system-clear',
    name: 'clear',
    description: 'Làm sạch màn hình terminal',
    category: 'Hệ thống',
    icon: 'clear',
    color: '#94a3b8',
    type: 'command',
    command: 'clear',
  },
  {
    id: 'system-help',
    name: 'help',
    description: 'Bảng tra cứu danh sách lệnh và phím tắt',
    category: 'Hệ thống',
    icon: 'help',
    color: '#cbd5e1',
    type: 'command',
    command: 'help',
  },
];

export const SLASH_ITEMS: SlashItem[] = [...ROOT_SLASH_ITEMS, ...MODE_SLASH_ITEMS];

export interface VibeSessionState {
  step: 'none' | 'mode' | 'goal' | 'permit';
  mode?: OperationalMode;
  goal?: string;
  task?: Task;
  permit?: ExecutionPermit;
}
