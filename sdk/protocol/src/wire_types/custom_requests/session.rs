use super::common::EmptyResponse;
use super::extension::GooseExtension;
use agent_client_protocol::schema::v1::{ContentBlock, Meta, SessionInfo};
use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Add an extension to an active session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/extensions/add", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct AddSessionExtensionRequest {
    pub session_id: String,
    pub extension: GooseExtension,
}

/// Remove an extension from an active session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/extensions/remove", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct RemoveSessionExtensionRequest {
    pub session_id: String,
    pub extension_key: String,
}

/// List all tools available in a session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/tools/list", response = GetToolsResponse)]
#[serde(rename_all = "camelCase")]
pub struct GetToolsRequest {
    pub session_id: String,
    /// Filter tools to those belonging to this extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension_name: Option<String>,
}

/// A single tool item returned by the tools list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolListItem {
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
    pub permission: Option<ToolPermissionLevel>,
    pub input_schema: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

/// Tools response.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct GetToolsResponse {
    pub tools: Vec<ToolListItem>,
}

/// Read a resource from an extension.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/resources/read", response = ReadResourceResponse)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceRequest {
    pub session_id: String,
    pub uri: String,
    pub extension_name: String,
}

/// Resource read response.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct ReadResourceResponse {
    /// The resource result from the extension (MCP ReadResourceResult).
    #[serde(default)]
    pub result: serde_json::Value,
}

/// Call a tool from an extension.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/tools/call", response = GooseToolCallResponse)]
#[serde(rename_all = "camelCase")]
pub struct GooseToolCallRequest {
    pub session_id: String,
    pub extension_name: String,
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// Tool call response.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct GooseToolCallResponse {
    #[serde(default)]
    pub content: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<serde_json::Value>,
    pub is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "_meta")]
    pub meta: Option<serde_json::Value>,
}

/// Update the working directory for a session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/working-dir/update", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkingDirRequest {
    pub session_id: String,
    pub working_dir: String,
}

/// How a session system prompt update should be applied.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionSystemPromptMode {
    /// Replace Goose's base system prompt with the provided text.
    Set,
    /// Append the provided text under Goose's "Additional Instructions" section.
    #[default]
    Append,
}

/// Set, append, or clear system prompt text for a session.
///
/// `mode: "set"` replaces goose's base system prompt. `mode: "append"` adds an
/// instruction under "Additional Instructions". Reusing a key replaces the
/// previous value for that mode/key; sending empty text clears it.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/session/system-prompt/set",
    response = EmptyResponse
)]
#[serde(rename_all = "camelCase")]
pub struct SetSessionSystemPromptRequest {
    pub session_id: String,
    #[serde(default)]
    pub mode: SessionSystemPromptMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub text: String,
}

/// Add user input to the currently active prompt without starting a new prompt.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/session/steer",
    response = SteerSessionResponse
)]
#[serde(rename_all = "camelCase")]
pub struct SteerSessionRequest {
    pub session_id: String,
    #[serde(default)]
    pub prompt: Vec<ContentBlock>,
    pub expected_run_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct SteerSessionResponse {
    pub run_id: String,
    /// Stable id of the queued steer message. The same id later appears as
    /// `messageId` on the streamed `UserMessageChunk` (with `_meta.goose.steer`),
    /// letting clients correlate a queued steer with its pickup.
    pub message_id: String,
}

/// Ask whether a new or existing chat can enter direct Live voice.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/session/live-voice/availability",
    response = LiveVoiceAvailabilityResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LiveVoiceAvailabilityRequest {
    pub session_id: Option<String>,
    #[serde(default, rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LiveVoiceStatus {
    Ready,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LiveVoiceAvailabilityResponse {
    pub status: LiveVoiceStatus,
    pub message: String,
}

/// Get a diagnostic report for a session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/session/live-voice/start",
    response = LiveVoiceStartResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LiveVoiceStartRequest {
    pub session_id: String,
    pub offer_sdp: String,
    #[serde(default, rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct LiveVoiceStartResponse {
    pub interaction_id: String,
    pub answer_sdp: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/session/live-voice/stop",
    response = EmptyResponse
)]
#[serde(rename_all = "camelCase")]
pub struct LiveVoiceStopRequest {
    pub session_id: String,
    pub interaction_id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/diagnostics/get",
    response = DiagnosticsGetResponse
)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsGetRequest {
    pub session_id: String,
    #[serde(default)]
    pub level: DiagnosticsReportLevel,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticsReportLevel {
    #[default]
    Summary,
    Full,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct DiagnosticsGetResponse {
    pub report: serde_json::Value,
}

/// Return list-style metadata for a single session without loading the conversation.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/session/info",
    response = GetSessionInfoResponse
)]
#[serde(rename_all = "camelCase")]
pub struct GetSessionInfoRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct GetSessionInfoResponse {
    pub session: SessionInfo,
}

/// Truncate a session conversation from the given message timestamp onward.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/session/conversation/truncate",
    response = EmptyResponse
)]
#[serde(rename_all = "camelCase")]
pub struct TruncateSessionConversationRequest {
    pub session_id: String,
    pub truncate_from: i64,
}

/// Update the project association for a session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/project/update", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionProjectRequest {
    pub session_id: String,
    pub project_id: Option<String>,
}

/// Rename a session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/rename", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct RenameSessionRequest {
    pub session_id: String,
    pub title: String,
}

/// Archive a session (soft delete).
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/archive", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSessionRequest {
    pub session_id: String,
}

/// Unarchive a previously archived session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/unarchive", response = EmptyResponse)]
#[serde(rename_all = "camelCase")]
pub struct UnarchiveSessionRequest {
    pub session_id: String,
}

/// Export a session as a JSON or markdown string.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/export", response = ExportSessionResponse)]
#[serde(rename_all = "camelCase")]
pub struct ExportSessionRequest {
    pub session_id: String,
    #[serde(default)]
    pub format: SessionExportFormat,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionExportFormat {
    #[default]
    Json,
    Markdown,
}

/// Export session response — raw JSON of the goose session with `conversation`,
/// or a markdown transcript when `format` is `markdown`.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct ExportSessionResponse {
    pub data: String,
}

/// Import a session from a JSON string or share link.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/session/import", response = ImportSessionResponse)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionRequest {
    pub input: String,
    pub source: SessionImportSource,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionImportSource {
    #[default]
    Auto,
    Json,
    Nostr,
}

/// Share a session through Nostr and return its share links.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/session/share/nostr",
    response = ShareSessionNostrResponse
)]
#[serde(rename_all = "camelCase")]
pub struct ShareSessionNostrRequest {
    pub session_id: String,
    pub relays: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ShareSessionNostrResponse {
    pub deeplink: String,
    pub event_id: String,
    pub nevent: String,
    pub relays: Vec<String>,
}

/// Import session response — metadata about the newly created session.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionResponse {
    pub session_id: String,
    pub title: Option<String>,
    pub updated_at: Option<String>,
    pub message_count: u64,
}

/// Permission level for a tool.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermissionLevel {
    AlwaysAllow,
    #[default]
    AskBefore,
    NeverAllow,
}

/// A single tool permission entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermissionEntry {
    pub tool_name: String,
    pub permission: ToolPermissionLevel,
}

/// Set permission levels for one or more tools.
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/tools/permissions/set", response = SetToolPermissionsResponse)]
#[serde(rename_all = "camelCase")]
pub struct SetToolPermissionsRequest {
    pub tool_permissions: Vec<ToolPermissionEntry>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct SetToolPermissionsResponse {}
