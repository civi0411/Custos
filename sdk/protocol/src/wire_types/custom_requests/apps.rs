use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// List available goose apps, optionally scoped to a session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/apps/list", response = AppsListResponse)]
#[serde(rename_all = "camelCase")]
pub struct AppsListRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct AppsListResponse {
    #[serde(default)]
    pub apps: Vec<serde_json::Value>,
}

/// Export a goose app as HTML.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/apps/export", response = AppsExportResponse)]
#[serde(rename_all = "camelCase")]
pub struct AppsExportRequest {
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct AppsExportResponse {
    pub html: String,
}

/// Import a goose app from HTML.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/apps/import", response = AppsImportResponse)]
#[serde(rename_all = "camelCase")]
pub struct AppsImportRequest {
    pub html: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct AppsImportResponse {
    pub name: String,
    pub message: String,
}

/// Delete a goose app by name.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/apps/delete", response = AppsDeleteResponse)]
#[serde(rename_all = "camelCase")]
pub struct AppsDeleteRequest {
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct AppsDeleteResponse {
    pub name: String,
    pub message: String,
}
