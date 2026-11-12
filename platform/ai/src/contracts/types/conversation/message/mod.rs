pub mod content;
mod content_accessors;
pub mod metadata;
pub mod tool;

#[cfg(test)]
mod tests;

pub use content::{
    deserialize_sanitized_content, DocumentContent, ErrorContent, MessageContent,
    MessageContentBlock, MessageErrorKind, RedactedThinkingContentBlock,
    SystemNotificationContent, SystemNotificationType, ThinkingContentBlock,
};
pub use metadata::{
    InferenceMetadata, MessageMetadata, MessageUsage, OperationNotes, TokenState,
};
pub use tool::{
    ActionRequired, ActionRequiredData, ProviderMetadata, ToolCallResult, ToolChainSummary,
    ToolConfirmationRequest, ToolNameParts, ToolRequest, ToolResponse, ToolResult,
    LEGACY_TOOL_META_CHAIN_SUMMARY_KEY, LEGACY_TOOL_META_EXTERNAL_DISPATCH_KEY,
    LEGACY_TOOL_META_PROVIDER_INDEX_KEY, LEGACY_TOOL_META_TITLE_KEY, TOOL_META_CHAIN_SUMMARY_KEY,
    TOOL_META_EXTERNAL_DISPATCH_KEY, TOOL_META_PROVIDER_INDEX_KEY, TOOL_META_TITLE_KEY,
};
pub(crate) use tool::sanitize_tool_result_in_place;

use crate::utils::sanitize_unicode_tags;
use chrono::Utc;
use rmcp::model::{CallToolRequestParams, CallToolResult, JsonObject, PromptMessage, Role, TextContent};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
/// A message to or from an LLM
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: Option<String>,
    pub role: Role,
    pub created: i64,
    #[serde(deserialize_with = "deserialize_sanitized_content")]
    pub content: Vec<MessageContentBlock>,
    pub metadata: MessageMetadata,
}

impl Message {
    pub fn new(role: Role, created: i64, content: Vec<MessageContentBlock>) -> Self {
        Message {
            id: None,
            role,
            created,
            content,
            metadata: MessageMetadata::default(),
        }
    }
    pub fn debug(&self) -> String {
        format!("{:?}", self)
    }

    pub fn agent_visible_content(&self) -> Message {
        let filtered_content = self
            .content
            .iter()
            .filter_map(|c| c.filter_for_audience(Role::Assistant))
            .collect();

        Message {
            content: filtered_content,
            ..self.clone()
        }
    }

    pub fn user_visible_content(&self) -> Message {
        let mut filtered_content: Vec<MessageContent> = Vec::new();
        for content in self
            .content
            .iter()
            .filter_map(MessageContentBlock::user_visible_content)
        {
            match (filtered_content.last_mut(), content) {
                (
                    Some(MessageContentBlock::Text(last_text)),
                    MessageContentBlock::Text(new_text),
                ) if last_text
                    .annotations
                    .as_ref()
                    .and_then(|a| a.audience.as_ref())
                    == new_text
                        .annotations
                        .as_ref()
                        .and_then(|a| a.audience.as_ref()) =>
                {
                    last_text.text.push_str(&new_text.text);
                }
                (_, content) => filtered_content.push(content),
            }
        }

        Message {
            content: filtered_content,
            ..self.clone()
        }
    }

    /// Create a new user message with the current timestamp
    pub fn user() -> Self {
        Message {
            id: None,
            role: Role::User,
            created: Utc::now().timestamp(),
            content: Vec::new(),
            metadata: MessageMetadata::default(),
        }
    }

    /// Create a new assistant message with the current timestamp
    pub fn assistant() -> Self {
        Message {
            id: None,
            role: Role::Assistant,
            created: Utc::now().timestamp(),
            content: Vec::new(),
            metadata: MessageMetadata::default(),
        }
    }

    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_generated_id(self) -> Self {
        self.with_id(format!("msg_{}", Uuid::new_v4()))
    }

    pub fn with_generated_id_if_missing(self) -> Self {
        if self.id.is_some() {
            self
        } else {
            self.with_generated_id()
        }
    }

    /// Add any MessageContent to the message
    pub fn with_content(mut self, content: MessageContentBlock) -> Self {
        self.content.push(content);
        self
    }

