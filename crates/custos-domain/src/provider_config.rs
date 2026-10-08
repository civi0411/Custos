use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub service_type: String, // "anthropic", "openai", "gemini", "local", "deepseek"
    pub api_key_masked: String,
    pub status: String, // "active", "configured", "unconfigured", "error"
    pub endpoint_url: Option<String>,
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
