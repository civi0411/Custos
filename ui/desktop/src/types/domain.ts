// Custos Domain Types
// Strictly mirrors crates/custos-domain and crates/custos-daemon API

export type TaskStatus =
  | 'Draft'
  | 'Active'
  | 'Blocked'
  | 'Completing'
  | 'Done'
  | 'Cancelled';

export type SessionMode =
  | 'autonomous'
  | 'supervised'
  | 'interactive'
  | 'headless';

export type RunStatus =
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
  id: string;
  role: 'user' | 'assistant' | 'system' | 'kernel';
  content: string;
  timestamp: string;
  badge?: string;
  step_name?: string;
  duration?: string;
  metadata?: Record<string, any>;
}

export interface Session {
  id: string;
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

export interface ExecutionWorkspace {
  id: string;
  name: string;
  kind: WorkspaceKind;
  path: string;
  status: WorkspaceStatus;
  status_reason?: string;
  lineage: WorkspaceLineage;
  metadata: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface CreateWorkspaceParams {
  name: string;
  kind: WorkspaceKind;
  path: string;
  lineage?: WorkspaceLineage;
  metadata?: Record<string, any>;
  setup_script?: string;
}