    /// Add text content to the message
    pub fn with_text<S: Into<String>>(self, text: S) -> Self {
        let raw_text = text.into();
        let sanitized_text = sanitize_unicode_tags(&raw_text);

        self.with_content(MessageContentBlock::Text(TextContent::new(sanitized_text)))
    }

    /// Add image content to the message
    pub fn with_image<S: Into<String>, T: Into<String>>(self, data: S, mime_type: T) -> Self {
        self.with_content(MessageContentBlock::image(data, mime_type))
    }

    pub fn with_document<S: Into<String>, T: Into<String>>(
        self,
        data: S,
        mime_type: T,
        name: Option<String>,
    ) -> Self {
        self.with_content(MessageContentBlock::document(data, mime_type, name))
    }

    /// Add a tool request to the message
    pub fn with_tool_request<S: Into<String>>(
        self,
        id: S,
        tool_call: ToolResult<CallToolRequestParams>,
    ) -> Self {
        self.with_content(MessageContentBlock::tool_request(id, tool_call))
    }

    pub fn with_tool_request_with_metadata<S: Into<String>>(
        self,
        id: S,
        tool_call: ToolResult<CallToolRequestParams>,
        metadata: Option<&ProviderMetadata>,
        tool_meta: Option<serde_json::Value>,
    ) -> Self {
        self.with_content(MessageContentBlock::ToolRequest(ToolRequest {
            id: id.into(),
            tool_call,
            metadata: metadata.cloned(),
            tool_meta,
        }))
    }

    pub fn with_tool_response<S: Into<String>>(
        self,
        id: S,
        result: ToolResult<CallToolResult>,
    ) -> Self {
        self.with_content(MessageContentBlock::tool_response(id, result))
    }

    pub fn add_tool_response_with_metadata<S: Into<String>>(
        &mut self,
        id: S,
        result: ToolResult<CallToolResult>,
        metadata: Option<&ProviderMetadata>,
    ) {
        self.content
            .push(MessageContentBlock::tool_response_with_metadata(
                id, result, metadata,
            ));
    }

    /// Add an action required message for tool confirmation
    pub fn with_action_required<S: Into<String>>(
        self,
        id: S,
        tool_name: String,
        arguments: JsonObject,
        prompt: Option<String>,
    ) -> Self {
        self.with_content(MessageContentBlock::action_required(
            id, tool_name, arguments, prompt,
        ))
    }

    /// Add thinking content to the message
    pub fn with_thinking<S1: Into<String>, S2: Into<String>>(
        self,
        thinking: S1,
        signature: S2,
    ) -> Self {
        self.with_content(MessageContentBlock::thinking(thinking, signature))
    }

    /// Add redacted thinking content to the message
    pub fn with_redacted_thinking<S: Into<String>>(self, data: S) -> Self {
        self.with_content(MessageContentBlock::redacted_thinking(data))
    }

    /// Get the concatenated text content of the message, separated by newlines
    pub fn as_concat_text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if the message is a tool call
    pub fn is_tool_call(&self) -> bool {
        self.content
            .iter()
            .any(|c| matches!(c, MessageContentBlock::ToolRequest(_)))
    }

    /// Check if the message is a tool response
    pub fn is_tool_response(&self) -> bool {
        self.content
            .iter()
            .any(|c| matches!(c, MessageContentBlock::ToolResponse(_)))
    }

