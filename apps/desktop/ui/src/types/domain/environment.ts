// Sovereign Agent Execution Environment (AEE) Domain Models
// Strictly mirrors platform/domain/src/environment and platform/core/src/contracts/environment.rs

import type { AssuranceLevel } from './core';

export type EnvironmentProfile =
  | { lens: 'coding'; repo_path: string; branch: string; worktree_path: string }
  | { lens: 'assistant'; project_id?: string | null }
  | { lens: 'research'; corpus_path: string; kernel_spec?: string | null; dataset_hash?: string | null }
  | { lens: 'quant'; data_partition: string; is_live_gateway: boolean };

export type EnvironmentStateType =
  | 'initializing'
  | 'ready'
  | 'degraded'
  | 'setup_failed'
  | 'torn_down';

export type EnvironmentState =
  | { type: 'initializing' }
  | { type: 'ready' }
  | { type: 'degraded'; reason: string }
  | { type: 'setup_failed'; error: string }
  | { type: 'torn_down' };

export interface EnvironmentAssurance {
  measured_assurance: AssuranceLevel;
  filesystem_isolated: boolean;
  network_restricted: boolean;
  process_isolated: boolean;
  observation_source: string;
  measured_at: number;
}

export interface EnvSnapshot {
  lockfile_hash?: string | null;
  seed?: number | null;
  dataset_hash?: string | null;
  tool_versions: Record<string, string>;
  captured_at: number;
}

export interface EnvironmentSpec {
  env_id: string;
  session_id: string;
  profile: EnvironmentProfile;
  target_assurance: AssuranceLevel;
  sandbox_policy_id?: string | null;
  terminal_policy_id?: string | null;
  harness_profile_id?: string | null;
  compute_lane?: string | null;
  created_at: number;
}

export interface EnvironmentHandle {
  env_id: string;
  spec: EnvironmentSpec;
  state: EnvironmentState;
  assurance: EnvironmentAssurance;
  snapshot?: EnvSnapshot | null;
  workspace_id?: string | null;
  terminal_session_id?: string | null;
}

export interface GateReport {
  env_id: string;
  passed: boolean;
  evaluated_at: number;
  state: EnvironmentState;
  assurance: EnvironmentAssurance;
  violations: string[];
}

export interface ProvisionEnvironmentParams {
  session_id: string;
  profile: EnvironmentProfile;
  target_assurance?: AssuranceLevel;
  sandbox_policy_id?: string;
  terminal_policy_id?: string;
  harness_profile_id?: string;
  compute_lane?: string;
}

export interface VerifyEnvironmentParams {
  env_id: string;
}

export interface TeardownEnvironmentParams {
  env_id: string;
}
