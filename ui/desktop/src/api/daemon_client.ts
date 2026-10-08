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
  CapabilityDescriptor,
  DirtyManifest,
  SpawnTerminalParams,
  TerminalOutputChunk,
  TerminalSession,
  WorkspaceFileTree,
  WorkspaceFileContent,
  WriteWorkspaceFileParams,
  WorkspaceDiffSummary,
  WorkspaceFileDiff,
  HarnessDescriptor,
  RunNativeHarnessParams,
  HarnessExecutionResult,
  ArtifactSummary,
  ArtifactLineageGraph,
  ArtifactDetailResponse,
  NoteRecord,
  NoteVersionRecord,
  SaveNoteParams,
  NotebookCell,
  NotebookKernelState,
  ExecuteCellParams,
  ExecuteCellResult,
  ReviewerRecord,
  RecordReviewParams,
  ReviewTargetType,
  ReviewStatus,
  ResearchSynthesisProposal,
  SaveSynthesisProposalParams,
  HandoffToCodingParams,
  HandoffToCodingResult,
} from '../types/domain';
import {
  SourceRecord,
  PassageAnchor,
  ResearchClaim,
  ResearchExperimentRun,
  ArtifactLineageNode,
  ResearchRecipe,
  ResearchExecutionRecord,
  AnnotationRecord,
} from '../types/research';

const camelKey = (key: string) => key.replace(/_([a-z])/g, (_, char: string) => char.toUpperCase());
const snakeKey = (key: string) => key.replace(/[A-Z]/g, (char) => `_${char.toLowerCase()}`);

const mapKeysDeep = (value: unknown, transform: (key: string) => string): any => {
  if (Array.isArray(value)) return value.map((item) => mapKeysDeep(item, transform));
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([key, entry]) => [
        transform(key),
        mapKeysDeep(entry, transform),
      ])
    );
  }
  return value;
};

const fromDaemon = <T,>(value: unknown): T => mapKeysDeep(value, camelKey) as T;
const toDaemon = (value: unknown): Record<string, any> => mapKeysDeep(value, snakeKey);

const normalizeWorkspace = (wire: any): ExecutionWorkspace => {
  const failure = wire.status && typeof wire.status === 'object' ? wire.status.setup_failed : null;
  const dirtyManifest = wire.dirty_manifest
    ? (fromDaemon(wire.dirty_manifest) as DirtyManifest)
    : wire.dirtyManifest ?? null;
  const ownerTaskId = wire.owner_task_id ?? wire.ownerTaskId ?? null;
  const baseCommitHash = wire.base_commit_hash ?? wire.baseCommitHash ?? null;

  return {
    ...wire,
    status: failure ? 'setup_failed' : wire.status,
    status_reason: failure?.reason,
    owner_task_id: ownerTaskId,
    ownerTaskId,
    base_commit_hash: baseCommitHash,
    baseCommitHash,
    dirty_manifest: dirtyManifest,
    dirtyManifest,
  } as ExecutionWorkspace;
};

export class DaemonClient {
  private isTauri: boolean;
  private baseUrl: string = 'http://127.0.0.1:3000';
  private daemonOnline: boolean = false;

  constructor() {
    this.isTauri =
      typeof window !== 'undefined' &&
      (Boolean((window as any).__TAURI_INTERNALS__) ||
        Boolean((window as any).__TAURI__));
  }

  async checkHealth(): Promise<boolean> {
    try {
      const res = await fetch(`${this.baseUrl}/api/health`, { method: 'GET' });
      this.daemonOnline = res.ok;
      return this.daemonOnline;
    } catch {
      this.daemonOnline = false;
      return false;
    }
  }

  get isConnected(): boolean {
    return this.isTauri || this.daemonOnline;
  }

  /**
   * Core request dispatcher. Invokes in-process Tauri command or sends HTTP RPC to custos-daemon.
   */
  async request<T>(method: string, params: Record<string, any> = {}): Promise<T> {
    if (this.isTauri) {
      try {
        return await invoke<T>('custos_request', { method, params });
      } catch (err: any) {
        console.warn(`[DaemonClient] Tauri invoke failed on ${method}, falling back to HTTP:`, err);
        return this.requestHttp<T>(method, params);
      }
    } else {
      return this.requestHttp<T>(method, params);
    }
  }

