import { invoke } from '@tauri-apps/api/core';
import {
  Task,
  Session,
  SessionJournalEntry,
  Run,
  SessionMode,
  CreateTaskParams,
  StartRunParams,
  CancelRunParams,
  ExecutionWorkspace,
  CreateWorkspaceParams,
} from '../types/domain';
import { initialProjectData } from '../data/mockData';

export class DaemonClient {
  private isTauri: boolean;
  private mockWorkspaces?: ExecutionWorkspace[];

  constructor() {
    this.isTauri =
      typeof window !== 'undefined' &&
      (Boolean((window as any).__TAURI_INTERNALS__) ||
        Boolean((window as any).__TAURI__));
  }

  get isDemoMode(): boolean {
    return !this.isTauri;
  }

  /**
   * Core request dispatcher. Invokes in-process Tauri command or falls back gracefully.
   */
  async request<T>(method: string, params: Record<string, any> = {}): Promise<T> {
    if (this.isTauri) {
      try {
        return await invoke<T>('custos_request', { method, params });
      } catch (err: any) {
        console.error(`[DaemonClient] IPC Error on ${method}:`, err);
        throw new Error(typeof err === 'string' ? err : err?.message || JSON.stringify(err));
      }
    } else {
      console.warn(`[DaemonClient] Web mode: dispatching fallback for ${method}`, params);
      return this.handleWebFallback<T>(method, params);
    }
  }

  // ---------------------------------------------------------
  // Tasks API
  // ---------------------------------------------------------

  async createTask(params: CreateTaskParams): Promise<Task> {
    return this.request<Task>('v1.tasks.create', params);
  }

  async getTask(taskId: string): Promise<Task> {
    return this.request<Task>('v1.tasks.get', { task_id: taskId });
  }

  async listTasks(): Promise<Task[]> {
    return this.request<Task[]>('v1.tasks.list', {});
  }

  async cancelTask(taskId: string, reason?: string): Promise<Task> {
    return this.request<Task>('v1.tasks.cancel', { task_id: taskId, reason });
  }

  async advanceTask(taskId: string, phase: string, status?: string): Promise<Task> {
    return this.request<Task>('v1.tasks.advance', {
      task_id: taskId,
      phase,
      status,
    });
  }

  async completeTask(taskId: string, evidence?: any): Promise<Task> {
    return this.request<Task>('v1.tasks.complete', {
      task_id: taskId,
      evidence,
    });
  }

  // ---------------------------------------------------------
  // Workflow & Execution API
  // ---------------------------------------------------------

  async startRun(params: StartRunParams): Promise<Run> {
    return this.request<Run>('v1.workflow.start_run', params);
  }

  async cancelRun(params: CancelRunParams): Promise<void> {
    return this.request<void>('v1.workflow.cancel_run', params);
  }

  // ---------------------------------------------------------
  // Sessions & Chat Journal API
  // ---------------------------------------------------------

  async createSession(mode: SessionMode = 'supervised', taskId?: string): Promise<Session> {
    return this.request<Session>('v1.sessions.create', { mode, task_id: taskId });
  }

  async getSession(sessionId: string): Promise<Session> {
    return this.request<Session>('v1.sessions.get', { session_id: sessionId });
  }

  async listSessions(): Promise<Session[]> {
    return this.request<Session[]>('v1.sessions.list', {});
  }

  async getSessionJournal(sessionId: string): Promise<SessionJournalEntry[]> {
    return this.request<SessionJournalEntry[]>('v1.sessions.journal', {
      session_id: sessionId,
    });
  }

  async appendSessionMessage(
    sessionId: string,
    role: string,
    content: string
  ): Promise<void> {
    return this.request<void>('v1.sessions.message', {
      session_id: sessionId,
      role,
      content,
    });
  }

  async promoteSession(sessionId: string, targetMode: SessionMode): Promise<void> {
    return this.request<void>('v1.sessions.promote', {
      session_id: sessionId,
      mode: targetMode,
    });
  }

  // ---------------------------------------------------------
  // Bridge & OI Reasoning API
  // ---------------------------------------------------------

  async attachBridge(sessionId: string, target: string): Promise<any> {
    return this.request<any>('v1.bridge.attach', {
      session_id: sessionId,
      target,
    });
  }

  async steerBridge(
    sessionId: string,
    action: string,
    parameters: Record<string, any> = {}
  ): Promise<any> {
    return this.request<any>('v1.bridge.steer', {
      session_id: sessionId,
      action,
      parameters,
    });
  }

  async explainOI(taskId: string, budget?: any): Promise<any> {
    return this.request<any>('v1.oi.explain', {
      task_id: taskId,
      budget,
    });
  }

  // ---------------------------------------------------------
  // ExecutionWorkspace API (RFC 006 / OrCa Integration)
  // ---------------------------------------------------------

  async createWorkspace(params: CreateWorkspaceParams): Promise<ExecutionWorkspace> {
    return this.request<ExecutionWorkspace>('v1.workspaces.create', params);
  }

  async getWorkspace(workspaceId: string): Promise<ExecutionWorkspace> {
    return this.request<ExecutionWorkspace>('v1.workspaces.get', { workspace_id: workspaceId });
  }

  async listWorkspaces(): Promise<ExecutionWorkspace[]> {
    return this.request<ExecutionWorkspace[]>('v1.workspaces.list', {});
  }

  async archiveWorkspace(workspaceId: string, deletePhysical = false): Promise<void> {
    return this.request<void>('v1.workspaces.archive', {
      workspace_id: workspaceId,
      delete_physical: deletePhysical,
    });
  }

