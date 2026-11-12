// Browser, Remote Fleet & Automation Types
// Strictly mirrors platform/domain (custos-domain) and apps/daemon (custos-daemon) API

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

export interface EmulatorDevice {
  id: string;
  name: string;
  platform: 'iOS' | 'Android' | 'iPadOS';
  width: number;
  height: number;
  scale: number;
  user_agent: string;
  status: 'ready' | 'running' | 'offline';
}

export interface EmulatorSession {
  session_id: string;
  device_id: string;
  width: number;
  height: number;
  scale: number;
  url: string;
  status: string;
}

export interface EmulatorSnapshotResult {
  session_id: string;
  snapshot: string;
  refs: Record<string, { name?: string; role?: string }>;
  origin: string;
}

export interface EmulatorScreenshotResult {
  session_id: string;
  path?: string;
  image_base64?: string;
}

