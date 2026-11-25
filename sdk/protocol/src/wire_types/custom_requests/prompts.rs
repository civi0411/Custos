use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Information about a prompt template, including its default content and customization status.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplateEntry {
    pub name: String,
    pub description: String,
    pub default_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_content: Option<String>,
    pub is_customized: bool,
}

/// List all available goose prompt templates.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/prompts/list", response = ListPromptsResponse)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsRequest {}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsResponse {
    pub prompts: Vec<PromptTemplateEntry>,
}

/// Read a goose prompt template.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/prompts/get", response = GetPromptResponse)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptRequest {
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptResponse {
    pub name: String,
    pub content: String,
    pub default_content: String,
    pub is_customized: bool,
}

/// Save a custom goose prompt template.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/prompts/save", response = PromptOperationResponse)]
#[serde(rename_all = "camelCase")]
pub struct SavePromptRequest {
    pub name: String,
    pub content: String,
}

/// Reset a goose prompt template to its default content.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/prompts/reset", response = PromptOperationResponse)]
#[serde(rename_all = "camelCase")]
pub struct ResetPromptRequest {
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct PromptOperationResponse {
    pub message: String,
}
