import { invoke } from '@tauri-apps/api/core';
import {
  Task,
  Session,
  SessionJournalEntry,
  Run,
  SessionMode,
  CreateTaskParams,
  StartRunParams,
  AttachSessionParams,
  CancelRunParams,
  ExecutionWorkspace,
  CreateWorkspaceParams,
} from '../types/domain';
import {
  SourceRecord,
  PassageAnchor,
  ResearchClaim,
  ResearchExperimentRun,
  ArtifactLineageNode,
} from '../types/research';
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

  async advanceTask(taskId: string, status: Task['status']): Promise<Task> {
    return this.request<Task>('v1.tasks.advance', {
      task_id: taskId,
      target_status: status,
    });
  }

  async completeTask(taskId: string, summary?: string, evidenceClaims: any[] = []): Promise<Task> {
    return this.request<Task>('v1.tasks.complete', {
      task_id: taskId,
      summary,
      evidence_claims: evidenceClaims,
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

  async createSession(mode: SessionMode = 'assisted'): Promise<Session> {
    const wireMode = mode === 'supervised' || mode === 'interactive' ? 'assisted' : mode === 'autonomous' || mode === 'headless' ? 'bare' : mode;
    return this.request<Session>('v1.sessions.create', { mode: wireMode });
  }

  async attachSession(params: AttachSessionParams): Promise<void> {
    await this.request<void>('v1.sessions.attach', params);
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
  // Research Workbench API (Sovereign Open Science)
  // ---------------------------------------------------------

  async listResearchSources(): Promise<SourceRecord[]> {
    return this.request<SourceRecord[]>('v1.research.sources.list', {});
  }

  async saveResearchSource(source: SourceRecord): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.research.sources.save', source);
  }

  async listResearchAnchors(sourceId: string): Promise<PassageAnchor[]> {
    return this.request<PassageAnchor[]>('v1.research.anchors.list', { source_id: sourceId });
  }

  async saveResearchAnchor(anchor: PassageAnchor): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.research.anchors.save', anchor);
  }

  async listResearchClaims(): Promise<ResearchClaim[]> {
    return this.request<ResearchClaim[]>('v1.research.claims.list', {});
  }

  async saveResearchClaim(claim: ResearchClaim): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.research.claims.save', claim);
  }

  async listResearchRuns(): Promise<ResearchExperimentRun[]> {
    return this.request<ResearchExperimentRun[]>('v1.research.runs.list', {});
  }

  async saveResearchRun(run: ResearchExperimentRun): Promise<{ saved: boolean; run_id: string }> {
    return this.request<{ saved: boolean; run_id: string }>('v1.research.runs.save', run);
  }

  async listResearchArtifactLineage(artifactPath: string): Promise<ArtifactLineageNode[]> {
    return this.request<ArtifactLineageNode[]>('v1.research.lineage.list', { artifact_path: artifactPath });
  }

  async handoffClaimToCoding(claimId: string, statement: string): Promise<{ handoff_status: string; task: Task }> {
    return this.request<{ handoff_status: string; task: Task }>('v1.research.handoff_coding', {
      claim_id: claimId,
      statement,
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

      // Research Workbench Fallbacks
      case 'v1.research.sources.list': {
        const mockSources: SourceRecord[] = [
          {
            id: 'src_nature_2024_01',
            sourceType: 'paper',
            title: 'Self-Organizing Invariant Architectures in Deterministic Multi-Agent Swarms',
            doi: '10.1038/s41586-024-07821-x',
            authors: ['V. Pham', 'M. Chen', 'E. Vance'],
            year: 2024,
            contentHash: 'blake3_9941a8e2f7b11c',
            verified: true,
            abstract:
              'We present a zero-trust consensus mechanism that bounds stochastic agent divergence using Merkle-sealed invariant contracts. In empirical evaluations across 10,000 runs, phantom state execution was reduced by 99.8% while maintaining zero I/O leakages.',
          },
          {
            id: 'src_arxiv_2025_02',
            sourceType: 'paper',
            title: 'On the Convergence Rates of Cryptographic Capability Tickets under Asymmetric Latency',
            doi: '10.48550/arXiv.2501.09912',
            authors: ['T. Lindholm', 'K. S. Rao'],
            year: 2025,
            contentHash: 'blake3_7718c091ad4e22',
            verified: true,
            abstract:
              'This study provides lower bounds for atomic ticket acquisition across distributed authority gates. When latency jitter exceeds 15ms, optimistic scheduling incurs double-dispatch vulnerability unless fenced by invariant CAS certificates.',
          },
          {
            id: 'src_dataset_card_03',
            sourceType: 'dataset',
            title: 'OmniBench-ZeroIO: 50,000 Verifiable Execution Traces for Multi-Agent Safety',
            doi: '10.5281/zenodo.1089221',
            authors: ['Custos Research Lab'],
            year: 2024,
            contentHash: 'blake3_3312e778bc099f',
            verified: true,
            abstract:
              'Curated dataset of sandboxed runtime executions with complete stdout/stderr logs, container Merkle snapshots, and invariant assertions.',
          },
        ];
        return mockSources as unknown as T;
      }
      case 'v1.research.sources.save': {
        return { saved: true, id: params.id || `src_${Date.now()}` } as unknown as T;
      }
      case 'v1.research.anchors.list': {
        const mockAnchors: PassageAnchor[] = [
          {
            id: 'anc_01',
            sourceId: params.source_id || 'src_nature_2024_01',
            sourceTitle: 'Self-Organizing Invariant Architectures',
            sectionTitle: 'Section 4.2 Invariant Bounding',
            pageNumber: 8,
            startOffset: 1240,
            endOffset: 1485,
            exactText: 'Phantom state execution was reduced by 99.8% across 10,000 runs.',
            passageHash: 'blake3_anc_4491c',
          },
        ];
        return mockAnchors as unknown as T;
      }
      case 'v1.research.anchors.save': {
        return { saved: true, id: params.id || `anc_${Date.now()}` } as unknown as T;
      }
      case 'v1.research.claims.list': {
        const mockClaims: ResearchClaim[] = [
          {
            id: 'claim_01',
            statement: 'Phantom state execution in unconstrained LLM loops can be reduced by 99.8% using Merkle-sealed state invariants.',
            level: 'L3_SEALED',
            confidenceScore: 0.99,
            invariants: ['INV-PHANTOM-STATE-BOUND', 'INV-CAS-SEALED'],
            createdAt: Date.now() - 3600000,
            sealedProofUri: 'cas://bafy2bzace4v3k99a77x1198',
            evidenceLinks: [
              {
                passageAnchorId: 'anc_01',
                sourceTitle: 'Self-Organizing Invariant Architectures in Swarms',
                exactText: 'Phantom state execution was reduced by 99.8% across 10,000 runs.',
                relation: 'SUPPORTS',
                rationale: 'Empirically proven with deterministic clean-room replays across 10,000 runs.',
                verifiedBy: 'deterministic_engine',
              },
            ],
          },
        ];
        return mockClaims as unknown as T;
      }
      case 'v1.research.claims.save': {
        return { saved: true, id: params.id || `claim_${Date.now()}` } as unknown as T;
      }
      case 'v1.research.runs.list': {
        const mockRuns: ResearchExperimentRun[] = [
          {
            runId: 'run_bench_001',
            sessionId: 'sess_exp_991',
            command: 'python scripts/benchmark_fencing.py --epochs 100 --seed 42',
            cwd: '/workspaces/custos-bench',
            status: 'ok',
            wallMs: 42150,
            surface: 'modal',
            reproducibility: 'deterministic',
            inputMerkleRoot: 'merkle_in_77a91',
            outputMerkleRoot: 'merkle_out_b34c2',
            envSnapshot: {
              pythonVersion: '3.11.8',
              lockfileHash: 'sha256_lock_9901aa',
              packageCount: 142,
              hardware: '8x NVIDIA A100-SXM4-80GB (PCIe gen4)',
              platform: 'Linux 6.5.0-x86_64-aws-ec2',
            },
            sadePermitId: 'pmt_modal_exec_883',
            casLogUri: 'cas://bafy2bzace4v3k99a_run001_logs',
            ts: Date.now() - 7200000,
          },
        ];
        return mockRuns as unknown as T;
      }
      case 'v1.research.runs.save': {
        return { saved: true, run_id: params.run_id || `run_${Date.now()}` } as unknown as T;
      }
      case 'v1.research.lineage.list': {
        const mockLineage: ArtifactLineageNode[] = [
          {
            artifactPath: params.artifact_path || 'artifacts/output.csv',
            version: 1,
            contentHash: 'hash_csv_v1',
            producedByRunId: 'run_bench_001',
            timestamp: Date.now() - 3600000,
          },
        ];
        return mockLineage as unknown as T;
      }
      case 'v1.research.handoff_coding': {
        const task: Task = {
          id: `task-research-${Date.now().toString(36)}`,
          title: `Implement & Verify Research Claim: ${params.statement || params.claim_id}`,
          status: 'Draft',
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        };
        return { handoff_status: 'task_created', task } as unknown as T;
      }

      default:
        return {} as T;
    }
  }
}

export const daemonClient = new DaemonClient();
