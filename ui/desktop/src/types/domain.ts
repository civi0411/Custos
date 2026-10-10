// Custos Domain Types
// Strictly mirrors crates/custos-domain and crates/custos-daemon API

export type TaskStatus =
  | 'draft'
  | 'queued'
  | 'running'
  | 'blocked'
  | 'succeeded'
  | 'failed'
  | 'cancelled'
  | 'Draft'
  | 'Active'
  | 'Blocked'
  | 'Completing'
  | 'Done'
  | 'Cancelled';

export type SessionMode =
  | 'bare'
  | 'assisted'
  | { attached: { task_id: string } }
  | 'autonomous'
  | 'supervised'
  | 'interactive'
  | 'headless';

export type RunStatus =
  | 'pending'
  | 'active'
  | 'suspended'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'Pending'
  | 'Running'
  | 'Completed'
  | 'Failed'
  | 'Cancelled'
  | 'Fenced';

export type AssuranceLevel = 'CustosMediated' | 'ProviderGoverned' | 'ObserveOnly' | 'Unknown';

export interface Criterion {
  name: string;
  description?: string;
  mandatory: boolean;
}

export interface TaskContract {
  pack_id?: string;
  name?: string;
  description?: string;
  required_capabilities?: string[];
  evidence_requirements?: unknown[];
  /** Legacy presentation alias; never send this to the daemon. */
  pack?: string;
  criteria?: Criterion[];
  invariants?: string[];
  budget_tokens?: number;
  max_cost_usd?: number;
}

export interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  contract?: TaskContract;
  metadata?: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface SessionJournalEntry {
  id?: string;
  entry_id?: number;
  session_id?: string;
  entry_type?: string;
  entry_data?: string;
  role?: 'user' | 'assistant' | 'system' | 'kernel';
  content?: string;
  timestamp?: string;
  occurred_at?: string;
  badge?: string;
  step_name?: string;
  duration?: string;
  metadata?: Record<string, any>;
}

export interface Session {
  id: string;
  status?: 'active' | 'paused' | 'closed' | { promoted: { task_id: string } };
  current_goal?: string;
  promotion_score?: number;
  attached_to?: string;
  promoted_to?: string;
  task_id?: string;
  mode: SessionMode;
  created_at: string;
  title?: string;
  journal?: SessionJournalEntry[];
}

export interface WorkerRun {
  id: string;
  node_id: string;
  status: RunStatus;
  started_at: string;
  completed_at?: string;
  harness_id?: string;
  metadata?: Record<string, any>;
}

export interface Run {
  id: string;
  run_id?: string;
  task_id: string;
  status: RunStatus;
  started_at: string;
  completed_at?: string;
  worker_runs?: WorkerRun[];
  metadata?: Record<string, any>;
  output?: string;
}

export interface ActionIntent {
  id: string;
  task_id: string;
  action_type: string;
  parameters: Record<string, any>;
  assurance: AssuranceLevel;
  timestamp: string;
}

// IPC Request / Response Types
export interface ApiRequest<T = any> {
  id: string;
  method: string;
  params: T;
}

export interface ApiResponse<T = any> {
  id: string;
  result?: T;
  error?: string;
}

export interface CreateTaskParams {
  title: string;
  contract?: TaskContract;
  metadata?: Record<string, any>;
}

export interface StartRunParams {
  task_id: string;
  instruction?: string;
  preferred_mode?: 'model' | 'native' | 'terminal';
  workspace_root?: string;
  harness_id?: string;
  session_id?: string;
  turn_id?: string;
  prompt?: string;
  model?: string;
}

export interface AttachSessionParams {
  session_id: string;
  task_id: string;
}

export interface CancelRunParams {
  run_id: string;
  reason?: string;
}

// ---------------------------------------------------------
// ExecutionWorkspace Types (RFC 006 / OrCa Integration)
// ---------------------------------------------------------

export type WorkspaceKind =
  | { type: 'git'; repo_path: string; branch: string; base_commit?: string }
  | { type: 'folder'; path: string }
  | { type: 'remote_ssh'; host: string; remote_path: string; user?: string; port?: number };

export type WorkspaceStatus = 'initializing' | 'ready' | 'setup_failed' | 'archived';