    /// Retrieves all tool `id` from the message
    pub fn get_tool_ids(&self) -> HashSet<&str> {
        self.content
            .iter()
            .filter_map(|content| match content {
                MessageContentBlock::ToolRequest(req) => Some(req.id.as_str()),
                MessageContentBlock::ToolResponse(res) => Some(res.id.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Retrieves all tool `id` from ToolRequest messages
    pub fn get_tool_request_ids(&self) -> HashSet<&str> {
        self.content
            .iter()
            .filter_map(|content| {
                if let MessageContentBlock::ToolRequest(req) = content {
                    Some(req.id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Retrieves all tool `id` from ToolResponse messages
    pub fn get_tool_response_ids(&self) -> HashSet<&str> {
        self.content
            .iter()
            .filter_map(|content| {
                if let MessageContentBlock::ToolResponse(res) = content {
                    Some(res.id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if the message has only TextContent
    pub fn has_only_text_content(&self) -> bool {
        self.content
            .iter()
            .all(|c| matches!(c, MessageContentBlock::Text(_)))
    }

    pub fn with_system_notification<S: Into<String>>(
        self,
        notification_type: SystemNotificationType,
        msg: S,
    ) -> Self {
        self.with_content(MessageContentBlock::system_notification(
            notification_type,
            msg,
        ))
        .with_metadata(MessageMetadata::user_only())
    }

    pub fn with_system_notification_with_data<S: Into<String>>(
        self,
        notification_type: SystemNotificationType,
        msg: S,
        data: serde_json::Value,
    ) -> Self {
        self.with_content(MessageContentBlock::system_notification_with_data(
            notification_type,
            msg,
            data,
        ))
        .with_metadata(MessageMetadata::user_only())
    }

    pub fn with_error<S: Into<String>>(self, kind: MessageErrorKind, message: S) -> Self {
        self.with_content(MessageContentBlock::error(kind, message))
            .with_metadata(MessageMetadata::user_only())
    }

    pub fn from_provider_error(err: &crate::errors::ProviderError) -> Self {
        use crate::errors::ProviderError;
        let text = match err {
            ProviderError::NetworkError(_) => {
                format!("{err}\n\nPlease resend your message to try again.")
            }
            ProviderError::ContextLengthExceeded(_) => {
                format!("{err}\n\nThe conversation is too long for the model's context window.")
            }
            _ => format!(
                "Ran into this error: {err}.\n\n\
                 Please retry if you think this is a transient or recoverable error."
            ),
        };
        Message::assistant().with_error(MessageErrorKind::from(err), text)
    }

    pub fn error_kind(&self) -> Option<MessageErrorKind> {
        self.content
            .iter()
            .find_map(|content| content.as_error().map(|error| error.kind))
    }

    pub fn with_visibility(mut self, user_visible: bool, agent_visible: bool) -> Self {
        self.metadata.user_visible = user_visible;
        self.metadata.agent_visible = agent_visible;
        self
    }

    pub fn with_metadata(mut self, metadata: MessageMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_inference(mut self, inference: InferenceMetadata) -> Self {
        self.metadata = self.metadata.with_inference(inference);
        self
    }

    pub fn with_steer(mut self) -> Self {
        self.metadata.steer = true;
        self
    }

    pub fn with_inference_if_assistant(self, inference: InferenceMetadata) -> Self {
        if self.role == Role::Assistant && self.metadata.inference.is_none() {
            self.with_inference(inference)
        } else {
            self
        }
    }

    pub fn user_only(mut self) -> Self {
        self.metadata.user_visible = true;
        self.metadata.agent_visible = false;
        self
    }

    pub fn agent_only(mut self) -> Self {
        self.metadata.user_visible = false;
        self.metadata.agent_visible = true;
        self
    }

    pub fn is_user_visible(&self) -> bool {
        self.metadata.user_visible
    }

    pub fn is_agent_visible(&self) -> bool {
        self.metadata.agent_visible
    }

    pub fn is_turn_context(&self) -> bool {
        self.metadata.turn_context
    }
}

impl From<PromptMessage> for Message {
    fn from(prompt_message: PromptMessage) -> Self {
        use crate::mcp_utils::extract_text_from_resource;
        use rmcp::model::ContentBlock;

        let message = match prompt_message.role {
            Role::User => Message::user(),
            Role::Assistant => Message::assistant(),
        };

        let content = match prompt_message.content {
            ContentBlock::Text(mut text) => {
                text.text = sanitize_unicode_tags(&text.text);
                MessageContentBlock::Text(text)
            }
            ContentBlock::Image(image) => MessageContentBlock::Image(image),
            ContentBlock::ResourceLink(_) => MessageContentBlock::text("[Resource link]"),
            ContentBlock::Resource(resource) => {
                MessageContentBlock::text(extract_text_from_resource(&resource.resource))
            }
            ContentBlock::Audio(_) => MessageContentBlock::text("[Audio content: not supported]"),
            _ => MessageContentBlock::text("[Unsupported content]"),
        };

        message.with_content(content)
    }
}
