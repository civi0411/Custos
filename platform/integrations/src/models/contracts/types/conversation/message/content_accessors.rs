use super::content::{
    ErrorContent, MessageContentBlock, MessageErrorKind, RedactedThinkingContentBlock,
    SystemNotificationContent, SystemNotificationType, ThinkingContentBlock,
};
use super::tool::{ActionRequired, ToolRequest, ToolResponse};
use crate::mcp_utils::extract_text_from_resource;
use rmcp::model::ContentBlock;

impl MessageContentBlock {
    pub fn thinking<S1: Into<String>, S2: Into<String>>(thinking: S1, signature: S2) -> Self {
        MessageContentBlock::Thinking(ThinkingContentBlock {
            thinking: thinking.into(),
            signature: signature.into(),
        })
    }

    pub fn redacted_thinking<S: Into<String>>(data: S) -> Self {
        MessageContentBlock::RedactedThinking(RedactedThinkingContentBlock { data: data.into() })
    }

    pub fn system_notification<S: Into<String>>(
        notification_type: SystemNotificationType,
        msg: S,
    ) -> Self {
        MessageContentBlock::SystemNotification(SystemNotificationContent {
            notification_type,
            msg: msg.into(),
            data: None,
        })
    }

    pub fn system_notification_with_data<S: Into<String>>(
        notification_type: SystemNotificationType,
        msg: S,
        data: serde_json::Value,
    ) -> Self {
        MessageContentBlock::SystemNotification(SystemNotificationContent {
            notification_type,
            msg: msg.into(),
            data: Some(data),
        })
    }

    pub fn error<S: Into<String>>(kind: MessageErrorKind, message: S) -> Self {
        MessageContentBlock::Error(ErrorContent {
            kind,
            message: message.into(),
        })
    }

    pub fn as_error(&self) -> Option<&ErrorContent> {
        if let MessageContentBlock::Error(error) = self {
            Some(error)
        } else {
            None
        }
    }

    pub fn as_system_notification(&self) -> Option<&SystemNotificationContent> {
        if let MessageContentBlock::SystemNotification(ref notification) = self {
            Some(notification)
        } else {
            None
        }
    }

    pub fn as_tool_request(&self) -> Option<&ToolRequest> {
        if let MessageContentBlock::ToolRequest(ref tool_request) = self {
            Some(tool_request)
        } else {
            None
        }
    }

    pub fn as_tool_response(&self) -> Option<&ToolResponse> {
        if let MessageContentBlock::ToolResponse(ref tool_response) = self {
            Some(tool_response)
        } else {
            None
        }
    }

    pub fn as_action_required(&self) -> Option<&ActionRequired> {
        if let MessageContentBlock::ActionRequired(ref action_required) = self {
            Some(action_required)
        } else {
            None
        }
    }

    pub fn as_tool_response_text(&self) -> Option<String> {
        if let Some(tool_response) = self.as_tool_response() {
            if let Ok(result) = &tool_response.tool_result {
                let texts: Vec<String> = result
                    .content
                    .iter()
                    .filter_map(|content| content.as_text().map(|t| t.text.to_string()))
                    .collect();
                if !texts.is_empty() {
                    return Some(texts.join("\n"));
                }
            }
        }
        None
    }

    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContentBlock::Text(text) => Some(&text.text),
            _ => None,
        }
    }

    /// Get the thinking content if this is a ThinkingContent variant
    pub fn as_thinking(&self) -> Option<&ThinkingContentBlock> {
        match self {
            MessageContentBlock::Thinking(thinking) => Some(thinking),
            _ => None,
        }
    }

    /// Get the redacted thinking content if this is a RedactedThinkingContent variant
    pub fn as_redacted_thinking(&self) -> Option<&RedactedThinkingContentBlock> {
        match self {
            MessageContentBlock::RedactedThinking(redacted) => Some(redacted),
            _ => None,
        }
    }
}

impl From<ContentBlock> for MessageContentBlock {
    fn from(content: ContentBlock) -> Self {
        match content {
            ContentBlock::Text(text) => MessageContentBlock::Text(text),
            ContentBlock::Image(image) => MessageContentBlock::Image(image),
            ContentBlock::ResourceLink(_link) => MessageContentBlock::text("[Resource link]"),
            ContentBlock::Resource(resource) => {
                MessageContentBlock::text(extract_text_from_resource(&resource.resource))
            }
            ContentBlock::Audio(_) => MessageContentBlock::text("[Audio content: not supported]"),
            _ => MessageContentBlock::text("[Unsupported content]"),
        }
    }
}

