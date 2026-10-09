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

/// Probed Model discovered live from an endpoint (e.g., Ollama /api/tags, OpenAI /v1/models, Anthropic /models).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbedModel {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub owned_by: Option<String>,
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

