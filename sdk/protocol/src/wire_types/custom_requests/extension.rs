use super::common::EmptyResponse;
use agent_client_protocol::schema::v1::McpServer;
use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GooseExtension {
    Builtin {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundled: Option<bool>,
        /// Tool allowlist for this extension. Omit this field to allow all tools.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        available_tools: Option<Vec<String>>,
    },
    Platform {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundled: Option<bool>,
        /// Tool allowlist for this extension. Omit this field to allow all tools.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        available_tools: Option<Vec<String>>,
    },
    Mcp {
        server: Box<McpServer>,
        #[serde(default, rename = "envKeys", skip_serializing_if = "Vec::is_empty")]
        env_keys: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        socket: Option<String>,
        /// Pre-registered OAuth client ID for the server's authorization server.
        #[serde(default, rename = "clientId", skip_serializing_if = "Option::is_none")]
        client_id: Option<String>,
        /// Name of the env/secret key holding the OAuth client secret.
        #[serde(
            default,
            rename = "clientSecretKey",
            skip_serializing_if = "Option::is_none"
        )]
        client_secret_key: Option<String>,
        /// OAuth scopes to request with `client_id`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        scopes: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundled: Option<bool>,
        /// Tool allowlist for this extension. Omit this field to allow all tools.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        available_tools: Option<Vec<String>>,
    },
}

impl Default for GooseExtension {
    fn default() -> Self {
        Self::Builtin {
            name: String::new(),
            description: None,
            display_name: None,
            timeout: None,
            bundled: None,
            available_tools: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GooseExtensionEntry {
    pub extension: GooseExtension,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionExtensionEntry {
    pub extension: GooseExtension,
    pub extension_key: String,
}

/// List configured extensions and any warnings.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/config/extensions/list",
    response = GetConfigExtensionsResponse
)]
pub struct GetConfigExtensionsRequest {}

/// List configured extensions and any warnings.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct GetConfigExtensionsResponse {
    pub extensions: Vec<GooseExtensionEntry>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

pub type GetExtensionsRequest = GetConfigExtensionsRequest;
pub type GetExtensionsResponse = GetConfigExtensionsResponse;

/// Persist a new extension to the user's global goose config.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/extensions/add", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct AddConfigExtensionRequest {
    pub extension: GooseExtension,
    #[serde(default)]
    pub enabled: bool,
}

/// Remove a persisted extension from the user's global goose config.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/extensions/remove", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct RemoveConfigExtensionRequest {
    pub config_key: String,
}

/// Set the `enabled` flag for a persisted extension in the user's global goose config.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/config/extensions/set-enabled",
    response = EmptyResponse
)]
#[serde(rename_all = "camelCase")]
pub struct SetConfigExtensionEnabledRequest {
    pub config_key: String,
    pub enabled: bool,
}

/// List extensions enabled for an active session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/extensions/list", response = GetSessionExtensionsResponse)]
#[serde(rename_all = "camelCase")]
pub struct GetSessionExtensionsRequest {
    pub session_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct GetSessionExtensionsResponse {
    pub extensions: Vec<SessionExtensionEntry>,
}
