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
  DirtyManifest,
  SpawnTerminalParams,
  TerminalOutputChunk,
  TerminalSession,
  CapabilityDescriptor,
  SaveNoteParams,
  PackageInventoryItem,
} from '../types/domain';
import { ArtifactLineageNode } from '../types/research';

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

import { QuantClient } from './workbenches/quant_client';
import { ResearchClient } from './workbenches/research_client';
import { AssistantClient } from './workbenches/assistant_client';
import { CodingClient } from './workbenches/coding_client';
import { ModelsClient } from './workbenches/models_client';
import { RemoteClient } from './workbenches/remote_client';
import { SharedCapabilitiesClient } from './shared_capabilities_client';
import { streamLiveRun } from './live_run';

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

  readonly quant: QuantClient;
  readonly research: ResearchClient;
  readonly assistant: AssistantClient;
  readonly coding: CodingClient;
  readonly models: ModelsClient;
  readonly remote: RemoteClient;
  readonly shared: SharedCapabilitiesClient;

  constructor() {
    this.isTauri =
      typeof window !== 'undefined' &&
      (Boolean((window as any).__TAURI_INTERNALS__) ||
        Boolean((window as any).__TAURI__));

    this.quant = new QuantClient(this);
    this.research = new ResearchClient(this);
    this.assistant = new AssistantClient(this);
    this.coding = new CodingClient(this);
    this.models = new ModelsClient(this);
    this.remote = new RemoteClient(this);
    this.shared = new SharedCapabilitiesClient(this);
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

  async watchRun(
    session: string,
    run: string,
    update: (snapshot: import('../types/domain').LiveRunSnapshot) => void,
    signal: AbortSignal
  ): Promise<void> {
    return streamLiveRun(this, () => this.resolveHttpEndpoint(), session, run, update, signal);
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

  // Tasks API
  createTask = (params: CreateTaskParams): Promise<Task> => this.request<Task>('v1.tasks.create', params);
  getTask = (taskId: string): Promise<Task> => this.request<Task>('v1.tasks.get', { task_id: taskId });
  listTasks = (sessionId?: string): Promise<Task[]> => this.request<Task[]>('v1.tasks.list', { session_id: sessionId });
  cancelTask = (taskId: string, reason?: string): Promise<Task> => this.request<Task>('v1.tasks.cancel', { task_id: taskId, reason });
  advanceTask = (taskId: string, status: Task['status']): Promise<Task> => this.request<Task>('v1.tasks.advance', { task_id: taskId, target_status: status });
  completeTask = (taskId: string, summary?: string, evidenceClaims: any[] = []): Promise<Task> => this.request<Task>('v1.tasks.complete', { task_id: taskId, summary, evidence_claims: evidenceClaims });

  // Workflow & Execution API
  listSessionRuns = (sessionId: string) => this.request<Array<{ run_id: string; model?: string; status: string; journal_occurred_at?: string }>>('v1.workflow.session_runs', { session_id: sessionId });
  startRun = (params: StartRunParams): Promise<RunHandle> => this.request<RunHandle>('v1.workflow.start_run', params);
  cancelRun = (params: CancelRunParams): Promise<void> => this.request<void>('v1.workflow.cancel_run', params);

  // Workflow Telemetry & Execution Supervision
  getSupervisionOverview = (): Promise<import('../types/workflow_telemetry').SupervisionOverview> =>
    this.request('v1.workflow.overview');
  listSupervisionRuns = (params?: { workbench_id?: string; status?: string; limit?: number }): Promise<{ runs: import('../types/workflow_telemetry').SupervisionRunRecord[] }> =>
    this.request('v1.workflow.runs', params || {});
  getRunWaterfall = (runId: string): Promise<import('../types/workflow_telemetry').RunWaterfallView | null> =>
    this.request('v1.workflow.inspect', { run_id: runId });
  listSupervisionLogs = (params?: { run_id?: string; level?: string; span_id?: string; limit?: number }): Promise<{ logs: import('../types/workflow_telemetry').SupervisionLogEntry[] }> =>
    this.request('v1.workflow.logs', params || {});
  listSupervisionFindings = (params?: { run_id?: string; workbench_id?: string; severity?: string; unresolved_only?: boolean; limit?: number }): Promise<{ findings: import('../types/workflow_telemetry').SupervisionFinding[] }> =>
    this.request('v1.workflow.findings', params || {});
  listSupervisionErrors = (params?: { workbench_id?: string; limit?: number }): Promise<{ error_groups: import('../types/workflow_telemetry').ErrorGroupSummary[] }> =>
    this.request('v1.workflow.errors', params || {});
  listSupervisionProfiles = (): Promise<{ profiles: import('../types/workflow_telemetry').SupervisionProfile[] }> =>
    this.request('v1.workflow.profiles');

  // Sessions API
  async createSession(mode: SessionMode = 'assisted'): Promise<Session> {
    const wireMode = mode === 'supervised' || mode === 'interactive' ? 'assisted' : mode === 'autonomous' || mode === 'headless' ? 'bare' : mode;
    return this.request<Session>('v1.sessions.create', { mode: wireMode });
  }
  attachSession = (params: AttachSessionParams): Promise<void> => this.request<void>('v1.sessions.attach', params);
  getSession = (sessionId: string): Promise<Session> => this.request<Session>('v1.sessions.get', { session_id: sessionId });
  listSessions = (): Promise<Session[]> => this.request<Session[]>('v1.sessions.list', {});
  deleteSession = (sessionId: string): Promise<{ deleted: boolean; session_id: string }> => this.request<{ deleted: boolean; session_id: string }>('v1.sessions.delete', { session_id: sessionId });
  getSessionJournal = (sessionId: string): Promise<SessionJournalEntry[]> => this.request<SessionJournalEntry[]>('v1.sessions.journal', { session_id: sessionId });
  openSessionInLens = (sessionId: string, lens: 'assistant' | 'coding' | 'research' | 'quant'): Promise<void> => this.request<void>('v1.sessions.open_in_lens', { session_id: sessionId, lens });
  appendSessionMessage = (sessionId: string, role: string, content: string): Promise<void> => this.request<void>('v1.sessions.message', { session_id: sessionId, role, content });
  promoteSession = (sessionId: string, targetMode: SessionMode): Promise<void> => this.request<void>('v1.sessions.promote', { session_id: sessionId, mode: targetMode });

  // Bridge & OI
  attachBridge = (sessionId: string, target: string): Promise<any> => this.request<any>('v1.bridge.attach', { session_id: sessionId, target });
  steerBridge = (sessionId: string, action: string, parameters: Record<string, any> = {}): Promise<any> => this.request<any>('v1.bridge.steer', { session_id: sessionId, action, parameters });
  explainOI = (taskId: string, budget?: any): Promise<any> => this.request<any>('v1.oi.explain', { task_id: taskId, budget });

  // ExecutionWorkspace API
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
  archiveWorkspace = (workspaceId: string, deletePhysical = false, force = false): Promise<void> => this.request<void>('v1.workspaces.archive', { workspace_id: workspaceId, delete_physical: deletePhysical, force });
  async inspectWorkspaceDirty(workspaceId: string): Promise<DirtyManifest> {
    return fromDaemon<DirtyManifest>(await this.request<unknown>('v1.workspaces.inspect_dirty', { workspace_id: workspaceId }));
  }
  async recoverWorkspace(workspaceId?: string): Promise<ExecutionWorkspace | ExecutionWorkspace[]> {
    const raw = await this.request<unknown>('v1.workspaces.recover', { workspace_id: workspaceId });
    if (Array.isArray(raw)) return raw.map(normalizeWorkspace);
    return normalizeWorkspace(raw);
  }
  detectWorkspace = (path: string): Promise<WorkspaceDetectionResult> => this.request<WorkspaceDetectionResult>('v1.workspaces.detect', { path });
  async pickFolder(purpose?: 'model'): Promise<string | null> {
    const res = await this.request<{ path: string | null }>('v1.workspaces.pick_folder', purpose ? { purpose } : {});
    return res?.path ?? null;
  }

  // Capability Registry
  async listCapabilities(): Promise<CapabilityDescriptor[]> {
    return fromDaemon<CapabilityDescriptor[]>(await this.request<unknown>('v1.capabilities.list', {}));
  }
  async getCapability(targetId: string): Promise<CapabilityDescriptor> {
    return fromDaemon<CapabilityDescriptor>(await this.request<unknown>('v1.capabilities.get', { capability_id: targetId }));
  }

  // Terminal API
  async spawnTerminal(params: SpawnTerminalParams): Promise<TerminalSession> {
    return normalizeTerminalSession(fromDaemon<TerminalSession>(await this.request<unknown>('v1.terminal.spawn', toDaemon(params))));
  }
  writeTerminal = (sessionId: string, data: string): Promise<{ written: number }> => this.request<{ written: number }>('v1.terminal.write', { session_id: sessionId, data });
  resizeTerminal = (sessionId: string, cols: number, rows: number): Promise<{ ok: boolean }> => this.request<{ ok: boolean }>('v1.terminal.resize', { session_id: sessionId, cols, rows });
  async readTerminal(sessionId: string, fromSeq = 0, maxBytes = 65536): Promise<TerminalOutputChunk> {
    return fromDaemon<TerminalOutputChunk>(await this.request<unknown>('v1.terminal.read', { session_id: sessionId, from_seq: fromSeq, max_bytes: maxBytes }));
  }
  terminateTerminal = (sessionId: string): Promise<{ terminated: boolean }> => this.request<{ terminated: boolean }>('v1.terminal.terminate', { session_id: sessionId });
  async listTerminals(workspaceId?: string): Promise<TerminalSession[]> {
    return fromDaemon<TerminalSession[]>(await this.request<unknown>('v1.terminal.list', { workspace_id: workspaceId })).map(normalizeTerminalSession);
  }
  async getTerminal(sessionId: string): Promise<TerminalSession> {
    return normalizeTerminalSession(fromDaemon<TerminalSession>(await this.request<unknown>('v1.terminal.get', { session_id: sessionId })));
  }

  // Shared Capabilities: Artifacts, Lineage, Notes, Notebooks, Reviews & Compute
  listArtifacts = () => this.shared.listArtifacts();
  getArtifact = (path: string, workspaceId?: string) => this.shared.getArtifact(path, workspaceId);
  getArtifactLineageGraph = (path?: string) => this.shared.getArtifactLineageGraph(path);
  recordArtifactLineage = (node: ArtifactLineageNode) => this.shared.recordArtifactLineage(node);
  listNotes = (sessionId?: string) => this.shared.listNotes(sessionId);
  getNote = (id: string) => this.shared.getNote(id);
  saveNote = (params: SaveNoteParams) => this.shared.saveNote(params);
  listNoteVersions = (noteId: string) => this.shared.listNoteVersions(noteId);
  listNotebookCells = (sessionId: string, notebookId?: string) => this.shared.listNotebookCells(sessionId, notebookId);
  saveNotebookCells = (sessionId: string, cells: Parameters<SharedCapabilitiesClient['saveNotebookCells']>[1], notebookId?: string) => this.shared.saveNotebookCells(sessionId, cells, notebookId);
  executeNotebookCell = (params: Parameters<SharedCapabilitiesClient['executeNotebookCell']>[0]) => this.shared.executeNotebookCell(params);
  interruptNotebookKernel = (sessionId: string, notebookId?: string) => this.shared.interruptNotebookKernel(sessionId, notebookId);
  resetNotebookKernel = (sessionId: string, notebookId?: string) => this.shared.resetNotebookKernel(sessionId, notebookId);
  getNotebookKernelStatus = (sessionId: string, notebookId?: string) => this.shared.getNotebookKernelStatus(sessionId, notebookId);
  listReviews = (params?: Parameters<SharedCapabilitiesClient['listReviews']>[0]) => this.shared.listReviews(params);
  getReview = (id: string) => this.shared.getReview(id);
  recordReview = (params: Parameters<SharedCapabilitiesClient['recordReview']>[0]) => this.shared.recordReview(params);
  markReviewStale = (id: string) => this.shared.markReviewStale(id);
  listComputeTargets = () => this.shared.listComputeTargets();
  preflightExperiment = (params: Parameters<SharedCapabilitiesClient['preflightExperiment']>[0]) => this.shared.preflightExperiment(params);
  submitExperiment = (params: Parameters<SharedCapabilitiesClient['submitExperiment']>[0]) => this.shared.submitExperiment(params);
  observeExperiment = (attemptId: string) => this.shared.observeExperiment(attemptId);
  stopExperiment = (attemptId: string) => this.shared.stopExperiment(attemptId);
  releaseExperiment = (attemptId: string) => this.shared.releaseExperiment(attemptId);
  collectExperimentArtifacts = (attemptId: string) => this.shared.collectExperimentArtifacts(attemptId);

  // Settings
  getSettings = (): Promise<Record<string, string>> => this.request<Record<string, string>>('v1.settings.get', {});
  async setSettings(settings: Record<string, string | number | boolean>): Promise<Record<string, string>> {
    const payload: Record<string, string> = {};
    for (const [k, v] of Object.entries(settings)) payload[k] = String(v);
    return this.request<Record<string, string>>('v1.settings.set', payload);
  }

  // Package Inventory
  async listPackageInventory(): Promise<{ items: PackageInventoryItem[] }> {
    const res = await this.request<{ items: PackageInventoryItem[] }>('packages.list_inventory');
    return { items: res.items || [] };
  }
  installPackage = (componentId: string, version: string = '0.1.0'): Promise<any> => this.request('packages.install', { component_id: componentId, version });
  enablePackage = (componentId: string): Promise<any> => this.request('packages.enable', { component_id: componentId });
  disablePackage = (componentId: string): Promise<any> => this.request('packages.disable', { component_id: componentId });

  // ── Delegated Domain Clients (100% backward-compatible forwarders) ──────────

  // Quant workbench
  submitQuantBacktest = (params: Parameters<QuantClient['submitQuantBacktest']>[0]) => this.quant.submitQuantBacktest(params);
  getQuantTearsheet = (strategy_id: string) => this.quant.getQuantTearsheet(strategy_id);
  createQuantOrders = (params: Parameters<QuantClient['createQuantOrders']>[0]) => this.quant.createQuantOrders(params);
  listQuantOrders = (params?: Parameters<QuantClient['listQuantOrders']>[0]) => this.quant.listQuantOrders(params);
  approveQuantOrder = (params: Parameters<QuantClient['approveQuantOrder']>[0]) => this.quant.approveQuantOrder(params);
  discoverQuantFactors = (params?: Parameters<QuantClient['discoverQuantFactors']>[0]) => this.quant.discoverQuantFactors(params);
  triggerQuantKillSwitch = (params?: Parameters<QuantClient['triggerQuantKillSwitch']>[0]) => this.quant.triggerQuantKillSwitch(params);
  evaluateQuantGrid = (params: Parameters<QuantClient['evaluateQuantGrid']>[0]) => this.quant.evaluateQuantGrid(params);
  getQuantMarketBars = (params: Parameters<QuantClient['getQuantMarketBars']>[0]) => this.quant.getQuantMarketBars(params);
  ingestQuantMarketBars = (params: Parameters<QuantClient['ingestQuantMarketBars']>[0]) => this.quant.ingestQuantMarketBars(params);
  getPipelineStatus = () => this.quant.getPipelineStatus();
  runPipeline = (params: Parameters<QuantClient['runPipeline']>[0]) => this.quant.runPipeline(params);
  computeQuantIndicators = (params: Parameters<QuantClient['computeQuantIndicators']>[0]) => this.quant.computeQuantIndicators(params);
  detectPipelineStructure = (content: string) => this.quant.detectPipelineStructure(content);
  evaluateQuantGate = (params: Parameters<QuantClient['evaluateQuantGate']>[0]) => this.quant.evaluateQuantGate(params);
  computeQuantReview = (params: Parameters<QuantClient['computeQuantReview']>[0]) => this.quant.computeQuantReview(params);
  getQuantOrderBook = (params: Parameters<QuantClient['getQuantOrderBook']>[0]) => this.quant.getQuantOrderBook(params);
  computeQuantPbo = (params: Parameters<QuantClient['computeQuantPbo']>[0]) => this.quant.computeQuantPbo(params);

  // Coding workbench
  spawnCandidates = (params: Parameters<CodingClient['spawnCandidates']>[0]) => this.coding.spawnCandidates(params);
  compareCandidates = (params: Parameters<CodingClient['compareCandidates']>[0]) => this.coding.compareCandidates(params);
  runCandidateRace = (params: Parameters<CodingClient['runCandidateRace']>[0]) => this.coding.runCandidateRace(params);
  integrateCandidate = (params: Parameters<CodingClient['integrateCandidate']>[0]) => this.coding.integrateCandidate(params);
  getWorkspaceFileTree = (workspaceId: string, relativeDir?: string, maxDepth?: number) => this.coding.getWorkspaceFileTree(workspaceId, relativeDir, maxDepth);
  readWorkspaceFile = (workspaceId: string, path: string, maxBytes?: number) => this.coding.readWorkspaceFile(workspaceId, path, maxBytes);
  writeWorkspaceFile = (params: Parameters<CodingClient['writeWorkspaceFile']>[0]) => this.coding.writeWorkspaceFile(params);
  getWorkspaceDiff = (workspaceId: string, staged?: boolean) => this.coding.getWorkspaceDiff(workspaceId, staged);
  getWorkspaceFileDiff = (workspaceId: string, path: string, staged?: boolean) => this.coding.getWorkspaceFileDiff(workspaceId, path, staged);
  stageWorkspaceFile = (workspaceId: string, path: string) => this.coding.stageWorkspaceFile(workspaceId, path);
  unstageWorkspaceFile = (workspaceId: string, path: string) => this.coding.unstageWorkspaceFile(workspaceId, path);
  discardWorkspaceFile = (workspaceId: string, path: string) => this.coding.discardWorkspaceFile(workspaceId, path);
  commitWorkspace = (workspaceId: string, message: string) => this.coding.commitWorkspace(workspaceId, message);
  listHarnesses = () => this.coding.listHarnesses();
  getHarness = (harnessId: string) => this.coding.getHarness(harnessId);
  runNativeHarness = (params: Parameters<CodingClient['runNativeHarness']>[0]) => this.coding.runNativeHarness(params);
  cancelHarness = (harnessId: string, runId: string) => this.coding.cancelHarness(harnessId, runId);
  steerHarness = (harnessId: string, runId: string, guidance: string) => this.coding.steerHarness(harnessId, runId, guidance);
  steerWorkspaceAgent = (workspaceId: string, instruction: string) => this.coding.steerWorkspaceAgent(workspaceId, instruction);
  getResourceCapabilities = (resourceId: string, workspaceId?: string) => this.coding.getResourceCapabilities(resourceId, workspaceId);

  // Research workbench
  listResearchSources = () => this.research.listResearchSources();
  saveResearchSource = (source: Parameters<ResearchClient['saveResearchSource']>[0]) => this.research.saveResearchSource(source);
  listResearchAnchors = (sourceId: string) => this.research.listResearchAnchors(sourceId);
  saveResearchAnchor = (anchor: Parameters<ResearchClient['saveResearchAnchor']>[0]) => this.research.saveResearchAnchor(anchor);
  listResearchClaims = () => this.research.listResearchClaims();
  saveResearchClaim = (claim: Parameters<ResearchClient['saveResearchClaim']>[0]) => this.research.saveResearchClaim(claim);
  listResearchRuns = () => this.research.listResearchRuns();
  saveResearchRun = (run: Parameters<ResearchClient['saveResearchRun']>[0]) => this.research.saveResearchRun(run);
  listResearchArtifactLineage = (artifactPath: string) => this.research.listResearchArtifactLineage(artifactPath);
  handoffClaimToCoding = (claimId: string, statement: string) => this.research.handoffClaimToCoding(claimId, statement);
  listResearchRecipes = () => this.research.listResearchRecipes();
  saveResearchRecipe = (recipe: Parameters<ResearchClient['saveResearchRecipe']>[0]) => this.research.saveResearchRecipe(recipe);
  getResearchRecipe = (recipeId: string) => this.research.getResearchRecipe(recipeId);
  listResearchExecutions = (recipeId: string) => this.research.listResearchExecutions(recipeId);
  getResearchExecution = (executionId: string) => this.research.getResearchExecution(executionId);
  assessResearchResume = (executionId: string) => this.research.assessResearchResume(executionId);
  listResearchAnnotations = (artifactId: string, version?: number) => this.research.listResearchAnnotations(artifactId, version);
  saveResearchAnnotation = (annotation: Parameters<ResearchClient['saveResearchAnnotation']>[0]) => this.research.saveResearchAnnotation(annotation);
  listSynthesisProposals = () => this.research.listSynthesisProposals();
  getSynthesisProposal = (id: string) => this.research.getSynthesisProposal(id);
  saveSynthesisProposal = (params: Parameters<ResearchClient['saveSynthesisProposal']>[0]) => this.research.saveSynthesisProposal(params);
  executeSynthesisHandoff = (params: Parameters<ResearchClient['executeSynthesisHandoff']>[0]) => this.research.executeSynthesisHandoff(params);

  // Assistant workbench
  listDrafts = (sessionId?: string, taskId?: string) => this.assistant.listDrafts(sessionId, taskId);
  getDraft = (id: string) => this.assistant.getDraft(id);
  saveDraft = (draft: Parameters<AssistantClient['saveDraft']>[0]) => this.assistant.saveDraft(draft);
  deleteDraft = (id: string) => this.assistant.deleteDraft(id);
  listOutbox = (taskId?: string) => this.assistant.listOutbox(taskId);
  approveOutbox = (id: string) => this.assistant.approveOutbox(id);
  dispatchOutbox = (id: string) => this.assistant.dispatchOutbox(id);
  createOutbox = (params: Parameters<AssistantClient['createOutbox']>[0]) => this.assistant.createOutbox(params);

  // Models, Providers, OAuth
  getProviderRouting = (service: string) => this.models.getProviderRouting(service);
  saveProviderRouting = (service: string, roundRobin: boolean, accountOrder: string[]) => this.models.saveProviderRouting(service, roundRobin, accountOrder);
  reserveProviderAccount = (service: string, model: string, reasoningEffort?: string) => this.models.reserveProviderAccount(service, model, reasoningEffort);
  listProviders = () => this.models.listProviders();
  getProvider = (providerId: string) => this.models.getProvider(providerId);
  saveProvider = (provider: any) => this.models.saveProvider(provider);
  deleteProvider = (providerId: string) => this.models.deleteProvider(providerId);
  getOAuthAuthorizeUrl = (params?: Parameters<ModelsClient['getOAuthAuthorizeUrl']>[0]) => this.models.getOAuthAuthorizeUrl(params);
  exchangeOAuthCode = (params: Parameters<ModelsClient['exchangeOAuthCode']>[0]) => this.models.exchangeOAuthCode(params);
  refreshOAuthToken = (provider_id: string = 'openai') => this.models.refreshOAuthToken(provider_id);
  getOAuthToken = (provider_id: string = 'openai') => this.models.getOAuthToken(provider_id);
  deleteOAuthToken = (provider_id: string = 'openai') => this.models.deleteOAuthToken(provider_id);
  cancelOAuth = () => this.models.cancelOAuth();
  getOAuthStatus = () => this.models.getOAuthStatus();
  startGitHubLogin = () => this.models.startGitHubLogin();
  getGitHubAuthStatus = () => this.models.getGitHubAuthStatus();
  getGitHubViewer = () => this.models.getGitHubViewer();
  probeModels = (baseUrl: string, providerType?: string, apiKey?: string, providerId?: string) => this.models.probeModels(baseUrl, providerType, apiKey, providerId);
  getModelCatalog = (providerType?: string, providerId?: string) => this.models.getModelCatalog(providerType, providerId);
  getModelPricing = (modelId: string, tokens?: Parameters<ModelsClient['getModelPricing']>[1]) => this.models.getModelPricing(modelId, tokens);
  listLocalFolders = () => this.models.listLocalFolders();
  addLocalFolder = (folderPath: string, label?: string) => this.models.addLocalFolder(folderPath, label);
  removeLocalFolder = (id: string) => this.models.removeLocalFolder(id);
  scanLocalModels = (folderId?: string) => this.models.scanLocalModels(folderId);
  listLocalModels = (folderId?: string) => this.models.listLocalModels(folderId);
  getLocalHardware = () => this.models.getLocalHardware();
  preflightLocalModel = (modelPath: string, requestedContext?: number) => this.models.preflightLocalModel(modelPath, requestedContext);
  getLocalModelLoadProfile = (modelPath: string) => this.models.getLocalModelLoadProfile(modelPath);
  saveLocalModelLoadProfile = (profile: Parameters<ModelsClient['saveLocalModelLoadProfile']>[0]) => this.models.saveLocalModelLoadProfile(profile);
  ensureLocalModel = (modelPath: string, forceReload?: boolean) => this.models.ensureLocalModel(modelPath, forceReload);
  verifyLocalModelGeneration = (modelPathOrName?: string) => this.models.verifyLocalModelGeneration(modelPathOrName);
  loadLocalModel = (modelPath: string) => this.models.loadLocalModel(modelPath);
  unloadLocalModel = (modelPathOrId: string) => this.models.unloadLocalModel(modelPathOrId);
  listLoadedInstances = () => this.models.listLoadedInstances();
  listClientKeys = () => this.models.listClientKeys();
  generateClientKey = (name?: string) => this.models.generateClientKey(name);
  revokeClientKey = (keyId: string) => this.models.revokeClientKey(keyId);
  checkLlmStatus = () => this.models.checkLlmStatus();

  // Remote, Browser, Emulator, Fleet, Packs
  listBrowserSessions = (workspaceId?: string) => this.remote.listBrowserSessions(workspaceId);
  createBrowserSession = (name: string, workspaceId?: string) => this.remote.createBrowserSession(name, workspaceId);
  listBrowserTabs = (sessionId: string) => this.remote.listBrowserTabs(sessionId);
  createBrowserTab = (sessionId: string, url: string) => this.remote.createBrowserTab(sessionId, url);
  navigateBrowserTab = (tabId: string, url: string) => this.remote.navigateBrowserTab(tabId, url);
  snapshotBrowserTab = (snapshot: Parameters<RemoteClient['snapshotBrowserTab']>[0]) => this.remote.snapshotBrowserTab(snapshot);
  clickBrowserTab = (tabId: string, ref: string) => this.remote.clickBrowserTab(tabId, ref);
  fillBrowserTab = (tabId: string, ref: string, text: string) => this.remote.fillBrowserTab(tabId, ref, text);
  screenshotBrowserTab = (tabId: string) => this.remote.screenshotBrowserTab(tabId);
  closeBrowserTab = (tabId: string) => this.remote.closeBrowserTab(tabId);
  listEmulatorDevices = () => this.remote.listEmulatorDevices();
  launchEmulatorDevice = (params: Parameters<RemoteClient['launchEmulatorDevice']>[0]) => this.remote.launchEmulatorDevice(params);
  navigateEmulator = (sessionId: string, url: string) => this.remote.navigateEmulator(sessionId, url);
  snapshotEmulator = (sessionId: string) => this.remote.snapshotEmulator(sessionId);
  captureEmulatorScreenshot = (sessionId: string) => this.remote.captureEmulatorScreenshot(sessionId);
  tapEmulator = (sessionId: string, ref: string) => this.remote.tapEmulator(sessionId, ref);
  listRemoteHosts = () => this.remote.listRemoteHosts();
  registerRemoteHost = (params: Parameters<RemoteClient['registerRemoteHost']>[0]) => this.remote.registerRemoteHost(params);
  pingRemoteHost = (hostId: string) => this.remote.pingRemoteHost(hostId);
  execFleet = (hostId: string, command: string) => this.remote.execFleet(hostId, command);
  listHeadlessJobs = (status?: string) => this.remote.listHeadlessJobs(status);
  createHeadlessJob = (params: Parameters<RemoteClient['createHeadlessJob']>[0]) => this.remote.createHeadlessJob(params);
  runHeadlessJob = (jobId: string, permitId?: string) => this.remote.runHeadlessJob(jobId, permitId);
  listPacksManifests = () => this.remote.listPacksManifests();
  listPacksJobs = (params?: Parameters<RemoteClient['listPacksJobs']>[0]) => this.remote.listPacksJobs(params);
  getPacksJob = (jobId: string) => this.remote.getPacksJob(jobId);
  listPacksRoles = (packId?: string) => this.remote.listPacksRoles(packId);
  runPacksJob = (params: Parameters<RemoteClient['runPacksJob']>[0]) => this.remote.runPacksJob(params);
}

export type { PackageInventoryItem };

export const daemonClient = new DaemonClient();
