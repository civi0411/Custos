// Domain Packs & Workbench Jobs Types
// Strictly mirrors platform/domain (custos-domain) and apps/daemon (custos-daemon) API

export interface DomainPackManifest {
  id: string;
  name: string;
  version: string;
  lens: string;
  description: string;
  supportedJobs?: string[];
  supported_jobs?: string[];
  supportedRoles?: string[];
  supported_roles?: string[];
}

export interface PacksJobCatalogItem {
  jobId: string;
  job_id?: string;
  lens: 'coding' | 'research' | 'assistant';
  name: string;
  description: string;
  primaryRole: string;
  primary_role?: string;
  recommendedTopology: string;
  recommended_topology?: string;
  status: 'available' | 'degraded' | 'planned';
  degradationReason?: string;
  degradation_reason?: string;
  requiredInputs: string[];
  required_inputs?: string[];
  expectedArtifacts: string[];
  expected_artifacts?: string[];
  requiredVerifiers: string[];
  required_verifiers?: string[];
  acceptanceRubric: string;
  acceptance_rubric?: string;
}

export interface WorkerRole {
  roleId: string;
  role_id?: string;
  packId: string;
  pack_id?: string;
  name: string;
  description: string;
  systemPrompt?: string;
  system_prompt?: string;
  defaultBudgetTokens?: number;
  default_budget_tokens?: number;
  allowedSkills?: string[];
  allowed_skills?: string[];
  readOnly: boolean;
  read_only?: boolean;
}

export interface RunPacksJobResult {
  taskId: string;
  task_id?: string;
  jobId: string;
  job_id?: string;
  status: string;
  lens: string;
  primaryRole?: string;
  primary_role?: string;
  recommendedTopology?: string;
  recommended_topology?: string;
}

export interface PackageInventoryItem {
  component_id: string;
  display_name?: string;
  state: 'builtin' | 'installed' | 'staged' | 'downloading' | 'failed' | 'disabled' | string;
  version: string;
  is_builtin?: boolean;
  manifest_ref?: string;
  installed_at?: string;
  enabled_in_profile?: string | null;
  is_enabled?: boolean;
}
