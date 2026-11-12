// Models, Providers, Local Inference, and OAuth Types
// Strictly mirrors platform/domain (custos-domain) and apps/daemon (custos-daemon) API

export interface ProbedModel {
  default_effort?: string;
  efforts?: string[];
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
  unavailable_models?: Record<string, { reason: string; kind: string; retry_at?: number | null }>;
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

export type ModelCompatibility = 'green' | 'yellow' | 'red';

export interface LocalModelFolder {
  id: string;
  folder_path: string;
  label?: string;
  is_active: boolean;
  auto_scan: boolean;
  created_at: number;
  updated_at: number;
}

export interface LocalModelDescriptor {
  file_path: string;
  folder_id: string;
  file_name: string;
  file_size_bytes: number;
  file_mtime: number;
  architecture: string;
  model_name: string;
  context_length: number;
  quantization: string;
  chat_template?: string;
  total_memory_required: number;
  compatibility: ModelCompatibility;
  scanned_at: number;
}

export interface LocalHardwareProfile {
  total_ram_bytes: number;
  metal_vram_limit_bytes: number;
  free_ram_bytes: number;
  is_apple_silicon: boolean;
  chip_name: string;
}

export interface LocalModelPreflightResponse {
  model_path: string;
  can_load: boolean;
  architecture: string;
  quantization: string;
  file_size_bytes: number;
  context_limit: number;
  estimated_memory_bytes: number;
  free_ram_bytes: number;
  metal_vram_limit_bytes: number;
  compatibility: ModelCompatibility;
  recommended_gpu_layers?: number;
  issues: string[];
}

export interface LoadedModelInstance {
  instance_id: string;
  model_name: string;
  model_path: string;
  backend_id: string;
  context_limit: number;
  /** Measured resident memory in bytes; zero means unavailable, not zero use. */
  memory_used_bytes: number;
  loaded_at: number;
  profile_revision: number;
  effective_gpu_layers?: number;
  generation_verified: boolean;
}

export interface LocalModelLoadProfile {
  model_path: string;
  revision: number;
  context_size: number;
  gpu_mode: 'auto' | 'max' | 'off' | 'manual';
  gpu_layers?: number;
  eval_batch_size?: number;
  cpu_threads?: number;
  flash_attention?: boolean;
  mmap: boolean;
  mlock: boolean;
  idle_timeout_seconds?: number;
  updated_at: number;
}

export interface OAuthAuthorizeResult {
  authorization_url: string;
  code_verifier?: string;
  code_challenge?: string;
  state?: string;
  redirect_uri?: string;
  flow_type?: 'authorization_code_pkce' | 'device_code';
  user_code?: string;
  verification_uri?: string;
  expires_in?: number;
  interval?: number;
}

export interface OAuthConnectionMetadata {
  provider_id: string;
  service_type: string;
  expires_at?: number;
  token_type?: string;
  scope?: string;
  created_at?: number;
  updated_at?: number;
  connected?: boolean;
  has_refresh_token?: boolean;
  access_token?: string;
}

export interface OAuthStatusResult {
  status: 'idle' | 'listening' | 'exchanging' | 'completed' | 'failed';
  port?: number;
  code?: string;
  provider_id?: string;
  error?: string;
}

/** Compatibility name for metadata-only OAuth responses. */
export type OAuthTokenRecord = OAuthConnectionMetadata;

