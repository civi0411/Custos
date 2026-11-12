/**
 * Canonical types for Run execution telemetry, waterfalls, and watchdog diagnostics.
 */

export type StepSpanKind =
  | 'llm'
  | 'tool'
  | 'agent'
  | 'error'
  | 'harness'
  | 'session'
  | 'verification';

export type StepSpanStatus = 'running' | 'success' | 'error';

export type LogSeverity = 'trace' | 'debug' | 'info' | 'warn' | 'error' | 'fatal';

export type FindingKind =
  | 'long_running'
  | 'repeated_tool'
  | 'repeated_error'
  | 'no_activity'
  | 'possible_loop';

export type FindingSeverity = 'info' | 'warning' | 'critical';

export interface RunStepSpan {
  id: string;
  run_id: string;
  worker_run_id?: string | null;
  parent_id?: string | null;
  kind: StepSpanKind;
  name: string;
  status: StepSpanStatus;
  started_at_ms: number;
  ended_at_ms?: number | null;
  duration_ms?: number | null;
  metadata: Record<string, any>;
}

export interface StepError {
  id: string;
  run_id: string;
  worker_run_id?: string | null;
  span_id?: string | null;
  error_type: string;
  message: string;
  signature_hash: string;
  stack?: string | null;
  agent: string;
  timestamp_ms: number;
}

export interface SupervisionLogEntry {
  id: string;
  run_id: string;
  worker_run_id?: string | null;
  span_id?: string | null;
  timestamp_ms: number;
  level: LogSeverity;
  message: string;
  metadata: Record<string, any>;
}

export interface SupervisionFinding {
  id: string;
  run_id: string;
  worker_run_id?: string | null;
  kind: FindingKind;
  severity: FindingSeverity;
  message: string;
  created_at_ms: number;
  resolved_at_ms?: number | null;
}

export interface ErrorGroupSummary {
  signature_hash: string;
  error_type: string;
  sample_message: string;
  count: number;
  first_seen_ms: number;
  last_seen_ms: number;
  affected_runs: string[];
}

export interface SupervisionRunRecord {
  run_id: string;
  workbench_id?: string | null;
  agent: string;
  status: 'running' | 'success' | 'error';
  started_at_ms: number;
  ended_at_ms?: number | null;
  duration_ms?: number | null;
  tokens?: number | null;
  cost_usd?: number | null;
  usage_measurement?: { certainty: string; value?: number };
}

export interface SupervisionOverview {
  active_runs: number;
  total_runs: number;
  failed_runs: number;
  open_findings: number;
  critical_findings: number;
  dropped_events: number;
  registered_profiles: string[];
}

export interface RunWaterfallView {
  run: SupervisionRunRecord;
  spans: RunStepSpan[];
  logs: SupervisionLogEntry[];
  errors: StepError[];
  error_groups: ErrorGroupSummary[];
  findings: SupervisionFinding[];
  applied_profile: SupervisionProfile;
}

export interface ThresholdPolicy {
  long_running_ms: number;
  repeated_tool_count: number;
  repeated_error_count: number;
  no_activity_ms: number;
  loop_repeat_count: number;
  possible_loop_window: number;
}

export interface SupervisionProfile {
  profile_id: string;
  workbench_id?: string | null;
  thresholds: ThresholdPolicy;
  exempt_span_kinds: StepSpanKind[];
  secret_attribute_keys: string[];
  redaction_enabled: boolean;
  max_payload_bytes: number;
}

