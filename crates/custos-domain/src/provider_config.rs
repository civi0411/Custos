use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub service_type: String, // "anthropic", "openai", "gemini", "local", "deepseek"
    pub api_key_masked: String,
    pub status: String, // "active", "configured", "unconfigured", "error"
    pub endpoint_url: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub fast_mode: Option<bool>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientApiKeyRecord {
    pub id: String,
    pub name: String,
    pub token: String,
    pub created_at: String,
    pub revoked: bool,
}

/// Persisted OAuth 2.0 token credentials for AI providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthTokenRecord {
    pub provider_id: String,
    pub service_type: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: i64,
    pub token_type: String,
    pub scope: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl OAuthTokenRecord {
    /// Returns true if the token is already expired or expiring within `skew_secs`.
    pub fn is_expired(&self, skew_secs: i64) -> bool {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let expiry_threshold = self.expires_at - (skew_secs * 1000);
        now_ms >= expiry_threshold
    }
}

/// Probed Model discovered live from an endpoint (e.g., Ollama /api/tags, OpenAI /v1/models, Anthropic /models).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbedModel {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub owned_by: Option<String>,
    #[serde(default)]
    pub default_effort: Option<String>,
    #[serde(default)]
    pub efforts: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_currency() -> String {
    "USD".to_string()
}

/// Token pricing in USD per Million tokens (from Open Science + OrCa).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPricing {
    pub input_cost_per_m: f64,
    pub output_cost_per_m: f64,
    #[serde(default)]
    pub cache_read_cost_per_m: Option<f64>,
    #[serde(default)]
    pub cache_write_cost_per_m: Option<f64>,
    #[serde(default = "default_currency")]
    pub currency: String,
}

/// Rich Model Catalog Option (combining OrCa agent-model-catalog & Open Science pricing/context).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelCatalogOption {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    pub provider_type: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub default_effort: Option<String>,
    #[serde(default)]
    pub efforts: Vec<String>,
    #[serde(default)]
    pub supports_fast_mode: bool,
    #[serde(default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub pricing: Option<ModelPricing>,
}

/// Catalog Result answered to UI / clients with cache origin and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelCatalogResult {
    pub origin: String, // "probe", "live-session", "catalog", "cached"
    pub models: Vec<ModelCatalogOption>,
    pub fetched_at: i64,
}

/// User-configured folder path for scanning local GGUF models.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalModelFolder {
    pub id: String,
    pub folder_path: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default = "default_true")]
    pub auto_scan: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

fn default_true() -> bool {
    true
}

/// Compatibility status of a model against local machine VRAM and RAM.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelCompatibility {
    Green,  // Fits 100% in Metal VRAM (Fast GPU inference)
    Yellow, // Fits partially in VRAM / RAM offload
    Red,    // Exceeds total usable memory (OOM risk)
}

impl Default for ModelCompatibility {
    fn default() -> Self {
        Self::Green
    }
}

/// Metadata and performance descriptor for a discovered local GGUF model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalModelDescriptor {
    pub file_path: String,
    pub folder_id: String,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub file_mtime: i64,
    pub architecture: String,
    pub model_name: String,
    pub context_length: u64,
    pub quantization: String,
    #[serde(default)]
    pub chat_template: Option<String>,
    pub total_memory_required: u64,
    pub compatibility: ModelCompatibility,
    pub scanned_at: i64,
}

/// Hardware resource profile (Apple Silicon Metal VRAM & System RAM).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalHardwareProfile {
    pub total_ram_bytes: u64,
    pub metal_vram_limit_bytes: u64,
    pub free_ram_bytes: u64,
    pub is_apple_silicon: bool,
    pub chip_name: String,
}

/// Preflight check request for a local model artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalModelPreflightRequest {
    pub model_path: String,
    #[serde(default)]
    pub requested_context: Option<u64>,
}

/// Preflight check result evaluating memory, architecture, and backend compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalModelPreflightResponse {
    pub model_path: String,
    pub can_load: bool,
    pub architecture: String,
    pub quantization: String,
    pub file_size_bytes: u64,
    pub context_limit: u64,
    pub estimated_memory_bytes: u64,
    pub free_ram_bytes: u64,
    pub metal_vram_limit_bytes: u64,
    pub compatibility: ModelCompatibility,
    /// Backend-derived recommendation when available. `None` means use the
    /// saved `auto` policy; it must not be replaced with a magic layer count.
    #[serde(default)]
    pub recommended_gpu_layers: Option<u32>,
    #[serde(default)]
    pub issues: Vec<String>,
}

/// Request to load a local model into active RAM / Metal VRAM.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalModelLoadRequest {
    pub model_path: String,
    #[serde(default)]
    pub context_size: Option<u64>,
    #[serde(default)]
    pub gpu_layers: Option<u32>,
}

/// Desired load-time configuration for one native model artifact.
///
/// This profile is durable. A loaded instance remains volatile and records the
/// revision that was actually applied, so editing a profile never pretends to
/// mutate weights that are already resident.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalModelLoadProfile {
    pub model_path: String,
    pub revision: u64,
    pub context_size: u64,
    /// `auto`, `max`, `off`, or `manual`.
    pub gpu_mode: String,
    #[serde(default)]
    pub gpu_layers: Option<u32>,
    #[serde(default)]
    pub eval_batch_size: Option<u32>,
    #[serde(default)]
    pub cpu_threads: Option<u32>,
    #[serde(default)]
    pub flash_attention: Option<bool>,
    #[serde(default = "default_true")]
    pub mmap: bool,
    #[serde(default)]
    pub mlock: bool,
    #[serde(default)]
    pub idle_timeout_seconds: Option<u64>,
    pub updated_at: i64,
}

/// Active in-memory loaded model instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoadedModelInstance {
    pub instance_id: String,
    pub model_name: String,
    pub model_path: String,
    pub backend_id: String, // "llamacpp" | "mlx"
    pub context_limit: u64,
    /// Measured resident memory, or zero when the backend cannot report it.
    pub memory_used_bytes: u64,
    pub loaded_at: i64,
    /// Durable desired-profile revision applied to this volatile instance.
    #[serde(default)]
    pub profile_revision: u64,
    /// Effective GPU layer count when known; `None` means backend-managed.
    #[serde(default)]
    pub effective_gpu_layers: Option<u32>,
    /// A successful load is not the same as successful generation.
    #[serde(default)]
    pub generation_verified: bool,
}
