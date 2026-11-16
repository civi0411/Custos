use super::tool::{
    sanitize_tool_result, sanitize_tool_result_in_place, ActionRequired, ActionRequiredData,
    ProviderMetadata, ToolConfirmationRequest, ToolRequest, ToolResponse, ToolResult,
    TOOL_META_PROVIDER_INDEX_KEY,
};
use crate::utils::sanitize_unicode_tags;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ElicitationAction, ImageContent,
    JsonObject, Role, TextContent,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// Custom deserializer for MessageContent that sanitizes Unicode Tags in text content
pub fn deserialize_sanitized_content<'de, D>(
    deserializer: D,
) -> Result<Vec<MessageContentBlock>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let raw: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;

    let mut migrated = Vec::with_capacity(raw.len());
    for item in raw {
        match item.get("type").and_then(|v| v.as_str()) {
            // Filter out old "conversationCompacted" messages from pre-14.0
            Some("conversationCompacted") => {}
            // Migrate old "reasoning" content to "thinking". Invalid legacy reasoning
            // blocks are dropped so they don't fail deserialization.
            Some("reasoning") => {
                if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    migrated.push(serde_json::json!({
                        "type": "thinking",
                        "thinking": text,
                        "signature": ""
                    }));
                }
            }
            _ => migrated.push(item),
        }
    }

    let mut content: Vec<MessageContentBlock> =
        serde_json::from_value(serde_json::Value::Array(migrated))
            .map_err(|e| Error::custom(format!("Failed to deserialize MessageContent: {}", e)))?;

    for message_content in &mut content {
        match message_content {
            MessageContentBlock::Text(text_content) => {
                let original = &text_content.text;
                let sanitized = sanitize_unicode_tags(original);
                if *original != sanitized {
                    tracing::info!(
                        original = %original,
                        sanitized = %sanitized,
                        removed_count = original.len() - sanitized.len(),
                        "Unicode Tags sanitized during Message deserialization"
                    );
                    text_content.text = sanitized;
                }
            }
            MessageContentBlock::ToolResponse(response) => {
                sanitize_tool_result_in_place(&mut response.tool_result);
            }
            _ => {}
        }
    }

    Ok(content)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingContentBlock {
    pub thinking: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedactedThinkingContentBlock {
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SystemNotificationType {
    ThinkingMessage,
    ProgressMessage,
    InlineMessage,
    CreditsExhausted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemNotificationContent {
    pub notification_type: SystemNotificationType,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageErrorKind {
    Authentication,
    ContextLengthExceeded,
    CreditsExhausted,
    #[serde(other)]
    Other,
}

impl From<&crate::errors::ProviderError> for MessageErrorKind {
    fn from(err: &crate::errors::ProviderError) -> Self {
        use crate::errors::ProviderError;
        match err {
            ProviderError::Authentication(_) => MessageErrorKind::Authentication,
            ProviderError::ContextLengthExceeded(_) => MessageErrorKind::ContextLengthExceeded,
            ProviderError::CreditsExhausted { .. } => MessageErrorKind::CreditsExhausted,
            _ => MessageErrorKind::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorContent {
    pub kind: MessageErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentContent {
    pub data: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl DocumentContent {
    pub fn new<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        Self {
            data: data.into(),
            mime_type: mime_type.into(),
            name: None,
        }
    }

    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }
}

pub type MessageContent = MessageContentBlock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Content passed inside a message, which can be both simple content and tool content
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessageContentBlock {
    Text(TextContent),
    Image(ImageContent),
    Document(DocumentContent),
    ToolRequest(ToolRequest),
    ToolResponse(ToolResponse),
    ToolConfirmationRequest(ToolConfirmationRequest),
    ActionRequired(ActionRequired),
    Thinking(ThinkingContentBlock),
    RedactedThinking(RedactedThinkingContentBlock),
    SystemNotification(SystemNotificationContent),
    Error(ErrorContent),
}

impl fmt::Display for MessageContentBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageContentBlock::Text(t) => write!(f, "{}", t.text),
            MessageContentBlock::Image(i) => write!(f, "[Image: {}]", i.mime_type),
            MessageContentBlock::Document(d) => match &d.name {
                Some(name) => write!(f, "[Document: {} ({})]", name, d.mime_type),
                None => write!(f, "[Document: {}]", d.mime_type),
            },
            MessageContentBlock::ToolRequest(r) => {
                write!(f, "[ToolRequest: {}]", r.to_readable_string())
            }
            MessageContentBlock::ToolResponse(r) => write!(
                f,
                "[ToolResponse: {}]",
                match &r.tool_result {
                    Ok(result) => format!("{} content item(s)", result.content.len()),
                    Err(e) => format!("Error: {e}"),
                }
            ),
            MessageContentBlock::ToolConfirmationRequest(r) => {
                write!(f, "[ToolConfirmationRequest: {}]", r.tool_name)
            }
            MessageContentBlock::ActionRequired(a) => match &a.data {
                ActionRequiredData::ToolConfirmation { tool_name, .. } => {
                    write!(f, "[ActionRequired: ToolConfirmation for {}]", tool_name)
                }
                ActionRequiredData::Elicitation { message, .. } => {
                    write!(f, "[ActionRequired: Elicitation - {}]", message)
                }
                ActionRequiredData::ElicitationResponse { id, .. } => {
                    write!(f, "[ActionRequired: ElicitationResponse for {}]", id)
                }
                ActionRequiredData::ToolConfirmationResponse { id, .. } => {
                    write!(f, "[ActionRequired: ToolConfirmationResponse for {}]", id)
                }
            },
            MessageContentBlock::Thinking(t) => write!(f, "[Thinking: {}]", t.thinking),
            MessageContentBlock::RedactedThinking(_r) => write!(f, "[RedactedThinking]"),
            MessageContentBlock::SystemNotification(r) => {
                write!(f, "[SystemNotification: {}]", r.msg)
            }
            MessageContentBlock::Error(e) => write!(f, "[Error: {}]", e.message),
        }
    }
}

fn content_audience(content: &ContentBlock) -> Option<&Vec<Role>> {
    match content {
        ContentBlock::Text(text) => text.annotations.as_ref()?.audience.as_ref(),
        ContentBlock::Image(image) => image.annotations.as_ref()?.audience.as_ref(),
        ContentBlock::Audio(audio) => audio.annotations.as_ref()?.audience.as_ref(),
        ContentBlock::Resource(resource) => resource.annotations.as_ref()?.audience.as_ref(),
        ContentBlock::ResourceLink(resource) => resource.annotations.as_ref()?.audience.as_ref(),
        _ => None,
    }
}

impl MessageContentBlock {
    pub fn text<S: Into<String>>(text: S) -> Self {
        MessageContentBlock::Text(TextContent::new(text))
    }

    pub fn filter_for_audience(&self, audience: Role) -> Option<MessageContentBlock> {
        match self {
            MessageContentBlock::Text(text) => {
                if text
                    .annotations
                    .as_ref()
                    .and_then(|a| a.audience.as_ref())
                    .map(|roles| roles.contains(&audience))
                    .unwrap_or(true)
                {
                    Some(self.clone())
                } else {
                    None
                }
            }
            MessageContentBlock::Image(img) => {
                if img
                    .annotations
                    .as_ref()
                    .and_then(|a| a.audience.as_ref())
                    .map(|roles| roles.contains(&audience))
                    .unwrap_or(true)
                {
                    Some(self.clone())
                } else {
                    None
                }
            }
            MessageContentBlock::ToolResponse(res) => {
                let Ok(result) = &res.tool_result else {
                    return Some(self.clone());
                };

                let filtered_content: Vec<ContentBlock> = result
                    .content
                    .iter()
                    .filter(|c| {
                        content_audience(c)
                            .map(|roles| roles.contains(&audience))
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect();

                // Preserve ToolResponse even when content is empty - some providers
                // (like Google) need to handle empty tool responses specially
                let mut tool_result = result.clone();
                tool_result.content = filtered_content;
                Some(MessageContentBlock::ToolResponse(ToolResponse {
                    id: res.id.clone(),
                    tool_result: Ok(tool_result),
                    metadata: res.metadata.clone(),
                }))
            }
            MessageContentBlock::Thinking(_) | MessageContentBlock::RedactedThinking(_) => {
                if audience == Role::Assistant {
                    Some(self.clone())
                } else {
                    None
                }
            }
            _ => Some(self.clone()),
        }
    }

    pub fn user_visible_content(&self) -> Option<MessageContent> {
        match self {
            MessageContentBlock::Text(_)
            | MessageContentBlock::Image(_)
            | MessageContentBlock::ToolResponse(_) => self.filter_for_audience(Role::User),
            _ => Some(self.clone()),
        }
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        MessageContentBlock::Image(ImageContent::new(data, mime_type))
    }

    pub fn document<S: Into<String>, T: Into<String>>(
        data: S,
        mime_type: T,
        name: Option<String>,
    ) -> Self {
        let document = DocumentContent::new(data, mime_type);
        MessageContentBlock::Document(match name {
            Some(name) => document.with_name(name),
            None => document,
        })
    }

    pub fn tool_request<S: Into<String>>(
        id: S,
        tool_call: ToolResult<CallToolRequestParams>,
    ) -> Self {
        MessageContentBlock::ToolRequest(ToolRequest {
            id: id.into(),
            tool_call,
            metadata: None,
            tool_meta: None,
        })
    }

    pub fn tool_request_with_metadata<S: Into<String>>(
        id: S,
        tool_call: ToolResult<CallToolRequestParams>,
        metadata: Option<&ProviderMetadata>,
    ) -> Self {
        MessageContentBlock::ToolRequest(ToolRequest {
            id: id.into(),
            tool_call,
            metadata: metadata.cloned(),
            tool_meta: None,
        })
    }

    pub fn tool_request_with_provider_index<S: Into<String>>(
        id: S,
        tool_call: ToolResult<CallToolRequestParams>,
        metadata: Option<&ProviderMetadata>,
        provider_index: i32,
    ) -> Self {
        MessageContentBlock::ToolRequest(ToolRequest {
            id: id.into(),
            tool_call,
            metadata: metadata.cloned(),
            tool_meta: Some(serde_json::json!({
                TOOL_META_PROVIDER_INDEX_KEY: provider_index,
            })),
        })
    }

    pub fn tool_response<S: Into<String>>(id: S, tool_result: ToolResult<CallToolResult>) -> Self {
        MessageContentBlock::ToolResponse(ToolResponse {
            id: id.into(),
            tool_result: sanitize_tool_result(tool_result),
            metadata: None,
        })
    }

    pub fn tool_response_with_metadata<S: Into<String>>(
        id: S,
        tool_result: ToolResult<CallToolResult>,
        metadata: Option<&ProviderMetadata>,
    ) -> Self {
        MessageContentBlock::ToolResponse(ToolResponse {
            id: id.into(),
            tool_result: sanitize_tool_result(tool_result),
            metadata: metadata.cloned(),
        })
    }

    pub fn action_required<S: Into<String>>(
        id: S,
        tool_name: String,
        arguments: JsonObject,
        prompt: Option<String>,
    ) -> Self {
        MessageContentBlock::ActionRequired(ActionRequired {
            data: ActionRequiredData::ToolConfirmation {
                id: id.into(),
                tool_name,
                arguments,
                prompt,
            },
        })
    }

    pub fn action_required_elicitation<S: Into<String>>(
        id: S,
        message: String,
        requested_schema: serde_json::Value,
    ) -> Self {
        MessageContentBlock::ActionRequired(ActionRequired {
            data: ActionRequiredData::Elicitation {
                id: id.into(),
                message,
                requested_schema,
            },
        })
    }

    pub fn action_required_tool_confirmation_response<S: Into<String>>(
        id: S,
        permission: crate::permission::Permission,
    ) -> Self {
        MessageContentBlock::ActionRequired(ActionRequired {
            data: ActionRequiredData::ToolConfirmationResponse {
                id: id.into(),
                permission,
            },
        })
    }

    pub fn action_required_elicitation_response<S: Into<String>>(
        id: S,
        user_data: serde_json::Value,
        action: ElicitationAction,
    ) -> Self {
        MessageContentBlock::ActionRequired(ActionRequired {
            data: ActionRequiredData::ElicitationResponse {
                id: id.into(),
                user_data,
                action,
            },
        })
    }

}
