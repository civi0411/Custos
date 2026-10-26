import { invoke } from '@tauri-apps/api/core';
import {
  Task,
  Session,
  SessionJournalEntry,
  RunHandle,
  SessionMode,
  CreateTaskParams,
  StartRunParams,
  AttachSessionParams,
  CancelRunParams,
  ExecutionWorkspace,
  CreateWorkspaceParams,
  WorkspaceDetectionResult,
  GitHubAuthStatusResponse,
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
  BrowserSession,
  BrowserTab,
  BrowserPageSnapshot,
  RemoteHostNode,
  RegisterHostParams,
  FleetExecReceipt,
  HeadlessAutomationJob,
  CreateHeadlessJobParams,
  ProbedModel,
  ModelPricing,
  ModelCatalogResult,
  ProviderConfigRecord,
  LocalModelFolder,
  LocalModelDescriptor,
  LocalHardwareProfile,
  LocalModelPreflightResponse,
  LoadedModelInstance,
  LocalModelLoadProfile,
  OAuthAuthorizeResult,
  OAuthConnectionMetadata,
  OAuthStatusResult,
  DomainPackManifest,
  PacksJobCatalogItem,
  WorkerRole,
  RunPacksJobResult,
  EmulatorDevice,
  EmulatorSession,
  EmulatorSnapshotResult,
  EmulatorScreenshotResult,
  AssistantDraft,
  OutboxActionItem,
  ResourceActionCapability,
} from '../types/domain';
import {
  SourceRecord,
  PassageAnchor,
  ResearchClaim,
  ResearchExperimentRun,
  ArtifactLineageNode,
  ResearchRecipe,
  ResearchExecutionRecord,
  CheckpointResumeAssessment,
  AnnotationRecord,
  ComputeTargetInfo,
  PreflightReport,
  ExecutionAttemptRecord,
} from '../types/research';

import {
  camelKey,
  snakeKey,
  mapKeysDeep,
  fromDaemon,
  toDaemon,
  normalizeTerminalSession,
  normalizeHarness,
  annotationFromWire,
  annotationToWire,
  normalizeWorkspaceFile,
  normalizeWorkspace,
} from './normalizers';
import {
  isReadMethod,
  isRetrySafeMethod,
  isTransportError,
  generateCommandId,
  prepareRequestParams,
  evaluateRetry,
} from './transport';

