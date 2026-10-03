import { invoke } from '@tauri-apps/api/core';
import { ExecutionPermit, OperationalMode, RiskLevel, Task, TaskSpan, TaskStatus } from '../types';
import { SAMPLE_DIFF } from '../data/constants';

let mockTasks: Task[] = [
  {
    id: 'tsk_01jk98a1m4z0',
    title: 'Implement verified JWT payload decoder with domain validation',
    status: 'Succeeded',
    epoch: 3,
    created_at: new Date(Date.now() - 3600000).toISOString(),
    updated_at: new Date(Date.now() - 1800000).toISOString(),
    metadata: { mode: 'code', origin: 'custos-cli', verified: true },
    result_summary: 'JWT payload decoder compiled, pass 14 test fixtures, committed to crates/runtime',
  },
  {
    id: 'tsk_01jk99c4b7x2',
    title: 'Deliberate and benchmark SQLite WAL mode vs memory-mapped WAL',
    status: 'Running',
    epoch: 2,
    created_at: new Date(Date.now() - 1200000).toISOString(),
    updated_at: new Date(Date.now() - 300000).toISOString(),
    metadata: { mode: 'research', origin: 'custos-cli' },
  },
  {
    id: 'tsk_01jk99f9v3k1',
    title: 'Coordinate multi-agent policy audit across workflow recipes',
    status: 'Queued',
    epoch: 1,
    created_at: new Date(Date.now() - 600000).toISOString(),
    updated_at: new Date(Date.now() - 600000).toISOString(),
    metadata: { mode: 'assistant', origin: 'custos-cli' },
  },
];

let mockSpans: Record<string, TaskSpan[]> = {
  tsk_01jk98a1m4z0: [
    { id: 1, task_id: 'tsk_01jk98a1m4z0', name: 'context_recipe_compile', stage: 'Inspection', started_at: '2026-10-02T14:10:00Z', duration_ms: 120, details: 'Resolved 34 workspace dependencies' },
    { id: 2, task_id: 'tsk_01jk98a1m4z0', name: 'trajectory_synthesis', stage: 'Planning', started_at: '2026-10-02T14:10:01Z', duration_ms: 840, details: 'LLM generated AST patch and validation suite' },
    { id: 3, task_id: 'tsk_01jk98a1m4z0', name: 'execution_permit_grant', stage: 'Governance', started_at: '2026-10-02T14:10:04Z', duration_ms: 45, details: 'Approved by human operator' },
    { id: 4, task_id: 'tsk_01jk98a1m4z0', name: 'worktree_commit', stage: 'Commit', started_at: '2026-10-02T14:10:06Z', duration_ms: 210, details: 'Sandbox patch verified and applied' },
  ],
  tsk_01jk99c4b7x2: [
    { id: 101, task_id: 'tsk_01jk99c4b7x2', name: 'topology_investigation', stage: 'Inspection', started_at: '2026-10-02T14:40:00Z', duration_ms: 340, details: 'Profiling disk I/O on sqlite store' },
    { id: 102, task_id: 'tsk_01jk99c4b7x2', name: 'benchmark_execution', stage: 'Running', started_at: '2026-10-02T14:40:02Z', duration_ms: 1200, details: 'Testing 10,000 concurrent writes' },
  ],
};

let mockPermits: ExecutionPermit[] = [];

type Listener = () => void;
const listeners = new Set<Listener>();

function notifyListeners() {
  listeners.forEach((l) => l());
}