  private async requestHttp<T>(method: string, params: Record<string, any> = {}): Promise<T> {
    const res = await fetch(`${this.baseUrl}/api/request`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: `web_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
        method,
        params,
      }),
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    }
    const data = await res.json();
    this.daemonOnline = true;
    if (data.error) {
      throw new Error(data.error);
    }
    return data.result as T;
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
    return normalizeWorkspace(await this.request<unknown>('v1.workspaces.create', params));
  }

  async getWorkspace(workspaceId: string): Promise<ExecutionWorkspace> {
    return normalizeWorkspace(await this.request<unknown>('v1.workspaces.get', { workspace_id: workspaceId }));
  }

  async listWorkspaces(): Promise<ExecutionWorkspace[]> {
    const records = await this.request<unknown[]>('v1.workspaces.list', {});
    return records.map(normalizeWorkspace);
  }

  async archiveWorkspace(workspaceId: string, deletePhysical = false, force = false): Promise<void> {
    return this.request<void>('v1.workspaces.archive', {
      workspace_id: workspaceId,
      delete_physical: deletePhysical,
      force,
    });
  }

  async inspectWorkspaceDirty(workspaceId: string): Promise<DirtyManifest> {
    return fromDaemon<DirtyManifest>(
      await this.request<unknown>('v1.workspaces.inspect_dirty', { workspace_id: workspaceId })
    );
  }

  async recoverWorkspace(workspaceId?: string): Promise<ExecutionWorkspace | ExecutionWorkspace[]> {
    const raw = await this.request<unknown>('v1.workspaces.recover', { workspace_id: workspaceId });
    if (Array.isArray(raw)) {
      return raw.map(normalizeWorkspace);
    }
    return normalizeWorkspace(raw);
  }

  // ---------------------------------------------------------
  // Research Workbench API (Sovereign Open Science)
  // ---------------------------------------------------------

  async listResearchSources(): Promise<SourceRecord[]> {
    return fromDaemon<SourceRecord[]>(await this.request<unknown>('v1.research.sources.list', {}));
  }

  async saveResearchSource(source: SourceRecord): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.research.sources.save', toDaemon(source));
  }

  async listResearchAnchors(sourceId: string): Promise<PassageAnchor[]> {
    return fromDaemon<PassageAnchor[]>(await this.request<unknown>('v1.research.anchors.list', { source_id: sourceId }));
  }

  async saveResearchAnchor(anchor: PassageAnchor): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.research.anchors.save', toDaemon(anchor));
  }

  async listResearchClaims(): Promise<ResearchClaim[]> {
    return fromDaemon<ResearchClaim[]>(await this.request<unknown>('v1.research.claims.list', {}));
  }

  async saveResearchClaim(claim: ResearchClaim): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.research.claims.save', toDaemon(claim));
  }

  async listResearchRuns(): Promise<ResearchExperimentRun[]> {
    return fromDaemon<ResearchExperimentRun[]>(await this.request<unknown>('v1.research.runs.list', {}));
  }

  async saveResearchRun(run: ResearchExperimentRun): Promise<{ saved: boolean; run_id: string }> {
    return this.request<{ saved: boolean; run_id: string }>('v1.research.runs.save', toDaemon(run));
  }

  async listResearchArtifactLineage(artifactPath: string): Promise<ArtifactLineageNode[]> {
    return fromDaemon<ArtifactLineageNode[]>(await this.request<unknown>('v1.research.lineage.list', { artifact_path: artifactPath }));
  }

  async handoffClaimToCoding(claimId: string, statement: string): Promise<{ handoff_status: string; task: Task }> {
    return this.request<{ handoff_status: string; task: Task }>('v1.research.handoff_coding', {
      claim_id: claimId,
      statement,
    });
  }

  // ---------------------------------------------------------
  // Recipes & Annotations API
  // ---------------------------------------------------------

  async listResearchRecipes(): Promise<ResearchRecipe[]> {
    return fromDaemon<ResearchRecipe[]>(await this.request<unknown>('v1.research.recipes.list', {}));
  }

  async saveResearchRecipe(recipe: ResearchRecipe): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.research.recipes.save', toDaemon(recipe));
  }

  async getResearchRecipe(recipeId: string): Promise<ResearchRecipe> {
    return fromDaemon<ResearchRecipe>(await this.request<unknown>('v1.research.recipes.get', { recipe_id: recipeId }));
  }

  async listResearchExecutions(recipeId: string): Promise<ResearchExecutionRecord[]> {
    return fromDaemon<ResearchExecutionRecord[]>(await this.request<unknown>('v1.research.executions.list', { recipe_id: recipeId }));
  }

  async getResearchExecution(executionId: string): Promise<ResearchExecutionRecord> {
    return fromDaemon<ResearchExecutionRecord>(await this.request<unknown>('v1.research.executions.get', { execution_id: executionId }));
  }

  async listResearchAnnotations(artifactId: string, version?: number): Promise<AnnotationRecord[]> {
    return fromDaemon<AnnotationRecord[]>(await this.request<unknown>('v1.research.annotations.list', { artifact_id: artifactId, version }));
  }

  async saveResearchAnnotation(annotation: AnnotationRecord): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.research.annotations.save', toDaemon(annotation));
  }

  // ---------------------------------------------------------
  // Providers, Keys & LLM Status API
  // ---------------------------------------------------------

  async listProviders(): Promise<any[]> {
    return this.request<any[]>('v1.providers.list', {});
  }

  async saveProvider(provider: any): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.providers.save', provider);
  }

  async listClientKeys(): Promise<any[]> {
    return this.request<any[]>('v1.keys.list', {});
  }

  async generateClientKey(name?: string): Promise<any> {
    return this.request<any>('v1.keys.generate', { name });
  }

  async revokeClientKey(keyId: string): Promise<{ revoked: boolean }> {
    return this.request<{ revoked: boolean }>('v1.keys.revoke', { key_id: keyId });
  }

  async checkLlmStatus(): Promise<{ configured: boolean; active_provider: string | null; message: string }> {
    return this.request<{ configured: boolean; active_provider: string | null; message: string }>('v1.llm.status', {});
  }

  // ---------------------------------------------------------
  // Capability Registry API
  // ---------------------------------------------------------

  async listCapabilities(): Promise<CapabilityDescriptor[]> {
    return fromDaemon<CapabilityDescriptor[]>(await this.request<unknown>('v1.capabilities.list', {}));
  }

  async getCapability(targetId: string): Promise<CapabilityDescriptor> {
    return fromDaemon<CapabilityDescriptor>(
      await this.request<unknown>('v1.capabilities.get', { capability_id: targetId })
    );
  }

  // ---------------------------------------------------------
  // Terminal / Bounded PTY API
  // ---------------------------------------------------------

  async spawnTerminal(params: SpawnTerminalParams): Promise<TerminalSession> {
    return fromDaemon<TerminalSession>(
      await this.request<unknown>('v1.terminal.spawn', toDaemon(params))
    );
  }

  async writeTerminal(sessionId: string, data: string): Promise<{ written: number }> {
    return this.request<{ written: number }>('v1.terminal.write', {
      session_id: sessionId,
      data,
    });
  }

  async resizeTerminal(sessionId: string, cols: number, rows: number): Promise<{ ok: boolean }> {
    return this.request<{ ok: boolean }>('v1.terminal.resize', {
      session_id: sessionId,
      cols,
      rows,
    });
  }

  async readTerminal(sessionId: string, fromSeq = 0, maxBytes = 65536): Promise<TerminalOutputChunk> {
    return fromDaemon<TerminalOutputChunk>(
      await this.request<unknown>('v1.terminal.read', {
        session_id: sessionId,
        from_seq: fromSeq,
        max_bytes: maxBytes,
      })
    );
  }

  async terminateTerminal(sessionId: string): Promise<{ terminated: boolean }> {
    return this.request<{ terminated: boolean }>('v1.terminal.terminate', {
      session_id: sessionId,
    });
  }

  async listTerminals(workspaceId?: string): Promise<TerminalSession[]> {
    return fromDaemon<TerminalSession[]>(
      await this.request<unknown>('v1.terminal.list', { workspace_id: workspaceId })
    );
  }

  async getTerminal(sessionId: string): Promise<TerminalSession> {
    return fromDaemon<TerminalSession>(
      await this.request<unknown>('v1.terminal.get', { session_id: sessionId })
    );
  }

  // ---------------------------------------------------------
  // Workspace Files & Git Diff API
  // ---------------------------------------------------------

  async getWorkspaceFileTree(
    workspaceId: string,
    relativeDir?: string,
    maxDepth?: number
  ): Promise<WorkspaceFileTree> {
    return fromDaemon<WorkspaceFileTree>(
      await this.request<unknown>('v1.workspace.files.tree', {
        workspace_id: workspaceId,
        relative_dir: relativeDir,
        max_depth: maxDepth,
      })
    );
  }

  async readWorkspaceFile(
    workspaceId: string,
    path: string,
    maxBytes?: number
  ): Promise<WorkspaceFileContent> {
    return fromDaemon<WorkspaceFileContent>(
      await this.request<unknown>('v1.workspace.files.read', {
        workspace_id: workspaceId,
        path,
        max_bytes: maxBytes,
      })
    );
  }

  async writeWorkspaceFile(params: WriteWorkspaceFileParams): Promise<WorkspaceFileContent> {
    return fromDaemon<WorkspaceFileContent>(
      await this.request<unknown>('v1.workspace.files.write', toDaemon(params))
    );
  }

  async getWorkspaceDiff(workspaceId: string, staged?: boolean): Promise<WorkspaceDiffSummary> {
    return fromDaemon<WorkspaceDiffSummary>(
      await this.request<unknown>('v1.workspace.diff', {
        workspace_id: workspaceId,
        staged,
      })
    );
  }

  async getWorkspaceFileDiff(
    workspaceId: string,
    path: string,
    staged?: boolean
  ): Promise<WorkspaceFileDiff> {
    return fromDaemon<WorkspaceFileDiff>(
      await this.request<unknown>('v1.workspace.diff.file', {
        workspace_id: workspaceId,
        path,
        staged,
      })
    );
  }

  async stageWorkspaceFile(
    workspaceId: string,
    path: string
  ): Promise<{ staged: boolean; path: string }> {
    return this.request<{ staged: boolean; path: string }>('v1.workspace.git.stage', {
      workspace_id: workspaceId,
      path,
    });
  }

  async unstageWorkspaceFile(
    workspaceId: string,
    path: string
  ): Promise<{ unstaged: boolean; path: string }> {
    return this.request<{ unstaged: boolean; path: string }>('v1.workspace.git.unstage', {
      workspace_id: workspaceId,
      path,
    });
  }

  async discardWorkspaceFile(
    workspaceId: string,
    path: string
  ): Promise<{ discarded: boolean; path: string }> {
    return this.request<{ discarded: boolean; path: string }>('v1.workspace.git.discard', {
      workspace_id: workspaceId,
      path,
    });
  }

  async listHarnesses(): Promise<HarnessDescriptor[]> {
    return this.request<HarnessDescriptor[]>('v1.harness.list', {});
  }

  async getHarness(harnessId: string): Promise<HarnessDescriptor> {
    return this.request<HarnessDescriptor>('v1.harness.get', { harness_id: harnessId });
  }

  async runNativeHarness(params: RunNativeHarnessParams): Promise<HarnessExecutionResult> {
    return this.request<HarnessExecutionResult>('v1.harness.run_native', {
      harness_id: params.harness_id ?? params.harnessId,
      instruction: params.instruction,
      workspace_id: params.workspace_id ?? params.workspaceId,
      cwd: params.cwd,
    });
  }

  async cancelHarness(harnessId: string, runId: string): Promise<{ cancelled: boolean }> {
    return this.request<{ cancelled: boolean }>('v1.harness.cancel', {
      harness_id: harnessId,
      run_id: runId,
    });
  }

  async steerHarness(harnessId: string, runId: string, guidance: string): Promise<{ steered: boolean }> {
    return this.request<{ steered: boolean }>('v1.harness.steer', {
      harness_id: harnessId,
      run_id: runId,
      guidance,
    });
  }

  // Artifacts & Lineage DAG
  async listArtifacts(): Promise<ArtifactSummary[]> {
    return this.request<ArtifactSummary[]>('v1.artifacts.list');
  }

  async getArtifact(path: string): Promise<ArtifactDetailResponse> {
    return this.request<ArtifactDetailResponse>('v1.artifacts.get', {
      artifact_path: path,
    });
  }

  async getArtifactLineageGraph(path?: string): Promise<ArtifactLineageGraph> {
    return this.request<ArtifactLineageGraph>('v1.artifacts.lineage_graph', {
      artifact_path: path,
    });
  }

  async recordArtifactLineage(node: ArtifactLineageNode): Promise<{ recorded: boolean }> {
    return this.request<{ recorded: boolean }>('v1.artifacts.record_lineage', node);
  }

  // Personal Notes & Scratchpads
  async listNotes(sessionId?: string): Promise<NoteRecord[]> {
    return this.request<NoteRecord[]>('v1.notes.list', {
      session_id: sessionId,
    });
  }

  async getNote(id: string): Promise<NoteRecord> {
    return this.request<NoteRecord>('v1.notes.get', { id });
  }

  async saveNote(params: SaveNoteParams): Promise<NoteRecord> {
    return this.request<NoteRecord>('v1.notes.save', params);
  }

  async listNoteVersions(noteId: string): Promise<NoteVersionRecord[]> {
    return this.request<NoteVersionRecord[]>('v1.notes.history', {
      note_id: noteId,
    });
  }

  // Notebook Cells & Python Kernel
  async listNotebookCells(sessionId: string): Promise<NotebookCell[]> {
    return this.request<NotebookCell[]>('v1.notebook.cells.list', {
      session_id: sessionId,
    });
  }

  async saveNotebookCells(sessionId: string, cells: NotebookCell[]): Promise<{ saved: boolean; count: number }> {
    return this.request<{ saved: boolean; count: number }>('v1.notebook.cells.save', {
      session_id: sessionId,
      cells,
    });
  }

  async executeNotebookCell(params: ExecuteCellParams): Promise<ExecuteCellResult> {
    return this.request<ExecuteCellResult>('v1.notebook.execute', params);
  }

  async interruptNotebookKernel(sessionId: string): Promise<{ interrupted: boolean }> {
    return this.request<{ interrupted: boolean }>('v1.notebook.interrupt', {
      session_id: sessionId,
    });
  }

  async resetNotebookKernel(sessionId: string): Promise<NotebookKernelState> {
    return this.request<NotebookKernelState>('v1.notebook.reset', {
      session_id: sessionId,
    });
  }

  async getNotebookKernelStatus(sessionId: string): Promise<NotebookKernelState> {
    return this.request<NotebookKernelState>('v1.notebook.status', {
      session_id: sessionId,
    });
  }

  // Evidence & Criteria Reviewer Records
  async listReviews(params?: {
    target_type?: ReviewTargetType;
    target_id?: string;
    reviewer?: string;
    status?: ReviewStatus;
    fresh_only?: boolean;
  }): Promise<ReviewerRecord[]> {
    return this.request<ReviewerRecord[]>('v1.reviews.list', params ?? {});
  }

  async getReview(id: string): Promise<ReviewerRecord | null> {
    return this.request<ReviewerRecord | null>('v1.reviews.get', { id });
  }

  async recordReview(params: RecordReviewParams): Promise<ReviewerRecord> {
    return this.request<ReviewerRecord>('v1.reviews.record', params);
  }

  async markReviewStale(id: string): Promise<{ marked_stale: boolean }> {
    return this.request<{ marked_stale: boolean }>('v1.reviews.mark_stale', { id });
  }

  // Research Synthesis Proposals & Coding Handoff
  async listSynthesisProposals(): Promise<ResearchSynthesisProposal[]> {
    return this.request<ResearchSynthesisProposal[]>('v1.synthesis.proposals.list', {});
  }

  async getSynthesisProposal(id: string): Promise<ResearchSynthesisProposal | null> {
    return this.request<ResearchSynthesisProposal | null>('v1.synthesis.proposals.get', { id });
  }

  async saveSynthesisProposal(params: SaveSynthesisProposalParams): Promise<ResearchSynthesisProposal> {
    return this.request<ResearchSynthesisProposal>('v1.synthesis.proposals.save', params);
  }

  async executeSynthesisHandoff(params: HandoffToCodingParams): Promise<HandoffToCodingResult> {
    return this.request<HandoffToCodingResult>('v1.synthesis.handoff.execute', params);
  }
}

export const daemonClient = new DaemonClient();