  // ---------------------------------------------------------
  // Browser dev / offline fallback simulator
  // ---------------------------------------------------------

  private handleWebFallback<T>(method: string, params: any): T {
    switch (method) {
      case 'v1.workspaces.list': {
        if (!this.mockWorkspaces) {
          this.mockWorkspaces = [
            {
              id: 'ws-main',
              name: 'main',
              kind: { type: 'git', repo_path: '/Users/mac/Project/AgentHub/Custos', branch: 'main' },
              path: '/Users/mac/Project/AgentHub/Custos',
              status: 'ready',
              lineage: {},
              metadata: { domain: 'engineering', assignedAgent: 'Human (Primary)', modifiedFilesCount: 0 },
              created_at: new Date().toISOString(),
              updated_at: new Date().toISOString(),
            },
            {
              id: 'ws-simd',
              name: 'feat/simd-dispatch',
              kind: { type: 'git', repo_path: '/Users/mac/Project/AgentHub/Custos', branch: 'feat/simd-dispatch' },
              path: '.worktrees/feat-simd-dispatch',
              status: 'ready',
              lineage: { base_commit: 'a3f2d1e' },
              metadata: { domain: 'engineering', assignedAgent: 'Claude Code (S2-Worker)', modifiedFilesCount: 3 },
              created_at: new Date(Date.now() - 45 * 60 * 1000).toISOString(),
              updated_at: new Date().toISOString(),
            },
            {
              id: 'ws-permits',
              name: 'fix/permits-race',
              kind: { type: 'git', repo_path: '/Users/mac/Project/AgentHub/Custos', branch: 'fix/permits-race' },
              path: '.worktrees/fix-permits-race',
              status: 'ready',
              lineage: { base_commit: 'a3f2d1e' },
              metadata: { domain: 'engineering', assignedAgent: 'Claude 3.7 Sonnet', modifiedFilesCount: 1 },
              created_at: new Date(Date.now() - 120 * 60 * 1000).toISOString(),
              updated_at: new Date().toISOString(),
            },
          ];
        }
        return this.mockWorkspaces as unknown as T;
      }
      case 'v1.workspaces.create': {
        const created: ExecutionWorkspace = {
          id: `ws-${Date.now().toString().slice(-4)}`,
          name: params.name || 'new-workspace',
          kind: params.kind,
          path: params.path,
          status: 'ready',
          lineage: params.lineage || {},
          metadata: params.metadata || { domain: 'engineering', assignedAgent: 'Claude Code' },
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        };
        if (!this.mockWorkspaces) this.mockWorkspaces = [];
        this.mockWorkspaces.unshift(created);
        return created as unknown as T;
      }
      case 'v1.workspaces.get': {
        const found = (this.mockWorkspaces || []).find((w) => w.id === params.workspace_id);
        return (found || this.mockWorkspaces?.[0]) as unknown as T;
      }
      case 'v1.workspaces.archive': {
        if (this.mockWorkspaces) {
          this.mockWorkspaces = this.mockWorkspaces.filter((w) => w.id !== params.workspace_id);
        }
        return {} as T;
      }
      case 'v1.tasks.list': {
        const mockTasks: Task[] = Object.values(initialProjectData)
          .flat()
          .map((s) => ({
            id: s.id,
            title: s.title,
            status: 'Active',
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          }));
        return mockTasks as unknown as T;
      }
      case 'v1.tasks.create': {
        const newTask: Task = {
          id: `task-${Date.now().toString(36)}`,
          title: params.title || 'Untitled Task',
          status: 'Active',
          contract: params.contract,
          metadata: params.metadata,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        };
        return newTask as unknown as T;
      }
      case 'v1.sessions.list': {
        const mockSessions: Session[] = Object.values(initialProjectData)
          .flat()
          .map((s) => ({
            id: s.id,
            task_id: s.id,
            mode: 'supervised',
            created_at: new Date().toISOString(),
            title: s.title,
          }));
        return mockSessions as unknown as T;
      }
      case 'v1.sessions.create': {
        return {
          id: `demo-session-${Date.now().toString(36)}`,
          task_id: params.task_id,
          mode: params.mode || 'supervised',
          created_at: new Date().toISOString(),
        } as T;
      }
      case 'v1.sessions.journal': {
        const session = Object.values(initialProjectData)
          .flat()
          .find((s) => s.id === params.session_id);
        const entries: SessionJournalEntry[] = (session?.messages || []).map((m) => ({
          id: m.id || `msg-${Math.random()}`,
          role: m.role,
          content: m.text,
          badge: m.badge,
          step_name: m.stepName,
          duration: m.duration,
          timestamp: new Date().toISOString(),
        }));
        return entries as unknown as T;
      }
      case 'v1.workflow.start_run': {
        const run: Run = {
          id: `run-${Date.now().toString(36)}`,
          task_id: params.task_id,
          status: 'Running',
          started_at: new Date().toISOString(),
          worker_runs: [
            {
              id: `wrun-${Date.now().toString(36)}`,
              node_id: 'node-exec-1',
              status: 'Running',
              started_at: new Date().toISOString(),
              harness_id: params.harness_id || 'claude_code',
            },
          ],
          metadata: {
            preferred_mode: params.preferred_mode || 'model',
            instruction: params.instruction,
          },
        };
        return run as unknown as T;
      }
      default:
        return {} as T;
    }
  }
}

export const daemonClient = new DaemonClient();
