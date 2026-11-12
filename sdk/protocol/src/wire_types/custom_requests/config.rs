use super::common::EmptyResponse;
use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Read allowlisted user preferences. Empty `keys` means all supported preferences.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/preferences/read", response = PreferencesReadResponse)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesReadRequest {
    #[serde(default)]
    pub keys: Vec<PreferenceKey>,
}

/// Save allowlisted user preferences.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/preferences/save", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesSaveRequest {
    #[serde(default)]
    pub values: Vec<PreferenceValue>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PreferenceKey {
    #[default]
    AutoCompactThreshold,
    GooseThinkingEffort,
    VoiceAutoSubmitPhrases,
    VoiceDictationProvider,
    VoiceDictationPreferredMic,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceValue {
    pub key: PreferenceKey,
    #[serde(default)]
    pub value: serde_json::Value,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesReadResponse {
    pub values: Vec<PreferenceValue>,
}

/// Read one goose configuration value.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/read", response = ConfigReadResponse)]
#[serde(rename_all = "camelCase")]
pub struct ConfigReadRequest {
    pub key: String,
    #[serde(default)]
    pub is_secret: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ConfigReadResponse {
    #[serde(default)]
    pub value: serde_json::Value,
}

/// Create or replace one goose configuration value.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/upsert", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpsertRequest {
    pub key: String,
    pub value: serde_json::Value,
    #[serde(default)]
    pub is_secret: bool,
}

/// Remove one goose configuration value.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/remove", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct ConfigRemoveRequest {
    pub key: String,
    #[serde(default)]
    pub is_secret: bool,
}

/// Read all non-secret goose configuration values.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/config/read-all", response = ConfigReadAllResponse)]
#[serde(rename_all = "camelCase")]
pub struct ConfigReadAllRequest {}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ConfigReadAllResponse {
    pub config: HashMap<String, serde_json::Value>,
}

/// Read goose default provider and model configuration.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/defaults/read", response = DefaultsReadResponse)]
#[serde(rename_all = "camelCase")]
pub struct DefaultsReadRequest {}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct DefaultsReadResponse {
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
}

/// Save goose default provider and model configuration.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/defaults/save", response = DefaultsReadResponse)]
#[serde(rename_all = "camelCase")]
pub struct DefaultsSaveRequest {
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

/// Clear goose default provider and model configuration.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/defaults/clear", response = DefaultsReadResponse)]
#[serde(rename_all = "camelCase")]
pub struct DefaultsClearRequest {}

/// Sources that onboarding knows how to discover and import.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingImportSourceKind {
    #[default]
    GooseConfig,
    ClaudeDesktop,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingImportCounts {
    pub providers: u32,
    pub extensions: u32,
    pub sessions: u32,
    pub skills: u32,
    pub projects: u32,
    pub preferences: u32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingImportCandidate {
    pub id: String,
    pub source_kind: OnboardingImportSourceKind,
    pub display_name: String,
    pub path: String,
    pub counts: OnboardingImportCounts,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Scan for existing goose and compatible app data that onboarding can import.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/onboarding/import/scan",
    response = OnboardingImportScanResponse
)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingImportScanRequest {
    /// Empty means all supported import sources.
    #[serde(default)]
    pub sources: Vec<OnboardingImportSourceKind>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingImportScanResponse {
    pub candidates: Vec<OnboardingImportCandidate>,
}

/// Import selected onboarding candidates.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/onboarding/import/apply",
    response = OnboardingImportApplyResponse
)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingImportApplyRequest {
    #[serde(default)]
    pub candidate_ids: Vec<String>,
    #[serde(default)]
    pub enable_imported_extensions: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingImportApplyResponse {
    pub imported: OnboardingImportCounts,
    pub skipped: OnboardingImportCounts,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_defaults: Option<DefaultsReadResponse>,
}