export interface WorkspaceLineage {
  parent_workspace_id?: string;
  base_commit?: string;
  target_branch?: string;
  head_commit?: string;
}

export interface DirtyManifest {
  is_dirty: boolean;
  isDirty?: boolean;
  modified_files: string[];
  modifiedFiles?: string[];
  untracked_files: string[];
  untrackedFiles?: string[];
  deleted_files: string[];
  deletedFiles?: string[];
  head_commit?: string | null;
  headCommit?: string | null;
  checked_at: number;
  checkedAt?: number;
}

export interface ExecutionWorkspace {
  id: string;
  name: string;
  kind: WorkspaceKind;
  path: string;
  status: WorkspaceStatus;
  status_reason?: string;
  lineage: WorkspaceLineage;
  owner_task_id?: string | null;
  ownerTaskId?: string | null;
  base_commit_hash?: string | null;
  baseCommitHash?: string | null;
  dirty_manifest?: DirtyManifest | null;
  dirtyManifest?: DirtyManifest | null;
  metadata: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface CreateWorkspaceParams {
  name: string;
  kind: WorkspaceKind;
  path: string;
  lineage?: WorkspaceLineage;
  owner_task_id?: string;
  ownerTaskId?: string;
  metadata?: Record<string, any>;
  setup_script?: string;
}

// ---------------------------------------------------------
// Supervised Invariants, Approval & Evidence Records
// ---------------------------------------------------------

export type ApprovalStatus = 'Pending' | 'Approved' | 'Denied' | 'TimedOut';

export interface ApprovalRequest {
  id: string;
  action_id: string;
  risk_class: string;
  summary: string;
  expires_at: number;
  status: ApprovalStatus;
  created_at: number;
}

export interface ApprovalDecision {
  request_id: string;
  approved: boolean;
  decided_by: string;
  decided_at: number;
  reason?: string;
}

export type CriterionVerificationStatus = 'Pass' | 'Fail' | 'Unknown' | 'Stale';

export interface CriterionVerificationRecord {
  criterion_name: string;
  status: CriterionVerificationStatus;
  method: string;
  version: string;
  source_revision: string;
  verified_at: number;
  evidence_uri?: string;
}

export interface TaskOutcome {
  task_id: string;
  status: 'Succeeded' | 'Failed' | 'Aborted';
  verification_records: CriterionVerificationRecord[];
  summary: string;
  sealed_at: number;
}

// ---------------------------------------------------------
// Capability Registry Contracts
// ---------------------------------------------------------

export type CapabilityStatusType = 'available' | 'degraded' | 'unavailable';

export type CapabilityStatus =
  | { type: 'available' }
  | { type: 'degraded'; reason: string }
  | { type: 'unavailable'; reason: string };

export type CapabilityGroup =
  | 'conversation'
  | 'code'
  | 'compute'
  | 'evidence'
  | 'browser'
  | 'personal'
  | 'coordination';

export interface CapabilityDescriptor {
  id: string;
  title: string;
  group: CapabilityGroup;
  status: CapabilityStatus;
  description: string;
  resourceId?: string | null;
  resource_id?: string | null;
  supportedOperations: string[];
  supported_operations?: string[];
  version: string;
}

// ---------------------------------------------------------
// Terminal / Bounded PTY Contracts
// ---------------------------------------------------------

export type TerminalSessionStatus =
  | { type: 'active' }
  | { type: 'exited'; exit_code?: number | null; exitCode?: number | null }
  | { type: 'terminated' };

export interface TerminalSession {
  id: string;
  workspace_id: string;
  workspaceId?: string;
  working_dir: string;
  workingDir?: string;
  command: string;
  cols: number;
  rows: number;
  status: TerminalSessionStatus;
  created_at: number;
  createdAt?: number;
  closed_at?: number | null;
  closedAt?: number | null;
}

export interface TerminalOutputChunk {
  session_id: string;
  sessionId?: string;
  start_seq: number;
  startSeq?: number;
  next_seq: number;
  nextSeq?: number;
  data: string;
  is_eof: boolean;
  isEof?: boolean;
}

export interface SpawnTerminalParams {
  workspace_id: string;
  workspaceId?: string;
  command?: string;
  working_dir?: string;
  workingDir?: string;
  cols?: number;
  rows?: number;
}

// ---------------------------------------------------------
// Workspace Files & Git Diff Contracts
// ---------------------------------------------------------

export interface WorkspaceFileEntry {
  path: string;
  name: string;
  is_dir: boolean;
  isDir?: boolean;
  size_bytes: number;
  sizeBytes?: number;
  modified_at?: number | null;
  modifiedAt?: number | null;
  is_readonly: boolean;
  isReadonly?: boolean;
}

export interface WorkspaceFileTree {
  workspace_id: string;
  workspaceId?: string;
  root_path: string;
  rootPath?: string;
  relative_dir?: string | null;
  relativeDir?: string | null;
  entries: WorkspaceFileEntry[];
  total_files: number;
  totalFiles?: number;
  total_dirs: number;
  totalDirs?: number;
  truncated: boolean;
}

export interface WorkspaceFileContent {
  workspace_id: string;
  workspaceId?: string;
  path: string;
  content: string;
  is_binary: boolean;
  isBinary?: boolean;
  size_bytes: number;
  sizeBytes?: number;
  truncated: boolean;
  line_count: number;
  lineCount?: number;
}

export interface WriteWorkspaceFileParams {
  workspace_id?: string;
  workspaceId?: string;
  path: string;
  content: string;
  create_parents?: boolean;
  createParents?: boolean;
  overwrite?: boolean;
}

export interface WorkspaceDiffEntry {
  path: string;
  old_path?: string | null;
  oldPath?: string | null;
  status: string; // 'M' | 'A' | 'D' | 'R' | '??'
  is_staged: boolean;
  isStaged?: boolean;
  additions: number;
  deletions: number;
}

export interface WorkspaceDiffSummary {
  workspace_id: string;
  workspaceId?: string;
  head_hash?: string | null;
  headHash?: string | null;
  base_hash?: string | null;
  baseHash?: string | null;
  branch?: string | null;
  files: WorkspaceDiffEntry[];
  raw_diff: string;
  rawDiff?: string;
  total_additions: number;
  totalAdditions?: number;
  total_deletions: number;
  totalDeletions?: number;
  is_clean: boolean;
  isClean?: boolean;
}

export interface WorkspaceFileDiff {
  workspace_id: string;
  workspaceId?: string;
  path: string;
  diff: string;
  is_staged: boolean;
  isStaged?: boolean;
}

export type ToolMediationLevel = 'custos_mediated' | 'provider_governed' | 'observe_only';
export type WorktreeOwnership = 'shared_live' | 'isolated_worktree';
export type CostVisibility = 'exact_tokens' | 'estimated' | 'none';

export interface HarnessProfile {
  harness_id: string;
  harnessId?: string;
  tool_mediation: ToolMediationLevel;
  toolMediation?: ToolMediationLevel;
  worktree_ownership: WorktreeOwnership;
  worktreeOwnership?: WorktreeOwnership;
  supports_cancel: boolean;
  supportsCancel?: boolean;
  supports_steer: boolean;
  supportsSteer?: boolean;
  cost_visibility: CostVisibility;
  costVisibility?: CostVisibility;
}

export interface HarnessDescriptor {
  id: string;
  name: string;
  description: string;
  binary_path: string;
  binaryPath?: string;
  is_available: boolean;
  isAvailable?: boolean;
  profile: HarnessProfile;
}

export interface RunNativeHarnessParams {
  harness_id?: string;
  harnessId?: string;
  instruction: string;
  workspace_id?: string;
  workspaceId?: string;
  cwd?: string;
}

export interface ObservedEffect {
  id: string;
  name: string;
  target: string;
  parameters: Record<string, unknown>;
  risk_level?: string;
  riskLevel?: string;
  assurance: string;
  task_id?: string | null;
  taskId?: string | null;
}

export interface HarnessExecutionResult {
  success: boolean;
  exit_code?: number | null;
  exitCode?: number | null;
  stdout: string;
  stderr: string;
  observed_effects: ObservedEffect[];
  observedEffects?: ObservedEffect[];
  mediation_level: ToolMediationLevel;
  mediationLevel?: ToolMediationLevel;
}

// ---------------------------------------------------------
// Artifact Identity, Lineage DAG & Personal Notes
// ---------------------------------------------------------

export interface ArtifactSummary {
  artifact_path: string;
  artifactPath?: string;
  latest_version: number;
  latestVersion?: number;
  latest_content_hash: string;
  latestContentHash?: string;
  versions_count: number;
  versionsCount?: number;
  produced_by_run_id?: string;
  producedByRunId?: string;
  updated_at: number;
  updatedAt?: number;
}

export interface LineageGraphNode {
  id: string;
  label: string;
  node_type: 'artifact' | 'run' | 'recipe' | 'note';
  nodeType?: 'artifact' | 'run' | 'recipe' | 'note';
  version?: number;
  content_hash?: string;
  contentHash?: string;
  timestamp: number;
  metadata: Record<string, unknown>;
}

export interface LineageGraphEdge {
  from: string;
  to: string;
  edge_type: string;
  edgeType?: string;
}

export interface ArtifactLineageGraph {
  nodes: LineageGraphNode[];
  edges: LineageGraphEdge[];
}

export interface ArtifactDetailResponse {
  artifact_path: string;
  artifactPath?: string;
  latest_version: number;
  latestVersion?: number;
  latest_content_hash: string;
  latestContentHash?: string;
  versions: import('./research').ArtifactLineageNode[];
  annotations: import('./research').AnnotationRecord[];
  content?: string;
}

export interface NoteRecord {
  id: string;
  title: string;
  content: string;
  version: number;
  content_hash: string;
  contentHash?: string;
  session_id?: string;
  sessionId?: string;
  task_id?: string;
  taskId?: string;
  tags: string[];
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export interface NoteVersionRecord {
  id: string;
  note_id: string;
  noteId?: string;
  version: number;
  content: string;
  content_hash: string;
  contentHash?: string;
  created_at: number;
  createdAt?: number;
}

export interface SaveNoteParams {
  id?: string;
  title: string;
  content: string;
  session_id?: string;
  sessionId?: string;
  task_id?: string;
  taskId?: string;
  tags?: string[];
}

export type NotebookCellType = 'code' | 'markdown';
export type CellExecutionStatus = 'idle' | 'running' | 'success' | 'error';
export type KernelStatus = 'idle' | 'busy' | 'interrupted';

export interface NotebookCell {
  id: string;
  session_id: string;
  sessionId?: string;
  cell_type: NotebookCellType;
  cellType?: NotebookCellType;
  source: string;
  cell_index: number;
  cellIndex?: number;
  execution_count?: number | null;
  executionCount?: number | null;
  status: CellExecutionStatus;
  stdout?: string | null;
  stderr?: string | null;
  output_image?: string | null;
  outputImage?: string | null;
  wall_ms?: number | null;
  wallMs?: number | null;
  epoch: number;
  updated_at: number;
  updatedAt?: number;
}

export interface NotebookKernelState {
  session_id: string;
  sessionId?: string;
  epoch: number;
  status: KernelStatus;
  python_version: string;
  pythonVersion?: string;
  execution_counter: number;
  executionCounter?: number;
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export interface ExecuteCellParams {
  session_id: string;
  sessionId?: string;
  cell_id: string;
  cellId?: string;
  code: string;
  workspace_id?: string;
  workspaceId?: string;
  cwd?: string;
}

export interface ExecuteCellResult {
  cell_id: string;
  cellId?: string;
  execution_count: number;
  executionCount?: number;
  status: CellExecutionStatus;
  stdout: string;
  stderr: string;
  output_image?: string | null;
  outputImage?: string | null;
  wall_ms: number;
  wallMs?: number;
  epoch: number;
}

export type ReviewTargetType = 'task' | 'artifact' | 'diff' | 'run' | 'note' | 'workspace';
export type ReviewMethod = 'automated_verifier' | 'peer_review' | 'model_evaluation' | 'contract_proof' | 'runtime_inspection';
export type ReviewStatus = 'approved' | 'rejected' | 'degraded' | 'pending';
export type FindingSeverity = 'info' | 'warning' | 'error' | 'blocker';

export interface ReviewFinding {
  severity: FindingSeverity;
  criterion: string;
  message: string;
  file_path?: string;
  filePath?: string;
  line_number?: number;
  lineNumber?: number;
}

export interface ReviewerRecord {
  id: string;
  target_type: ReviewTargetType;
  targetType?: ReviewTargetType;
  target_id: string;
  targetId?: string;
  reviewer: string;
  method: ReviewMethod;
  status: ReviewStatus;
  evidence_summary: string;
  evidenceSummary?: string;
  evidence_digest?: string | null;
  evidenceDigest?: string | null;
  findings: ReviewFinding[];
  is_fresh: boolean;
  isFresh?: boolean;
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export interface RecordReviewParams {
  target_type: ReviewTargetType;
  target_id: string;
  reviewer: string;
  method: ReviewMethod;
  status: ReviewStatus;
  evidence_summary: string;
  evidence_digest?: string | null;
  findings?: ReviewFinding[];
}

export type SynthesisProposalStatus = 'draft' | 'submitted' | 'handoff_completed' | 'rejected';

export interface ClaimHandoffSummary {
  claim_id: string;
  claimId?: string;
  statement: string;
  level: string;
  confidence_score: number;
  confidenceScore?: number;
  has_fresh_review: boolean;
  hasFreshReview?: boolean;
  sealed_proof_uri?: string | null;
  sealedProofUri?: string | null;
}

export interface RecipeHandoffSummary {
  recipe_id: string;
  recipeId?: string;
  name: string;
  command: string;
  inputs_count: number;
  inputsCount?: number;
  outputs: string[];
}

export interface ResearchSynthesisProposal {
  id: string;
  title: string;
  summary: string;
  claims: ClaimHandoffSummary[];
  recipes: RecipeHandoffSummary[];
  artifact_paths: string[];
  artifactPaths?: string[];
  workspace_id?: string | null;
  workspaceId?: string | null;
  target_branch?: string | null;
  targetBranch?: string | null;
  caveats: string[];
  status: SynthesisProposalStatus;
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export interface SaveSynthesisProposalParams {
  id?: string;
  title: string;
  summary: string;
  claim_ids: string[];
  recipe_ids: string[];
  artifact_paths?: string[];
  workspace_id?: string;
  target_branch?: string;
}

export interface HandoffToCodingParams {
  proposal_id?: string;
  title: string;
  claim_ids: string[];
  recipe_ids: string[];
  artifact_paths?: string[];
  workspace_id?: string;
  target_branch?: string;
  enforce_verification: boolean;
}

export interface HandoffToCodingResult {
  handoff_id: string;
  handoffId?: string;
  proposal_id?: string | null;
  proposalId?: string | null;
  task_ids: string[];
  taskIds?: string[];
  workspace_id?: string | null;
  workspaceId?: string | null;
  target_branch?: string | null;
  targetBranch?: string | null;
  verified_claims_count: number;
  verifiedClaimsCount?: number;
  converted_recipes_count: number;
  convertedRecipesCount?: number;
  caveats: string[];
  timestamp: number;
}

// ==========================================
// Step 10: Browser, Remote Fleet & Automation Types
// ==========================================

export type BrowserTabStatus = 'idle' | 'loading' | 'ready' | 'error' | 'closed';

export interface BrowserConsoleEntry {
  level: string;
  message: string;
  timestamp: number;
}

export interface BrowserNetworkRequest {
  url: string;
  method: string;
  status?: number | null;
  content_type?: string | null;
  contentType?: string | null;
  duration_ms: number;
  durationMs?: number;
  timestamp: number;
}

export interface BrowserPageSnapshot {
  tab_id: string;
  tabId?: string;
  url: string;
  title: string;
  dom_tree_summary: string;
  domTreeSummary?: string;
  text_content: string;
  textContent?: string;
  links: string[];
  viewport_width: number;
  viewportWidth?: number;
  viewport_height: number;
  viewportHeight?: number;
  screenshot_uri?: string | null;
  screenshotUri?: string | null;
  timestamp: number;
}

export interface BrowserTab {
  id: string;
  session_id: string;
  sessionId?: string;
  url: string;
  title: string;
  status: BrowserTabStatus;
  active: boolean;
  last_snapshot?: BrowserPageSnapshot | null;
  lastSnapshot?: BrowserPageSnapshot | null;
  console_logs: BrowserConsoleEntry[];
  consoleLogs?: BrowserConsoleEntry[];
  network_requests: BrowserNetworkRequest[];
  networkRequests?: BrowserNetworkRequest[];
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export interface BrowserSession {
  id: string;
  name: string;
  workspace_id?: string | null;
  workspaceId?: string | null;
  tabs: BrowserTab[];
  active_tab_id?: string | null;
  activeTabId?: string | null;
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export type RemoteHostStatus = 'online' | 'offline' | 'degraded' | 'auth_failed';

export type SshAuthMethod =
  | { type: 'key_pair'; private_key_path: string; passphrase?: string | null }
  | { type: 'agent' }
  | { type: 'password_prompt' };

export interface RemoteHostNode {
  id: string;
  name: string;
  host: string;
  port: number;
  user: string;
  auth_method: SshAuthMethod;
  authMethod?: SshAuthMethod;
  status: RemoteHostStatus;
  labels: Record<string, string>;
  last_ping_ms?: number | null;
  lastPingMs?: number | null;
  os_info?: string | null;
  osInfo?: string | null;
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export interface RegisterHostParams {
  name: string;
  host: string;
  port?: number;
  user: string;
  private_key_path?: string;
  labels?: Record<string, string>;
}

export interface FleetExecReceipt {
  execution_id: string;
  executionId?: string;
  host_id: string;
  hostId?: string;
  command: string;
  exit_code?: number | null;
  exitCode?: number | null;
  stdout: string;
  stderr: string;
  duration_ms: number;
  durationMs?: number;
  verified: boolean;
  timestamp: number;
}

export type HeadlessJobStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled' | 'timed_out';

export type HeadlessTrigger =
  | { type: 'manual' }
  | { type: 'cron'; schedule: string }
  | { type: 'webhook'; endpoint: string }
  | { type: 'on_commit'; branch: string }
  | { type: 'pipeline_step'; parent_task_id: string };

export interface HeadlessTaskSpec {
  workspace_id?: string | null;
  workspaceId?: string | null;
  target_type: string;
  targetType?: string;
  command_or_script: string;
  commandOrScript?: string;
  timeout_secs?: number;
  timeoutSecs?: number;
  env?: Record<string, string>;
  required_evidence?: string[];
  requiredEvidence?: string[];
}

export interface HeadlessAutomationJob {
  id: string;
  name: string;
  spec: HeadlessTaskSpec;
  trigger: HeadlessTrigger;
  status: HeadlessJobStatus;
  exit_code?: number | null;
  exitCode?: number | null;
  output_log: string;
  outputLog?: string;
  created_at: number;
  createdAt?: number;
  started_at?: number | null;
  startedAt?: number | null;
  completed_at?: number | null;
  completedAt?: number | null;
}

export interface CreateHeadlessJobParams {
  name: string;
  spec: HeadlessTaskSpec;
  trigger?: HeadlessTrigger;
}

export interface ProbedModel {
  id: string;
  name?: string;
  context_window?: number;
  owned_by?: string;
  description?: string;
}

export interface ModelPricing {
  input_cost_per_m: number;
  output_cost_per_m: number;
  cache_read_cost_per_m?: number;
  cache_write_cost_per_m?: number;
  currency: string;
}

export interface ModelCatalogOption {
  id: string;
  label: string;
  description?: string;
  provider_type: string;
  is_default: boolean;
  default_effort?: string;
  efforts: string[];
  supports_fast_mode: boolean;
  context_window?: number;
  pricing?: ModelPricing;
}

export interface ModelCatalogResult {
  origin: string;
  models: ModelCatalogOption[];
  fetched_at: number;
}

export interface ProviderConfigRecord {
  id: string;
  name: string;
  service_type: string;
  api_key_masked: string;
  status: string;
  endpoint_url?: string;
  default_model?: string;
  context_window?: number;
  fast_mode?: boolean;
  created_at: number;
  updated_at: number;
}

export interface OAuthAuthorizeResult {
  authorization_url: string;
  code_verifier: string;
  code_challenge: string;
  state: string;
  redirect_uri: string;
}

export interface OAuthTokenRecord {
  provider_id: string;
  service_type: string;
  access_token?: string;
  refresh_token?: string;
  expires_at?: number;
  token_type?: string;
  scope?: string;
  created_at?: number;
  updated_at?: number;
  connected?: boolean;
  has_refresh_token?: boolean;
}

export interface OAuthStatusResult {
  status: 'idle' | 'listening' | 'exchanging' | 'completed' | 'failed';
  port?: number;
  code?: string;
  provider_id?: string;
  error?: string;
}

