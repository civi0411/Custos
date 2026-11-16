// Harness & Governed Execution Domain Types
// Strictly mirrors platform/domain (custos-domain) and apps/daemon (custos-daemon) API

export interface GitHubAuthStatusResponse {
  authenticated: boolean;
  source: 'gh_cli' | 'token' | 'oauth';
  installed?: boolean;
  user?: {
    login: string;
    name?: string;
    avatar_url?: string;
    html_url?: string;
    bio?: string;
    public_repos?: number;
    email?: string;
  } | null;
  scopes?: string[];
  details?: string;
  error?: string;
}

export interface AssistantDraft {
  id: string;
  task_id?: string | null;
  taskId?: string | null;
  session_id?: string | null;
  sessionId?: string | null;
  title: string;
  kind: string; // 'email' | 'document' | 'schedule' | 'note'
  content: string;
  recipient?: string | null;
  status: string; // 'draft' | 'approved' | 'archived'
  payload_lock?: string | null;
  payloadLock?: string | null;
  created_at: string;
  createdAt?: string;
  updated_at: string;
  updatedAt?: string;
}

export interface OutboxActionItem {
  id: string;
  task_id: string;
  taskId?: string;
  title: string;
  action_type: string;
  actionType?: string;
  recipient: string;
  payload_summary: string;
  payloadSummary?: string;
  payload_digest: string;
  payloadDigest?: string;
  status: string; // 'pending_approval' | 'approved' | 'dispatching' | 'receipted' | 'failed' | 'uncertain'
  permit_id?: string | null;
  permitId?: string | null;
  receipt_id?: string | null;
  receiptId?: string | null;
  payload_json?: string | null;
  payloadJson?: string | null;
  error_message?: string | null;
  errorMessage?: string | null;
  created_at: string;
  createdAt?: string;
}

export interface ExecutionReceipt {
  receipt_id: string;
  permit_id: string;
  action_id: string;
  status: string;
  output_digest?: string | null;
  output_data?: Record<string, any> | null;
  error_message?: string | null;
  duration_ms: number;
  executed_at: string;
  assurance: string;
}

export interface ResourceActionCapability {
  id: string;
  label: string;
  status: 'available' | 'degraded' | 'unavailable';
  reason?: string | null;
  primary: boolean;
  shortcut?: string | null;
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
  launch_cmd?: string;
  launchCmd?: string;
  maturity?: 'verified_native' | 'discovered_cli';
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

