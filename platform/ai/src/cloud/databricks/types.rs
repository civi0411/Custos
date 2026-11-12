use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::cloud::transport::api_client::ApiClient;
use super::auth::{DatabricksAuth, DatabricksRefreshHook, DatabricksSessionIdProvider};
use crate::images::ImageFormat;
use crate::retry::RetryConfig;

pub const DEFAULT_PROVIDER_TIMEOUT_SECS: u64 = 600;
pub const DATABRICKS_ENDPOINT_METADATA_TIMEOUT_SECS: u64 = 5;
pub const DATABRICKS_ENDPOINT_METADATA_TTL_SECS: u64 = 60;

pub const DATABRICKS_DEFAULT_MODEL: &str = "databricks-claude-sonnet-4";
pub const DATABRICKS_KNOWN_MODELS: &[&str] = &[
    "databricks-claude-sonnet-4-5",
    "databricks-meta-llama-3-3-70b-instruct",
    "databricks-meta-llama-3-1-405b-instruct",
];
pub const DATABRICKS_DOC_URL: &str =
    "https://docs.databricks.com/en/generative-ai/external-models/index.html";

#[derive(Debug, Clone)]
pub struct DatabricksEndpointInfo {
    pub name: String,
    pub upstream_model_name: Option<String>,
    pub upstream_model_provider: Option<String>,
    pub reasoning: Option<bool>,
    pub supports_responses_api: bool,
}

#[derive(Debug, Clone)]
pub struct DatabricksUpstreamModel {
    pub name: String,
    pub provider: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CachedDatabricksEndpointInfo {
    pub info: Option<DatabricksEndpointInfo>,
    pub fetched_at: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EndpointMetadataLookup {
    ContextDiscovery,
    InferenceRouting,
}

impl CachedDatabricksEndpointInfo {
    pub fn applies_to(&self, lookup: EndpointMetadataLookup) -> bool {
        self.fetched_at.elapsed() < Duration::from_secs(DATABRICKS_ENDPOINT_METADATA_TTL_SECS)
            && (lookup == EndpointMetadataLookup::ContextDiscovery || self.info.is_some())
    }
}

pub static DATABRICKS_ENDPOINT_INFO_CACHE: LazyLock<
    Mutex<HashMap<String, CachedDatabricksEndpointInfo>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(serde::Serialize)]
pub struct DatabricksProvider {
    #[serde(skip)]
    pub(crate) api_client: ApiClient,
    #[serde(skip)]
    pub(crate) host: String,
    pub(crate) auth: DatabricksAuth,
    pub(crate) image_format: ImageFormat,
    #[serde(skip)]
    pub(crate) retry_config: RetryConfig,
    #[serde(skip)]
    pub(crate) name: String,
    #[serde(skip)]
    pub(crate) token_cache: Arc<Mutex<Option<String>>>,
    #[serde(skip)]
    pub(crate) instance_id: Option<String>,
    #[serde(skip)]
    pub(crate) refresh_hook: Option<DatabricksRefreshHook>,
    #[serde(skip)]
    pub(crate) session_id_provider: Option<DatabricksSessionIdProvider>,
}

