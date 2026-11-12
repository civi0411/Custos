use super::common::EmptyResponse;
use super::providers_inventory::RefreshProviderInventoryResponse;
use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigKey {
    pub name: String,
    pub required: bool,
    pub secret: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub oauth_flow: bool,
    #[serde(default)]
    pub device_code_flow: bool,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigFieldValueDto {
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
    pub is_set: bool,
    pub is_secret: bool,
    pub required: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigStatusDto {
    pub provider_id: String,
    pub is_configured: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigFieldUpdate {
    pub key: String,
    pub value: String,
}

/// Read saved configuration field values for one provider.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/config/read",
    response = ProviderConfigReadResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigReadRequest {
    pub provider_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigReadResponse {
    pub fields: Vec<ProviderConfigFieldValueDto>,
}

/// Return provider configured statuses. Empty provider_ids means all providers.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/config/status",
    response = ProviderConfigStatusResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigStatusRequest {
    #[serde(default)]
    pub provider_ids: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigStatusResponse {
    pub statuses: Vec<ProviderConfigStatusDto>,
}

/// Save provider configuration fields and start an inventory refresh when supported.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/config/save",
    response = ProviderConfigChangeResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigSaveRequest {
    pub provider_id: String,
    pub fields: Vec<ProviderConfigFieldUpdate>,
}

/// Delete provider configuration fields and start an inventory refresh when supported.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/config/delete",
    response = ProviderConfigChangeResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigDeleteRequest {
    pub provider_id: String,
}

/// Run a provider-owned native authentication flow and start an inventory refresh when supported.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/config/authenticate",
    response = ProviderConfigChangeResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigAuthenticateRequest {
    pub provider_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigChangeResponse {
    pub status: ProviderConfigStatusDto,
    pub refresh: RefreshProviderInventoryResponse,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSecretStorageDto {
    #[default]
    SecretStore,
    ProviderCache,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSecretStatusDto {
    Valid,
    Expired,
    #[default]
    Unknown,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSecretDto {
    pub id: String,
    pub provider: String,
    pub provider_display_name: String,
    pub name: String,
    pub storage: ProviderSecretStorageDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub status: ProviderSecretStatusDto,
    pub configured: bool,
    pub has_secret: bool,
    pub can_delete: bool,
    pub can_configure: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configure_provider: Option<String>,
}

/// List provider credentials stored locally by goose.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/secrets/list",
    response = ProviderSecretsListResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSecretsListRequest {}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSecretsListResponse {
    pub secrets: Vec<ProviderSecretDto>,
}

/// Delete a locally stored provider credential by id.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/secrets/delete",
    response = EmptyResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSecretDeleteRequest {
    pub id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalModelInfoDto {
    pub provider: String,
    pub model: String,
    pub context_limit: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<usize>,
    pub reasoning: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_token_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_token_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_token_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_token_cost: Option<f64>,
    pub currency: String,
}

/// Look up canonical (bundled-registry) model info for a provider/model pair.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/canonical-model-info",
    response = CanonicalModelInfoResponse
)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalModelInfoRequest {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalModelInfoResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_info: Option<CanonicalModelInfoDto>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTemplateCatalogEntryDto {
    pub provider_id: String,
    pub name: String,
    pub format: String,
    pub api_url: String,
    pub model_count: usize,
    pub doc_url: String,
    pub env_var: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSetupCategoryDto {
    Agent,
    #[default]
    Model,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSetupMethodDto {
    None,
    SingleApiKey,
    ConfigFields,
    HostWithOauthFallback,
    OauthBrowser,
    OauthDeviceCode,
    CloudCredentials,
    Local,
    CliAuth,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSetupGroupDto {
    Default,
    Additional,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSetupFieldDto {
    pub key: String,
    pub label: String,
    pub secret: bool,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSetupCatalogEntryDto {
    pub provider_id: String,
    pub name: String,
    pub category: ProviderSetupCategoryDto,
    /// Whether this provider communicates through ACP.
    #[serde(default)]
    pub acp: bool,
    pub description: String,
    pub setup_method: ProviderSetupMethodDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_connect_query: Option<String>,
    #[serde(default)]
    pub fields: Vec<ProviderSetupFieldDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_url: Option<String>,
    pub group: ProviderSetupGroupDto,
    pub show_only_when_installed: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub supports_install: bool,
    pub supports_auth: bool,
    pub supports_auth_status: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTemplateCapabilitiesDto {
    pub tool_call: bool,
    pub reasoning: bool,
    pub attachment: bool,
    pub temperature: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTemplateModelDto {
    pub id: String,
    pub name: String,
    pub context_limit: usize,
    pub capabilities: ProviderTemplateCapabilitiesDto,
    pub deprecated: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTemplateDto {
    pub provider_id: String,
    pub name: String,
    pub format: String,
    pub api_url: String,
    pub models: Vec<ProviderTemplateModelDto>,
    pub supports_streaming: bool,
    pub env_var: String,
    pub doc_url: String,
}

/// List custom-provider catalog entries. Omit `format` to list all formats.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/catalog/list",
    response = ProviderCatalogListResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogListRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogListResponse {
    pub providers: Vec<ProviderTemplateCatalogEntryDto>,
}

/// List provider setup catalog entries
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/setup/catalog/list",
    response = ProviderSetupCatalogListResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSetupCatalogListRequest {}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSetupCatalogListResponse {
    pub providers: Vec<ProviderSetupCatalogEntryDto>,
}

/// Return the editable template for one catalog provider.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/catalog/template",
    response = ProviderCatalogTemplateResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogTemplateRequest {
    pub provider_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogTemplateResponse {
    pub template: ProviderTemplateDto,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderConfigDto {
    pub provider_id: String,
    pub engine: String,
    pub display_name: String,
    pub api_url: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_streaming: Option<bool>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub requires_auth: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_path: Option<String>,
    pub toolshim: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    pub api_key_set: bool,
    pub preserves_thinking: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderUpsertDto {
    pub engine: String,
    pub display_name: String,
    pub api_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_streaming: Option<bool>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub requires_auth: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserves_thinking: Option<bool>,
}

/// Create a custom provider backed by goose's declarative provider store.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/custom/create",
    response = CustomProviderCreateResponse
)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderCreateRequest {
    #[serde(flatten)]
    pub provider: CustomProviderUpsertDto,
    pub toolshim: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderCreateResponse {
    pub provider_id: String,
    pub status: ProviderConfigStatusDto,
    pub refresh: RefreshProviderInventoryResponse,
}

/// Read a declarative provider config. Custom configs are editable; bundled configs are read-only.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/custom/read",
    response = CustomProviderReadResponse
)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderReadRequest {
    pub provider_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderReadResponse {
    pub provider: CustomProviderConfigDto,
    pub editable: bool,
    pub status: ProviderConfigStatusDto,
}

/// Update a custom provider backed by goose's declarative provider store.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/custom/update",
    response = CustomProviderUpdateResponse
)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderUpdateRequest {
    pub provider_id: String,
    #[serde(flatten)]
    pub provider: CustomProviderUpsertDto,
    pub toolshim: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderUpdateResponse {
    pub provider_id: String,
    pub status: ProviderConfigStatusDto,
    pub refresh: RefreshProviderInventoryResponse,
}

/// Delete a custom provider from goose's declarative provider store.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/providers/custom/delete",
    response = CustomProviderDeleteResponse
)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderDeleteRequest {
    pub provider_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderDeleteResponse {
    pub provider_id: String,
    pub refresh: RefreshProviderInventoryResponse,
}
