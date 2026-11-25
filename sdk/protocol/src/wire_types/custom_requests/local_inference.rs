use super::common::EmptyResponse;
use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocalInferenceToolCallingMode {
    #[default]
    Auto,
    ForceNative,
    ForceEmulated,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocalInferenceChatTemplate {
    #[default]
    Embedded,
    Builtin {
        name: String,
    },
    CustomInline {
        template: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum LocalInferenceSamplingConfig {
    Greedy,
    Temperature {
        temperature: f32,
        top_k: i32,
        top_p: f32,
        min_p: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seed: Option<u32>,
    },
    MirostatV2 {
        tau: f32,
        eta: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seed: Option<u32>,
    },
}

impl Default for LocalInferenceSamplingConfig {
    fn default() -> Self {
        Self::Temperature {
            temperature: 0.8,
            top_k: 40,
            top_p: 0.95,
            min_p: 0.05,
            seed: None,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelSettingsDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_model: Option<String>,
    #[serde(default)]
    pub sampling: LocalInferenceSamplingConfig,
    pub repeat_penalty: f32,
    pub repeat_last_n: i32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_batch: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_gpu_layers: Option<u32>,
    pub use_mlock: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flash_attention: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_threads: Option<i32>,
    #[serde(default)]
    pub tool_calling: LocalInferenceToolCallingMode,
    #[serde(default)]
    pub chat_template: LocalInferenceChatTemplate,
    pub enable_thinking: bool,
    pub vision_capable: bool,
    pub image_token_estimate: usize,
    pub mmproj_size_bytes: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum LocalInferenceDownloadState {
    #[default]
    NotDownloaded,
    Downloading,
    Downloaded,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelDownloadStatusDto {
    pub state: LocalInferenceDownloadState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress_percent: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_downloaded: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_bps: Option<u64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceDownloadProgressDto {
    pub model_id: String,
    pub status: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub progress_percent: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_bps: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub task_exited: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelDto {
    pub id: String,
    pub repo_id: String,
    pub filename: String,
    pub quantization: String,
    pub size_bytes: u64,
    pub status: LocalInferenceModelDownloadStatusDto,
    pub recommended: bool,
    pub is_loaded: bool,
    pub settings: LocalInferenceModelSettingsDto,
    pub vision_capable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mmproj_status: Option<LocalInferenceModelDownloadStatusDto>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceHfModelVariantDto {
    pub variant_id: String,
    pub label: String,
    pub backend_id: String,
    pub format: String,
    pub model_id: String,
    pub download_id: String,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    pub description: String,
    pub quality_rank: u8,
    pub sharded: bool,
    pub supported: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsupported_reason: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceHfGgufFileDto {
    pub filename: String,
    pub size_bytes: u64,
    pub quantization: String,
    pub download_url: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceHfModelInfoDto {
    pub repo_id: String,
    pub author: String,
    pub model_name: String,
    pub downloads: u64,
    #[serde(default)]
    pub gguf_files: Vec<LocalInferenceHfGgufFileDto>,
    #[serde(default)]
    pub variants: Vec<LocalInferenceHfModelVariantDto>,
}

/// List locally available inference models.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/models/list",
    response = LocalInferenceModelsListResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelsListRequest {}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelsListResponse {
    pub models: Vec<LocalInferenceModelDto>,
}

/// Download a model for local inference.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/models/download",
    response = LocalInferenceModelDownloadResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelDownloadRequest {
    pub spec: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_id: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelDownloadResponse {
    pub model_id: String,
}

/// Get the progress of a local model download.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/models/download/progress",
    response = LocalInferenceModelDownloadProgressResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelDownloadProgressRequest {
    pub model_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelDownloadProgressResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<LocalInferenceDownloadProgressDto>,
}

/// Cancel a local model download.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/models/download/cancel",
    response = EmptyResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelDownloadCancelRequest {
    pub model_id: String,
}

/// Delete a downloaded local inference model.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/models/delete",
    response = EmptyResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelDeleteRequest {
    pub model_id: String,
}

/// Evict a local inference model from memory.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/models/evict",
    response = EmptyResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelEvictRequest {
    pub model_id: String,
}

/// Read the sampling settings for a local inference model.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/models/settings/read",
    response = LocalInferenceModelSettingsReadResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelSettingsReadRequest {
    pub model_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelSettingsReadResponse {
    pub settings: LocalInferenceModelSettingsDto,
}

/// Update the sampling settings for a local inference model.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/models/settings/update",
    response = LocalInferenceModelSettingsUpdateResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelSettingsUpdateRequest {
    pub model_id: String,
    pub settings: LocalInferenceModelSettingsDto,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceModelSettingsUpdateResponse {
    pub settings: LocalInferenceModelSettingsDto,
}

/// Search Hugging Face for local inference models.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/huggingface/search",
    response = LocalInferenceHuggingFaceSearchResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceHuggingFaceSearchRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceHuggingFaceSearchResponse {
    pub models: Vec<LocalInferenceHfModelInfoDto>,
}

/// List downloadable variants of a Hugging Face model repository.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/huggingface/repo/variants",
    response = LocalInferenceHuggingFaceRepoVariantsResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceHuggingFaceRepoVariantsRequest {
    pub repo_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceHuggingFaceRepoVariantsResponse {
    pub variants: Vec<LocalInferenceHfModelVariantDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_index: Option<usize>,
    pub available_memory_bytes: u64,
    pub downloaded_quants: Vec<String>,
    pub downloaded_variants: Vec<String>,
}

/// List built-in chat templates for local inference.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/local-inference/chat-templates/builtin/list",
    response = LocalInferenceBuiltinChatTemplatesListResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceBuiltinChatTemplatesListRequest {}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LocalInferenceBuiltinChatTemplatesListResponse {
    pub templates: Vec<String>,
}
