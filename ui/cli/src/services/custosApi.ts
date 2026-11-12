import { invoke } from '@tauri-apps/api/core';
import { ExecutionPermit, OperationalMode, RiskLevel, Task, TaskSpan, TaskStatus } from '../types';

type Listener = () => void;
const listeners = new Set<Listener>();
let activePermits: ExecutionPermit[] = [];

function notifyListeners(): void {
  listeners.forEach((listener) => listener());
}

export function subscribeToApi(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function isTauriEnvironment(): boolean {
  return (
    typeof window !== 'undefined' &&
    Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
  );
}

function getDaemonHttpPort(): string {
  if (typeof window !== 'undefined') {
    const queryPort = new URLSearchParams(window.location.search).get('daemon_port');
    if (queryPort && /^\d+$/.test(queryPort)) return queryPort;
    try {
      const saved = window.localStorage?.getItem('custos_daemon_port');
      if (saved && /^\d+$/.test(saved)) return saved;
    } catch {
      // Ignore storage access errors in restricted webviews.
    }
  }
  const envPort = (import.meta as unknown as { env?: Record<string, string | undefined> }).env
    ?.VITE_CUSTOS_DAEMON_PORT;
  return envPort && /^\d+$/.test(envPort) ? envPort : '3000';
}

async function daemonRpc<T>(method: string, params: Record<string, unknown> = {}): Promise<T> {
  const port = getDaemonHttpPort();
  const endpoint = `http://127.0.0.1:${port}/api/request`;
  let response: Response;
  try {
    response = await fetch(endpoint, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        id: `cli_ui_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
        method,
        params,
      }),
    });
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    throw new Error(
      `Custos daemon offline at ${endpoint} (${detail}). Start custos-daemon or launch via CLI.`
    );
  }

  if (!response.ok) {
    throw new Error(`Custos daemon HTTP ${response.status} on ${method}`);
  }

  const envelope = (await response.json()) as { result?: T; error?: string };
  if (envelope.error) {
    throw new Error(envelope.error);
  }
  return envelope.result as T;
}

export const CustosApi = {
  async listTasks(): Promise<Task[]> {
    if (isTauriEnvironment()) {
      return await invoke<Task[]>('list_tasks');
    }
    return await daemonRpc<Task[]>('v1.tasks.list', {});
  },

  async getTask(id: string): Promise<Task | null> {
    if (isTauriEnvironment()) {
      return await invoke<Task | null>('get_task', { id });
    }
    try {
      return await daemonRpc<Task>('v1.tasks.get', { task_id: id });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      if (message.toLowerCase().includes('not found')) {
        return null;
      }
      throw err;
    }
  },

  async createTask(
    title: string,
    mode: OperationalMode = 'Code',
    metadata?: Record<string, unknown>
  ): Promise<Task> {
    const payloadMeta = {
      mode: mode.toLowerCase(),
      origin: 'custos-cli',
      ...metadata,
    };
    const task = isTauriEnvironment()
      ? await invoke<Task>('create_task', {
          title,
          metadata: JSON.stringify(payloadMeta),
        })
      : await daemonRpc<Task>('v1.tasks.create', {
          title,
          metadata: payloadMeta,
        });
    notifyListeners();
    return task;
  },

  async advanceTask(id: string, status: TaskStatus, rationale?: string): Promise<Task> {
    const task = isTauriEnvironment()
      ? await invoke<Task>('advance_task', { id, status, rationale })
      : await daemonRpc<Task>('v1.tasks.advance', {
          task_id: id,
          target_status: status,
          rationale,
        });
    notifyListeners();
    return task;
  },

  async completeTask(id: string, summary: string = 'Completed via Custos CLI'): Promise<Task> {
    const task = isTauriEnvironment()
      ? await invoke<Task>('complete_task', { id, summary })
      : await daemonRpc<Task>('v1.tasks.complete', {
          task_id: id,
          summary,
          evidence_claims: [],
        });
    notifyListeners();
    return task;
  },

  async cancelTask(id: string, reason: string = 'Cancelled via Custos CLI'): Promise<Task> {
    const task = isTauriEnvironment()
      ? await invoke<Task>('cancel_task', { id, reason })
      : await daemonRpc<Task>('v1.tasks.cancel', {
          task_id: id,
          reason,
        });
    notifyListeners();
    return task;
  },

  async listSpans(taskId: string): Promise<TaskSpan[]> {
    if (isTauriEnvironment()) {
      return await invoke<TaskSpan[]>('list_spans', { taskId });
    }
    return await daemonRpc<TaskSpan[]>('v1.tasks.spans', { task_id: taskId });
  },

  createPermit(
    taskId: string,
    actionType: string,
    target: string,
    risk: RiskLevel,
    filePath: string,
    oldCode: string,
    newCode: string
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
    activePermits = [permit, ...activePermits];
    notifyListeners();
    return permit;
  },

  resolvePermit(permitId: string, approved: boolean): ExecutionPermit | null {
    const permit = activePermits.find((p) => p.id === permitId);
    if (!permit) return null;
    permit.status = approved ? 'approved' : 'rejected';
    activePermits = activePermits.filter((p) => p.id !== permitId);
    notifyListeners();
    return { ...permit };
  },

  getPendingPermits(): ExecutionPermit[] {
    return activePermits.filter((p) => p.status === 'pending');
  },

  async explainArchitecture(query?: string): Promise<string> {
    if (isTauriEnvironment()) {
      return await invoke<string>('explain_architecture', { query });
    }
    const status = await daemonRpc<Record<string, unknown>>('v1.capabilities.list', {});
    return JSON.stringify(
      {
        query: query || 'daemon_capabilities',
        daemon: `http://127.0.0.1:${getDaemonHttpPort()}`,
        capabilities: status,
      },
      null,
      2
    );
  },
};