export function subscribeToApi(listener: Listener) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function isTauri(): boolean {
  return typeof window !== 'undefined' && Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

export const CustosApi = {
  async listTasks(): Promise<Task[]> {
    if (isTauri()) {
      try {
        return await invoke<Task[]>('list_tasks');
      } catch (err) {
        console.warn('Tauri invoke list_tasks failed, falling back to mock:', err);
      }
    }
    return [...mockTasks];
  },

  async getTask(id: string): Promise<Task | null> {
    if (isTauri()) {
      try {
        return await invoke<Task>('get_task', { id });
      } catch (err) {
        console.warn('Tauri invoke get_task failed, falling back to mock:', err);
      }
    }
    const found = mockTasks.find((t) => t.id === id);
    return found ? { ...found } : null;
  },

  async createTask(
    title: string,
    mode: OperationalMode = 'Code',
    metadata?: Record<string, unknown>
  ): Promise<Task> {
    if (isTauri()) {
      try {
        const res = await invoke<Task>('create_task', {
          title,
          metadata: JSON.stringify({ mode: mode.toLowerCase(), origin: 'custos-cli', ...metadata }),
        });
        notifyListeners();
        return res;
      } catch (err) {
        console.warn('Tauri invoke create_task failed, falling back to mock:', err);
      }
    }

    const newTask: Task = {
      id: `tsk_${Date.now().toString(36)}${Math.random().toString(36).substring(2, 6)}`,
      title,
      status: 'Draft',
      epoch: 1,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      metadata: {
        mode: mode.toLowerCase(),
        origin: 'custos-cli',
        ...metadata,
      },
    };

    mockTasks = [newTask, ...mockTasks];
    mockSpans[newTask.id] = [
      {
        id: Math.floor(Math.random() * 10000),
        task_id: newTask.id,
        name: 'task_created',
        stage: 'Initialization',
        started_at: new Date().toISOString(),
        duration_ms: 10,
        details: `Task registered with mode [${mode}]`,
      },
    ];

    notifyListeners();
    return newTask;
  },

  async advanceTask(id: string, status: TaskStatus, rationale?: string): Promise<Task> {
    if (isTauri()) {
      try {
        const res = await invoke<Task>('advance_task', { id, status, rationale });
        notifyListeners();
        return res;
      } catch (err) {
        console.warn('Tauri invoke advance_task failed, falling back to mock:', err);
      }
    }

    const task = mockTasks.find((t) => t.id === id);
    if (!task) {
      throw new Error(`Task with id '${id}' not found`);
    }

    task.status = status;
    task.epoch += 1;
    task.updated_at = new Date().toISOString();
    if (rationale) {
      task.rationale = rationale;
    }

    if (!mockSpans[id]) {
      mockSpans[id] = [];
    }
    mockSpans[id].push({
      id: Math.floor(Math.random() * 10000),
      task_id: id,
      name: `advanced_to_${status.toLowerCase()}`,
      stage: status,
      started_at: new Date().toISOString(),
      duration_ms: 45,
      details: rationale || `Advanced to ${status}`,
    });

    notifyListeners();
    return { ...task };
  },

  async completeTask(id: string, summary: string = 'Completed via Custos CLI'): Promise<Task> {
    if (isTauri()) {
      try {
        const res = await invoke<Task>('complete_task', { id, summary });
        notifyListeners();
        return res;
      } catch (err) {
        console.warn('Tauri invoke complete_task failed, falling back to mock:', err);
      }
    }

    const task = mockTasks.find((t) => t.id === id);
    if (!task) throw new Error(`Task with id '${id}' not found`);

    task.status = 'Succeeded';
    task.epoch += 1;
    task.updated_at = new Date().toISOString();
    task.result_summary = summary;

    if (!mockSpans[id]) mockSpans[id] = [];
    mockSpans[id].push({
      id: Math.floor(Math.random() * 10000),
      task_id: id,
      name: 'task_completed',
      stage: 'Completion',
      started_at: new Date().toISOString(),
      duration_ms: 50,
      details: summary,
    });

    notifyListeners();
    return { ...task };
  },

  async cancelTask(id: string, reason: string = 'Cancelled via Custos CLI'): Promise<Task> {
    if (isTauri()) {
      try {
        const res = await invoke<Task>('cancel_task', { id, reason });
        notifyListeners();
        return res;
      } catch (err) {
        console.warn('Tauri invoke cancel_task failed, falling back to mock:', err);
      }
    }

    const task = mockTasks.find((t) => t.id === id);
    if (!task) throw new Error(`Task with id '${id}' not found`);

    task.status = 'Cancelled';
    task.epoch += 1;
    task.updated_at = new Date().toISOString();
    task.failure_reason = reason;

    if (!mockSpans[id]) mockSpans[id] = [];
    mockSpans[id].push({
      id: Math.floor(Math.random() * 10000),
      task_id: id,
      name: 'task_cancelled',
      stage: 'Cancelled',
      started_at: new Date().toISOString(),
      duration_ms: 20,
      details: reason,
    });

    notifyListeners();
    return { ...task };
  },

  async listSpans(taskId: string): Promise<TaskSpan[]> {
    if (isTauri()) {
      try {
        return await invoke<TaskSpan[]>('list_spans', { taskId });
      } catch (err) {
        console.warn('Tauri invoke list_spans failed, falling back to mock:', err);
      }
    }
    return mockSpans[taskId] ? [...mockSpans[taskId]] : [];
  },

  createPermit(
    taskId: string,
    actionType: string = 'ExecutionPermit: Apply Code Diff',
    target: string = 'crates/runtime/src/handler.rs',
    risk: RiskLevel = 'Medium',
    filePath: string = SAMPLE_DIFF.filePath,
    oldCode: string = SAMPLE_DIFF.oldCode,
    newCode: string = SAMPLE_DIFF.newCode
  ): ExecutionPermit {
    const permit: ExecutionPermit = {
      id: `prm_${Date.now().toString(36)}`,
      taskId,
      actionType,
      target,
      risk,
      filePath,
      oldCode,
      newCode,
      status: 'pending',
      createdAt: new Date().toISOString(),
    };
    mockPermits = [permit, ...mockPermits];
    notifyListeners();
    return permit;
  },

  resolvePermit(permitId: string, approved: boolean): ExecutionPermit | null {
    const permit = mockPermits.find((p) => p.id === permitId);
    if (!permit) return null;
    permit.status = approved ? 'approved' : 'rejected';
    notifyListeners();
    return { ...permit };
  },

  getPendingPermits(): ExecutionPermit[] {
    return mockPermits.filter((p) => p.status === 'pending');
  },

  async explainArchitecture(query?: string): Promise<string> {
    if (isTauri()) {
      try {
        return await invoke<string>('explain_architecture', { query });
      } catch (err) {
        console.warn('Tauri invoke explain_architecture failed, falling back to mock:', err);
      }
    }

    return `=== Custos Architecture & End-to-End Vertical Slice ===
[Domain Core]: custos-domain provides immutable Task, Status transitions, and Epoch Concurrency
[Provider SDK]: custos-provider provides pluggable LLM connectors (OpenAI, Anthropic, Gemini, Local)
[Daemon Layer]: custos-daemon hosts LocalApiClient over JSON-RPC Unix/Pipe IPC with SQLite audit store
[Isolation]: Sandboxed worktrees guarantee code execution never escapes to production files without Permit
[Human Governance]: ExecutionPermit enforcement requires explicit human approval for mutating worktrees
Query context: ${query || 'Full vertical slice summary'}`;
  },
};
