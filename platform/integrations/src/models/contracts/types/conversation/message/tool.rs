use crate::conversation::tool_result_serde;
use crate::utils::sanitize_unicode_tags;
use base64::Engine;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ElicitationAction, JsonObject,
    ResourceContents,
};
use serde::{Deserialize, Serialize};

pub enum ToolCallResult<T> {
    Success { value: T },
    Error { error: String },
}

/// Provider-specific metadata for tool requests/responses.
/// Allows providers to store custom data without polluting the core model.
pub type ProviderMetadata = serde_json::Map<String, serde_json::Value>;
pub type ToolResult<T> = Result<T, rmcp::model::ErrorData>;

pub(crate) fn sanitize_tool_result_in_place(tool_result: &mut ToolResult<CallToolResult>) {
    match tool_result {
        Ok(result) => {
            for content in &mut result.content {
                match content {
                    ContentBlock::Text(text) => {
                        text.text = sanitize_unicode_tags(&text.text);
                    }
                    ContentBlock::Resource(resource) => match &mut resource.resource {
                        ResourceContents::TextResourceContents { text, .. } => {
                            *text = sanitize_unicode_tags(text);
                        }
                        ResourceContents::BlobResourceContents { blob, .. } => {
                            let Ok(bytes) =
                                base64::engine::general_purpose::STANDARD.decode(blob.as_bytes())
                            else {
                                *blob = sanitize_unicode_tags(blob);
                                continue;
                            };
                            let Ok(text) = String::from_utf8(bytes) else {
                                continue;
                            };
                            let sanitized = sanitize_unicode_tags(&text);
                            if text != sanitized {
                                *blob = base64::engine::general_purpose::STANDARD
                                    .encode(sanitized.as_bytes());
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        Err(error) => {
            error.message = sanitize_unicode_tags(error.message.as_ref()).into();
        }
    }
}

pub(crate) fn sanitize_tool_result(
    mut tool_result: ToolResult<CallToolResult>,
) -> ToolResult<CallToolResult> {
    sanitize_tool_result_in_place(&mut tool_result);
    tool_result
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub id: String,
    #[serde(with = "tool_result_serde")]
    pub tool_call: ToolResult<CallToolRequestParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ProviderMetadata>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub tool_meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolNameParts<'a> {
    pub extension_name: Option<&'a str>,
    pub tool_name: &'a str,
}

/// A chain summary persisted on the first tool request of a chain.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolChainSummary {
    pub summary: String,
    pub count: usize,
}

/// Marker key under `ToolRequest.tool_meta` indicating the tool was already
/// executed externally; the agent loop must skip redispatch.
pub const TOOL_META_EXTERNAL_DISPATCH_KEY: &str = "custos.external_dispatch";
pub const LEGACY_TOOL_META_EXTERNAL_DISPATCH_KEY: &str = "goose.external_dispatch";

/// Key under `ToolRequest.tool_meta` storing the LLM-generated short title
/// for this tool call. Used to make the title survive session reload.
pub const TOOL_META_TITLE_KEY: &str = "custos.toolSummary.title";
pub const LEGACY_TOOL_META_TITLE_KEY: &str = "goose.toolSummary.title";

/// Key under `ToolRequest.tool_meta` storing the provider-reported index of the
/// tool call within the streamed response. Streaming clients need this to
/// correlate incremental argument fragments with the right call when a model
/// emits several tool calls in parallel.
pub const TOOL_META_PROVIDER_INDEX_KEY: &str = "custos.toolCall.providerIndex";
pub const LEGACY_TOOL_META_PROVIDER_INDEX_KEY: &str = "goose.toolCall.providerIndex";

/// Key under `ToolRequest.tool_meta` storing the LLM-generated chain summary
/// for the chain that starts at this tool request. Shape: `{ "summary": String,
/// "count": u64 }`. Only attached to the FIRST tool request in a chain.
pub const TOOL_META_CHAIN_SUMMARY_KEY: &str = "custos.toolChain.summary";
pub const LEGACY_TOOL_META_CHAIN_SUMMARY_KEY: &str = "goose.toolChain.summary";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    pub id: String,
    #[serde(with = "tool_result_serde::call_tool_result")]
    pub tool_result: ToolResult<CallToolResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ProviderMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfirmationRequest {
    pub id: String,
    pub tool_name: String,
    pub arguments: JsonObject,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "actionType", rename_all = "camelCase")]
pub enum ActionRequiredData {
    #[serde(rename_all = "camelCase")]
    ToolConfirmation {
        id: String,
        tool_name: String,
        arguments: JsonObject,
        prompt: Option<String>,
    },
    Elicitation {
        id: String,
        message: String,
        requested_schema: serde_json::Value,
    },
    ElicitationResponse {
        id: String,
        user_data: serde_json::Value,
        #[serde(default = "default_elicitation_action")]
        action: ElicitationAction,
    },
    ToolConfirmationResponse {
        id: String,
        permission: crate::permission::Permission,
    },
}

pub(crate) fn default_elicitation_action() -> ElicitationAction {
    ElicitationAction::Accept
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRequired {
    pub data: ActionRequiredData,
}