export {
  camelKey,
  snakeKey,
  mapKeysDeep,
  fromDaemon,
  toDaemon,
  normalizeTerminalSession,
  normalizeHarness,
  annotationFromWire,
  annotationToWire,
  normalizeWorkspaceFile,
  normalizeWorkspace,
  isReadMethod,
  isRetrySafeMethod,
  isTransportError,
  generateCommandId,
  prepareRequestParams,
  evaluateRetry,
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

  private async resolveHttpEndpoint(): Promise<string> {
    if (this.isTauri) {
      try {
        const endpoint = await invoke<string>('custos_http_endpoint');
        const parsed = new URL(endpoint);
        if (parsed.protocol !== 'http:' || parsed.hostname !== '127.0.0.1') throw new Error('Invalid daemon HTTP endpoint');
        this.baseUrl = endpoint;
      } catch {
        if (!this.baseUrl) {
          this.baseUrl = 'http://127.0.0.1:3000';
        }
      }
    }
    return this.baseUrl || 'http://127.0.0.1:3000';
  }

  async watchRun(session: string, run: string, update: (snapshot: import('../types/domain').LiveRunSnapshot) => void, signal: AbortSignal): Promise<void> {
    type Snapshot = import('../types/domain').LiveRunSnapshot;
    const terminal = (status: string) => ['completed', 'failed', 'cancelled', 'interrupted'].includes(status);
    let snapshot: Snapshot | null = null;
    let backoff = 1000;
    while (!signal.aborted) {
      try {
        const base = await this.resolveHttpEndpoint();
        const replay: { snapshot: Snapshot } = await this.request<{ snapshot: Snapshot }>('v1.workflow.live_run', { session_id: session, run_id: run, cursor: snapshot?.cursor || 0 });
        snapshot = { ...replay.snapshot, reconnecting: false };
        update(snapshot);
        if (terminal(snapshot.status)) return;
        const response = await fetch(`${base}/api/events?session_id=${encodeURIComponent(session)}&run_id=${encodeURIComponent(run)}&cursor=${snapshot.cursor}`, { signal, headers: { Accept: 'text/event-stream' } });
        if (!response.ok || !response.body) throw new Error(`Live connection HTTP ${response.status}`);
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let pending = '';
        const consume = (frame: string) => {
          let event = ''; const data: string[] = [];
          for (const line of frame.split('\n')) {
            if (line.startsWith('event:')) event = line.slice(6).trim();
            if (line.startsWith('data:')) data.push(line.slice(5).trimStart());
          }
          if (!data.length || !snapshot) return;
          const value = JSON.parse(data.join('\n'));
          if (event === 'snapshot') {
            if (value.run_id !== run || value.session_id !== session) throw new Error('Mismatched live snapshot');
            snapshot = { ...value, reconnecting: false };
          } else if (event === 'run') {
            if (value.run_id !== run || value.session_id !== session || value.id <= snapshot.cursor) return;
            if (value.id !== snapshot.cursor + 1) throw new Error('Live cursor gap');
            const payload = value.payload || {};
            snapshot = { ...snapshot, cursor: value.id, reconnecting: false };
            if (value.kind === 'text_delta') snapshot.text += payload.text || '';
            if (value.kind === 'status') snapshot.status = payload.status;
            if (['tool_input', 'tool_output', 'reasoning_summary'].includes(value.kind)) snapshot.activities = [...snapshot.activities, { kind: value.kind, payload }].slice(-100);
            if (value.kind === 'usage') snapshot.usage = payload;
            if (value.kind === 'result') { snapshot.status = payload.status; snapshot.text = payload.text || snapshot.text; snapshot.error = payload.error; }
          }
          if (snapshot) update(snapshot);
        };
        try {
          while (!signal.aborted) {
            const { value, done } = await reader.read();
            if (done) break;
            pending += decoder.decode(value, { stream: true }).replace(/\r\n/g, '\n');
            if (pending.length > 3 * 1024 * 1024) throw new Error('Live frame exceeded limit');
            let end: number;
            while ((end = pending.indexOf('\n\n')) >= 0) { consume(pending.slice(0, end)); pending = pending.slice(end + 2); }
            if (snapshot && terminal(snapshot.status)) return;
            backoff = 1000;
          }
        } finally { await reader.cancel().catch(() => undefined); reader.releaseLock(); }
        if (!signal.aborted) throw new Error('Live stream disconnected');
      } catch (error) {
        if (signal.aborted) return;
        if (snapshot) update({ ...snapshot, reconnecting: true });
        await new Promise<void>(resolve => {
          const timer = window.setTimeout(() => { signal.removeEventListener('abort', abort); resolve(); }, backoff);
          const abort = () => { window.clearTimeout(timer); resolve(); };
          signal.addEventListener('abort', abort, { once: true });
        });
        backoff = Math.min(5000, backoff * 2);
      }
    }
  }

  async getSessionMetrics(sessionId?: string): Promise<{
    session_id?: string;
    tokens_in: number;
    tokens_out: number;
    total_tokens: number;
    estimated_cost_usd: number;
    active_runs: number;
    pending_approvals: number;
  }> {
    return this.request('v1.sessions.metrics', { session_id: sessionId });
  }

  async getProviderUsage(includeQuota = false): Promise<import('../types/provider_types').ProviderUsageReport> {
    return this.request('v1.providers.usage', { include_quota: includeQuota });
  }

  async getSystemMetrics(): Promise<{
    memory_resident_mb: number;
    system_total_memory_mb?: number | null;
    process_groups?: Array<{
      id: string;
      name: string;
      kind: string;
      pid: number;
      memory_mb: number;
      cpu_percent: number;
    }>;
    active_runs: number;
    active_ports: number[];
    listening_ports?: Array<{
      port: number;
      pid: number;
      command: string;
      public: boolean;
    }>;
    caffeinate: {
      mode: 'off' | 'agent' | 'always';
      active: boolean;
    };
  }> {
    return this.request('v1.system.metrics');
  }

  async setCaffeinate(mode: 'off' | 'agent' | 'always'): Promise<{
    mode: 'off' | 'agent' | 'always';
    active: boolean;
  }> {
    return this.request('v1.system.caffeinate', { mode });
  }

  async checkHealth(): Promise<boolean> {
    try {
      const res = await fetch(`${await this.resolveHttpEndpoint()}/api/health`, { method: 'GET' });
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
   * Ensures mutations carry a stable logical commandId across Tauri IPC and HTTP retry/fallback paths.
   */
  async request<T>(method: string, params: Record<string, any> = {}): Promise<T> {
    const finalParams = prepareRequestParams(method, params);

    if (this.isTauri) {
      try {
        return await invoke<T>('custos_request', { method, params: finalParams });
      } catch (err: any) {
        const decision = evaluateRetry(method, finalParams, err);
        if (!decision.shouldRetry) {
          console.warn(`[DaemonClient] Rejection on ${method} (${decision.reason}), failing without HTTP retry:`, err);
          throw err;
        }

        console.warn(`[DaemonClient] Tauri invoke failed on ${method} (${decision.reason}), retrying over HTTP:`, err);
        return this.requestHttp<T>(method, finalParams);
      }
    } else {
      return this.requestHttp<T>(method, finalParams);
    }
  }

  private async requestHttp<T>(method: string, params: Record<string, any> = {}): Promise<T> {
    const baseUrl = await this.resolveHttpEndpoint();
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 15000);
    const reqId = params.command_id || `web_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
    try {
      const res = await fetch(`${baseUrl}/api/request`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        signal: controller.signal,
        body: JSON.stringify({
          id: reqId,
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
    } finally {
      clearTimeout(timeout);
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

  async listTasks(sessionId?: string): Promise<Task[]> {
    return this.request<Task[]>('v1.tasks.list', { session_id: sessionId });
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

  async listSessionRuns(sessionId: string): Promise<Array<{ run_id: string; model?: string; status: string; journal_occurred_at?: string }>> {
    return this.request('v1.workflow.session_runs', { session_id: sessionId });
  }

  async startRun(params: StartRunParams): Promise<RunHandle> {
    return this.request<RunHandle>('v1.workflow.start_run', params);
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

  async deleteSession(sessionId: string): Promise<{ deleted: boolean; session_id: string }> {
    return this.request<{ deleted: boolean; session_id: string }>('v1.sessions.delete', {
      session_id: sessionId,
    });
  }

  async getSessionJournal(sessionId: string): Promise<SessionJournalEntry[]> {
    return this.request<SessionJournalEntry[]>('v1.sessions.journal', {
      session_id: sessionId,
    });
  }

  async openSessionInLens(sessionId: string, lens: 'assistant' | 'coding' | 'research'): Promise<void> {
    await this.request<{ status: string }>('v1.sessions.open_in_lens', {
      session_id: sessionId,
      lens,
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

  async detectWorkspace(path: string): Promise<WorkspaceDetectionResult> {
    return this.request<WorkspaceDetectionResult>('v1.workspaces.detect', { path });
  }

  async pickFolder(purpose?: 'model'): Promise<string | null> {
    const res = await this.request<{ path: string | null }>('v1.workspaces.pick_folder', purpose ? { purpose } : {});
    return res?.path ?? null;
  }

  async spawnCandidates(params: {
    task_id: string;
    base_workspace_id?: string;
    repo_path?: string;
    count?: number;
    base_commit_hash?: string;
    harness_ids?: string[];
    setup_script?: string;
    command_id?: string;
  }): Promise<{ task_id: string; candidates: ExecutionWorkspace[]; partial: boolean; failed: { candidate_index: number; error: string }[] }> {
    const raw = await this.request<{ task_id: string; candidates: unknown[]; partial?: boolean; failed?: { candidate_index: number; error: string }[] }>('v1.workspaces.candidates.spawn', params);
    return {
      task_id: raw.task_id,
      partial: raw.partial === true,
      failed: raw.failed || [],
      candidates: (raw.candidates || []).map(normalizeWorkspace),
    };
  }

  async compareCandidates(params: {
    task_id?: string;
    workspace_ids?: string[];
  }): Promise<{
    count: number;
    candidates: Array<{
      workspace_id: string;
      name: string;
      path: string;
      branch: string;
      base_commit_hash?: string;
      is_dirty: boolean;
      modified_count: number;
      untracked_count: number;
      deleted_count: number;
      modified_files: string[];
      untracked_files: string[];
      deleted_files: string[];
      diff: string;
      status: string;
    }>;
  }> {
    return this.request('v1.workspaces.candidates.compare', params);
  }

  async integrateCandidate(params: {
    winner_workspace_id: string;
    target_branch?: string;
    loser_workspace_ids?: string[];
    force_loser_cleanup?: boolean;
    preview?: boolean;
    expected_target_head?: string;
    expected_winner_head?: string;
  }): Promise<{
    winner_workspace_id: string;
    winner_branch: string;
    target_branch: string;
    is_stale_base: boolean;
    current_target_head?: string;
    winner_base_commit?: string;
    winner_head?: string;
    merge_status?: string;
    preview?: boolean;
    merge_success: boolean;
    merge_detail: string;
    loser_cleanup: Array<{ workspace_id: string; success: boolean; error?: string }>;
  }> {
    return this.request('v1.workspaces.integrate', params);
  }

  // ---------------------------------------------------------
  // Settings & Quotas API
  // ---------------------------------------------------------

  async getSettings(): Promise<Record<string, string>> {
    return this.request<Record<string, string>>('v1.settings.get', {});
  }

  async setSettings(settings: Record<string, string | number | boolean>): Promise<Record<string, string>> {
    const payload: Record<string, string> = {};
    for (const [k, v] of Object.entries(settings)) {
      payload[k] = String(v);
    }
    return this.request<Record<string, string>>('v1.settings.set', payload);
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

  async assessResearchResume(executionId: string): Promise<CheckpointResumeAssessment> {
    return fromDaemon<CheckpointResumeAssessment>(await this.request<unknown>(
      'v1.research.executions.resume.assess', { execution_id: executionId },
    ));
  }

  async listResearchAnnotations(artifactId: string, version?: number): Promise<AnnotationRecord[]> {
    const records = await this.request<unknown[]>('v1.research.annotations.list', { artifact_id: artifactId, version });
    return records.map(annotationFromWire);
  }

  async saveResearchAnnotation(annotation: AnnotationRecord): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.research.annotations.save', annotationToWire(annotation));
  }

  // ---------------------------------------------------------
  // Providers, Keys & LLM Status API
  // ---------------------------------------------------------

  async getProviderRouting(service: string): Promise<{ service_type: string; round_robin: boolean; account_order: string[] }> {
    return this.request('v1.providers.routing.get', { service_type: service });
  }

  async saveProviderRouting(service: string, roundRobin: boolean, accountOrder: string[]): Promise<{ service_type: string; round_robin: boolean; account_order: string[] }> {
    return this.request('v1.providers.routing.save', { service_type: service, round_robin: roundRobin, account_order: accountOrder });
  }

  async reserveProviderAccount(service: string, model: string, reasoningEffort?: string): Promise<{ account_id: string | null }> {
    return this.request('v1.providers.route', { service_type: service, model, reasoning_effort: reasoningEffort });
  }

  async listProviders(): Promise<any[]> {
    return this.request<any[]>('v1.providers.list', {});
  }

  async getProvider(providerId: string): Promise<ProviderConfigRecord> {
    return this.request<ProviderConfigRecord>('v1.providers.get', { id: providerId });
  }

  async saveProvider(provider: any): Promise<{ saved: boolean; id: string }> {
    return this.request<{ saved: boolean; id: string }>('v1.providers.save', provider);
  }

  async deleteProvider(providerId: string): Promise<{ deleted: boolean; id: string }> {
    return this.request<{ deleted: boolean; id: string }>('v1.providers.delete', { id: providerId });
  }

  // ---------------------------------------------------------
  // OAuth 2.0 PKCE Flow API
  // ---------------------------------------------------------

  async getOAuthAuthorizeUrl(params?: {
    provider_id?: string;
    service_type?: string;
    client_id?: string;
    redirect_uri?: string;
    scope?: string;
    auth_endpoint?: string;
    token_endpoint?: string;
  }): Promise<OAuthAuthorizeResult> {
    return this.request<OAuthAuthorizeResult>('v1.oauth.authorize', params || {});
  }

  async exchangeOAuthCode(params: {
    code_or_url: string;
    code_verifier: string;
    provider_id?: string;
    service_type?: string;
    client_id?: string;
    redirect_uri?: string;
    token_endpoint?: string;
  }): Promise<OAuthConnectionMetadata> {
    return this.request<OAuthConnectionMetadata>('v1.oauth.exchange', params);
  }

  async refreshOAuthToken(provider_id: string = 'openai'): Promise<OAuthConnectionMetadata> {
    return this.request<OAuthConnectionMetadata>('v1.oauth.refresh', { provider_id });
  }

  async getOAuthToken(provider_id: string = 'openai'): Promise<OAuthConnectionMetadata | null> {
    return this.request<OAuthConnectionMetadata | null>('v1.oauth.get', { provider_id });
  }

  async deleteOAuthToken(provider_id: string = 'openai'): Promise<{ deleted: boolean; provider_id: string }> {
    return this.request<{ deleted: boolean; provider_id: string }>('v1.oauth.delete', { provider_id });
  }

  async cancelOAuth(): Promise<{ cancelled: boolean }> {
    return this.request('v1.oauth.cancel', {});
  }

  async getOAuthStatus(): Promise<OAuthStatusResult> {
    return this.request<OAuthStatusResult>('v1.oauth.status', {});
  }

  async startGitHubLogin(): Promise<TerminalSession> {
    return fromDaemon<TerminalSession>(await this.request<unknown>('v1.github.login', {}));
  }

  async getGitHubAuthStatus(): Promise<GitHubAuthStatusResponse> {
    return this.request<GitHubAuthStatusResponse>('v1.github.auth_status', {});
  }

  async getGitHubViewer(): Promise<any> {
    return this.request('v1.github.viewer', {});
  }


  async probeModels(
    baseUrl: string,
    providerType?: string,
    apiKey?: string,
    providerId?: string
  ): Promise<{ models: ProbedModel[]; count: number; base_url: string }> {
    const probePromise = this.request<{ models: ProbedModel[]; count: number; base_url: string }>('v1.models.probe', {
      base_url: baseUrl,
      provider_type: providerType,
      api_key: apiKey,
      provider_id: providerId,
    });
    let timer: ReturnType<typeof setTimeout> | undefined;
    const timeoutPromise = new Promise<never>((_, reject) => {
      timer = setTimeout(() => reject(new Error('Model probe timed out')), 30000);
    });
    try {
      return await Promise.race([probePromise, timeoutPromise]);
    } finally {
      clearTimeout(timer);
    }
  }

  async getModelCatalog(providerType?: string, providerId?: string): Promise<ModelCatalogResult> {
    return this.request<ModelCatalogResult>('v1.models.catalog', {
      provider_type: providerType,
      provider_id: providerId,
    });
  }

  async getModelPricing(
    modelId: string,
    tokens?: {
      input_tokens?: number;
      output_tokens?: number;
      cache_read_tokens?: number;
      cache_write_tokens?: number;
    }
  ): Promise<{
    model_id: string;
    pricing: ModelPricing | null;
    estimated_cost_usd: number | null;
  }> {
    return this.request<{
      model_id: string;
      pricing: ModelPricing | null;
      estimated_cost_usd: number | null;
    }>('v1.models.pricing', {
      model_id: modelId,
      ...(tokens || {}),
    });
  }

  // ── Local AI Model & Folder Detection Methods ─────────────────────────────

  async listLocalFolders(): Promise<LocalModelFolder[]> {
    return this.request<LocalModelFolder[]>('v1.local_models.folders.list', {});
  }

  async addLocalFolder(
    folderPath: string,
    label?: string
  ): Promise<{
    folder: LocalModelFolder;
    models_discovered: number;
    models: LocalModelDescriptor[];
  }> {
    return this.request('v1.local_models.folders.add', {
      folder_path: folderPath,
      label,
    });
  }

  async removeLocalFolder(id: string): Promise<{ deleted: boolean; id: string }> {
    return this.request('v1.local_models.folders.remove', { id });
  }

  async scanLocalModels(
    folderId?: string
  ): Promise<{
    scanned_folders: number;
    models_count: number;
    models: LocalModelDescriptor[];
    hardware: LocalHardwareProfile;
    scanned_at: number;
  }> {
    return this.request('v1.local_models.scan', {
      folder_id: folderId,
    });
  }

  async listLocalModels(folderId?: string): Promise<{ models: LocalModelDescriptor[] }> {
    return this.request('v1.local_models.list', {
      folder_id: folderId,
    });
  }

  async getLocalHardware(): Promise<LocalHardwareProfile> {
    return this.request<LocalHardwareProfile>('v1.local_models.hardware', {});
  }

  async preflightLocalModel(
    modelPath: string,
    requestedContext?: number
  ): Promise<LocalModelPreflightResponse> {
    return this.request<LocalModelPreflightResponse>('v1.local_models.preflight', {
      model_path: modelPath,
      requested_context: requestedContext,
    });
  }

  async getLocalModelLoadProfile(
    modelPath: string
  ): Promise<{ profile: LocalModelLoadProfile }> {
    return this.request('v1.local_models.profile.get', { model_path: modelPath });
  }

  async saveLocalModelLoadProfile(
    profile: Omit<LocalModelLoadProfile, 'revision' | 'updated_at'>
  ): Promise<{ profile: LocalModelLoadProfile }> {
    return this.request('v1.local_models.profile.save', profile);
  }

  async ensureLocalModel(
    modelPath: string,
    forceReload?: boolean
  ): Promise<{ instance: LoadedModelInstance; drift_detected: boolean; reloaded: boolean }> {
    return this.request<{ instance: LoadedModelInstance; drift_detected: boolean; reloaded: boolean }>(
      'v1.local_models.ensure',
      {
        model_path: modelPath,
        force_reload: forceReload ?? false,
      }
    );
  }

  async verifyLocalModelGeneration(
    modelPathOrName?: string
  ): Promise<{ verified: boolean; model: string; test_output: string; latency_ms: number }> {
    return this.request<{ verified: boolean; model: string; test_output: string; latency_ms: number }>(
      'v1.local_models.verify',
      {
        model_path: modelPathOrName,
      }
    );
  }

  async loadLocalModel(
    modelPath: string
  ): Promise<LoadedModelInstance> {
    return this.request<LoadedModelInstance>('v1.local_models.load', {
      model_path: modelPath,
    });
  }

  async unloadLocalModel(
    modelPathOrId: string
  ): Promise<{ evicted: boolean; model: string }> {
    return this.request<{ evicted: boolean; model: string }>('v1.local_models.unload', {
      model_path: modelPathOrId,
    });
  }

  async listLoadedInstances(): Promise<{ instances: LoadedModelInstance[] }> {
    return this.request<{ instances: LoadedModelInstance[] }>('v1.local_models.instances', {});
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
    return normalizeTerminalSession(fromDaemon<TerminalSession>(
      await this.request<unknown>('v1.terminal.spawn', toDaemon(params))
    ));
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
    ).map(normalizeTerminalSession);
  }

  async getTerminal(sessionId: string): Promise<TerminalSession> {
    return normalizeTerminalSession(fromDaemon<TerminalSession>(
      await this.request<unknown>('v1.terminal.get', { session_id: sessionId })
    ));
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
    return normalizeWorkspaceFile(
      await this.request<WorkspaceFileContent>('v1.workspace.files.read', {
        workspace_id: workspaceId,
        path,
        max_bytes: maxBytes,
      })
    );
  }

  async writeWorkspaceFile(params: WriteWorkspaceFileParams): Promise<WorkspaceFileContent> {
    return normalizeWorkspaceFile(
      await this.request<WorkspaceFileContent>('v1.workspace.files.write', toDaemon(params))
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

  async commitWorkspace(
    workspaceId: string,
    message: string
  ): Promise<{ committed: boolean; output: string }> {
    return this.request<{ committed: boolean; output: string }>('v1.workspace.git.commit', {
      workspace_id: workspaceId,
      message,
    });
  }

  async listHarnesses(): Promise<HarnessDescriptor[]> {
    return (await this.request<HarnessDescriptor[]>('v1.harness.list', {})).map(normalizeHarness);
  }

  async getHarness(harnessId: string): Promise<HarnessDescriptor> {
    return normalizeHarness(await this.request<HarnessDescriptor>('v1.harness.get', { harness_id: harnessId }));
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

  async getArtifact(path: string, workspaceId?: string): Promise<ArtifactDetailResponse> {
    const detail = await this.request<ArtifactDetailResponse>('v1.artifacts.get', { artifact_path: path, workspace_id: workspaceId });
    return { ...detail, annotations: (detail.annotations || []).map(annotationFromWire) };
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

  // ==========================================
  // Step 10: Scoped Browser Workbench API
  // ==========================================
  async listBrowserSessions(workspaceId?: string): Promise<BrowserSession[]> {
    return this.request<BrowserSession[]>('v1.browser.sessions.list', {
      workspace_id: workspaceId,
    });
  }

  async createBrowserSession(name: string, workspaceId?: string): Promise<BrowserSession> {
    return this.request<BrowserSession>('v1.browser.sessions.create', {
      name,
      workspace_id: workspaceId,
    });
  }

  async listBrowserTabs(sessionId: string): Promise<BrowserTab[]> {
    return this.request<BrowserTab[]>('v1.browser.tabs.list', {
      session_id: sessionId,
    });
  }

  async createBrowserTab(sessionId: string, url: string): Promise<BrowserTab> {
    return this.request<BrowserTab>('v1.browser.tabs.create', {
      session_id: sessionId,
      url,
    });
  }

  async navigateBrowserTab(tabId: string, url: string): Promise<BrowserTab> {
    return this.request<BrowserTab>('v1.browser.tabs.navigate', {
      tab_id: tabId,
      url,
    });
  }

  async snapshotBrowserTab(snapshot: BrowserPageSnapshot | { tab_id: string }): Promise<{ saved: boolean; tab_id: string; snapshot?: BrowserPageSnapshot }> {
    return this.request<{ saved: boolean; tab_id: string; snapshot?: BrowserPageSnapshot }>('v1.browser.tabs.snapshot', snapshot);
  }

  async clickBrowserTab(tabId: string, ref: string): Promise<BrowserTab> {
    return this.request<BrowserTab>('v1.browser.tabs.click', {
      tab_id: tabId,
      ref,
    });
  }

  async fillBrowserTab(tabId: string, ref: string, text: string): Promise<BrowserTab> {
    return this.request<BrowserTab>('v1.browser.tabs.fill', {
      tab_id: tabId,
      ref,
      text,
    });
  }

  async screenshotBrowserTab(tabId: string): Promise<{ path: string; tab_id?: string; image_base64?: string }> {
    return this.request<{ path: string; tab_id?: string; image_base64?: string }>('v1.browser.tabs.screenshot', {
      tab_id: tabId,
    });
  }

  async closeBrowserTab(tabId: string): Promise<{ closed: boolean; tab_id: string }> {
    return this.request<{ closed: boolean; tab_id: string }>('v1.browser.tabs.close', {
      tab_id: tabId,
    });
  }

  // ==========================================
  // Mobile Simulator / Emulator API
  // ==========================================
  async listEmulatorDevices(): Promise<EmulatorDevice[]> {
    return this.request<EmulatorDevice[]>('v1.emulator.devices.list', {});
  }

  async launchEmulatorDevice(params: { device_id?: string; session_id?: string; url?: string }): Promise<EmulatorSession> {
    return this.request<EmulatorSession>('v1.emulator.devices.launch', params);
  }

  async navigateEmulator(sessionId: string, url: string): Promise<{ session_id: string; url: string; status: string }> {
    return this.request<{ session_id: string; url: string; status: string }>('v1.emulator.devices.navigate', {
      session_id: sessionId,
      url,
    });
  }

  async snapshotEmulator(sessionId: string): Promise<EmulatorSnapshotResult> {
    return this.request<EmulatorSnapshotResult>('v1.emulator.devices.snapshot', {
      session_id: sessionId,
    });
  }

  async captureEmulatorScreenshot(sessionId: string): Promise<EmulatorScreenshotResult> {
    return this.request<EmulatorScreenshotResult>('v1.emulator.devices.screenshot', {
      session_id: sessionId,
    });
  }

  async tapEmulator(sessionId: string, ref: string): Promise<{ session_id: string; ref: string; tapped: boolean }> {
    return this.request<{ session_id: string; ref: string; tapped: boolean }>('v1.emulator.devices.tap', {
      session_id: sessionId,
      ref,
    });
  }

  // ==========================================
  // Step 10: Remote SSH Fleet API
  // ==========================================
  async listRemoteHosts(): Promise<RemoteHostNode[]> {
    return this.request<RemoteHostNode[]>('v1.fleet.hosts.list', {});
  }

  async registerRemoteHost(params: RegisterHostParams): Promise<RemoteHostNode> {
    return this.request<RemoteHostNode>('v1.fleet.hosts.register', params);
  }

  async pingRemoteHost(hostId: string): Promise<{ host_id: string; status: string; ping_ms?: number; error?: string }> {
    return this.request<{ host_id: string; status: string; ping_ms?: number; error?: string }>('v1.fleet.hosts.ping', {
      host_id: hostId,
    });
  }

  async execFleet(hostId: string, command: string): Promise<FleetExecReceipt> {
    return this.request<FleetExecReceipt>('v1.fleet.exec', {
      host_id: hostId,
      command,
    });
  }

  // ==========================================
  // Step 10: Headless Automation API
  // ==========================================
  async listHeadlessJobs(status?: string): Promise<HeadlessAutomationJob[]> {
    return this.request<HeadlessAutomationJob[]>('v1.automation.jobs.list', {
      status,
    });
  }

  async createHeadlessJob(params: CreateHeadlessJobParams): Promise<HeadlessAutomationJob> {
    return this.request<HeadlessAutomationJob>('v1.automation.jobs.create', params);
  }

  async runHeadlessJob(jobId: string, permitId?: string): Promise<HeadlessAutomationJob> {
    return this.request<HeadlessAutomationJob>('v1.automation.jobs.run', {
      job_id: jobId,
      permit_id: permitId,
      force_execute: true,
    });
  }

  // ==========================================
  // Step 11: Domain Packs & Workbench Jobs API
  // ==========================================
  async listPacksManifests(): Promise<DomainPackManifest[]> {
    return this.request<DomainPackManifest[]>('v1.packs.manifests.list', {});
  }

  async listPacksJobs(params?: { lens?: string; pack_id?: string }): Promise<PacksJobCatalogItem[]> {
    return this.request<PacksJobCatalogItem[]>('v1.packs.jobs.list', params ?? {});
  }

  async getPacksJob(jobId: string): Promise<PacksJobCatalogItem> {
    return this.request<PacksJobCatalogItem>('v1.packs.jobs.get', { job_id: jobId });
  }

  async listPacksRoles(packId?: string): Promise<WorkerRole[]> {
    return this.request<WorkerRole[]>('v1.packs.roles.list', { pack_id: packId });
  }

  async runPacksJob(params: {
    jobId: string;
    sessionId?: string;
    workspaceId?: string;
    inputs?: Record<string, any>;
  }): Promise<RunPacksJobResult> {
    return this.request<RunPacksJobResult>('v1.packs.jobs.run', {
      job_id: params.jobId,
      session_id: params.sessionId,
      workspace_id: params.workspaceId,
      inputs: params.inputs ?? {},
    });
  }

  // ---------------------------------------------------------
  // Assistant Drafts & Outbox API
  // ---------------------------------------------------------

  async listDrafts(sessionId?: string, taskId?: string): Promise<AssistantDraft[]> {
    const drafts = await this.request<unknown[]>('v1.assistant.drafts.list', {
      session_id: sessionId,
      task_id: taskId,
    });
    return drafts.map((d) => fromDaemon<AssistantDraft>(d));
  }

  async getDraft(id: string): Promise<AssistantDraft> {
    return fromDaemon<AssistantDraft>(await this.request<unknown>('v1.assistant.drafts.get', { id }));
  }

  async saveDraft(draft: Partial<AssistantDraft> & { id?: string; title: string; kind?: string; content: string }): Promise<AssistantDraft> {
    const payload = {
      ...draft,
      id: draft.id || `draft-${Date.now()}`,
      kind: draft.kind || 'email',
    };
    return fromDaemon<AssistantDraft>(await this.request<unknown>('v1.assistant.drafts.save', toDaemon(payload)));
  }

  async deleteDraft(id: string): Promise<boolean> {
    const res = await this.request<{ deleted: boolean }>('v1.assistant.drafts.delete', { id });
    return Boolean(res?.deleted);
  }

  async listOutbox(taskId?: string): Promise<OutboxActionItem[]> {
    const items = await this.request<unknown[]>('v1.assistant.outbox.list', { task_id: taskId });
    return items.map((i) => {
      const value = fromDaemon<OutboxActionItem>(i);
      return { ...value, task_id: value.task_id ?? value.taskId ?? '', action_type: value.action_type ?? value.actionType ?? '',
        payload_summary: value.payload_summary ?? value.payloadSummary ?? '', payload_digest: value.payload_digest ?? value.payloadDigest ?? '',
        permit_id: value.permit_id ?? value.permitId, receipt_id: value.receipt_id ?? value.receiptId, created_at: value.created_at ?? value.createdAt ?? '' };
    });
  }

  async approveOutbox(id: string): Promise<any> {
    return this.request<any>('v1.assistant.outbox.approve', { id });
  }

  async dispatchOutbox(id: string): Promise<any> {
    return this.request<any>('v1.assistant.outbox.dispatch', { id });
  }

  async createOutbox(params: { action_type?: string; task_id?: string; content?: string }): Promise<any> {
    return this.request<any>('v1.assistant.outbox.create', params);
  }

  // ---------------------------------------------------------
  // Multi-Agent Worktree Steering & Capabilities API
  // ---------------------------------------------------------

  async steerWorkspaceAgent(workspaceId: string, instruction: string): Promise<any> {
    return this.request<any>('v1.workspaces.agent_steer', {
      workspace_id: workspaceId,
      instruction,
    });
  }

  async getResourceCapabilities(resourceId: string, workspaceId?: string): Promise<ResourceActionCapability[]> {
    const list = await this.request<unknown[]>('v1.resources.capabilities', {
      resource_id: resourceId,
      workspace_id: workspaceId,
    });
    return list.map((c) => fromDaemon<ResourceActionCapability>(c));
  }

  // ---------------------------------------------------------
  // Compute Targets & GPU Experiment Lifecycle API
  // ---------------------------------------------------------

  async listComputeTargets(): Promise<ComputeTargetInfo[]> {
    const res = await this.request<unknown[]>('v1.compute.targets.list', {});
    return (res || []).map((t) => fromDaemon<ComputeTargetInfo>(t));
  }

  async preflightExperiment(params: {
    backend: string;
    recipe_id?: string;
    ssh_host?: string;
    ssh_port?: number;
    ssh_user?: string;
    pod_id?: string;
    cloud_api_key?: string;
  }): Promise<PreflightReport> {
    const res = await this.request<unknown>('v1.experiment.preflight', params);
    return fromDaemon<PreflightReport>(res);
  }

  async submitExperiment(params: {
    experiment_id: string;
    recipe_id: string;
    backend: string;
    parent_checkpoint_id?: string;
    ssh_host?: string;
    ssh_port?: number;
    ssh_user?: string;
    pod_id?: string;
    cloud_api_key?: string;
  }): Promise<{ attempt: ExecutionAttemptRecord; externalRef: any }> {
    const res = await this.request<{ attempt: unknown; external_ref: unknown }>('v1.experiment.submit', params);
    return {
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
      externalRef: fromDaemon<any>(res.external_ref),
    };
  }

  async observeExperiment(attemptId: string): Promise<{ attempt: ExecutionAttemptRecord; observation: any }> {
    const res = await this.request<{ attempt: unknown; observation: unknown }>('v1.experiment.observe', {
      attempt_id: attemptId,
    });
    return {
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
      observation: fromDaemon<any>(res.observation),
    };
  }

  async stopExperiment(attemptId: string): Promise<{ stopped: boolean; attempt: ExecutionAttemptRecord }> {
    const res = await this.request<{ stopped: boolean; attempt: unknown }>('v1.experiment.stop', {
      attempt_id: attemptId,
    });
    return {
      stopped: res.stopped,
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
    };
  }

  async releaseExperiment(attemptId: string): Promise<{ released: boolean; attempt: ExecutionAttemptRecord }> {
    const res = await this.request<{ released: boolean; attempt: unknown }>('v1.experiment.release', {
      attempt_id: attemptId,
    });
    return {
      released: res.released,
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
    };
  }

  async collectExperimentArtifacts(attemptId: string): Promise<{ artifacts: any[]; attempt: ExecutionAttemptRecord }> {
    const res = await this.request<{ artifacts: any[]; attempt: unknown }>('v1.experiment.collect', {
      attempt_id: attemptId,
    });
    return {
      artifacts: res.artifacts || [],
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
    };
  }
}

export const daemonClient = new DaemonClient();
