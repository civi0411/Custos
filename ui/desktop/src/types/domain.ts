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
  task_id: string;
  status: RunStatus;
  started_at: string;
  completed_at?: string;
  worker_runs: WorkerRun[];
  metadata: Record<string, any>;
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


