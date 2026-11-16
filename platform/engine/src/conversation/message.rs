//! Clean Zero-I/O Conversation Message model

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MessageMetadata {
    pub agent_visible: bool,
}

impl MessageMetadata {
    pub fn user_only() -> Self {
        Self { agent_visible: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolCallData {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolRequestData {
    pub id: String,
    pub tool_call: Result<ToolCallData, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MessageContentBlock {
    Text(String),
    ToolRequest(ToolRequestData),
    ToolResponse(String),
}

impl MessageContentBlock {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_tool_request(&self) -> Option<&ToolRequestData> {
        match self {
            Self::ToolRequest(r) => Some(r),
            _ => None,
        }
    }

    pub fn as_tool_response_text(&self) -> Option<String> {
        match self {
            Self::ToolResponse(s) => Some(s.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: Option<String>,
    pub content: Vec<MessageContentBlock>,
    pub metadata: MessageMetadata,
}

impl Message {
    pub fn new() -> Self {
        Self {
            id: None,
            content: Vec::new(),
            metadata: MessageMetadata { agent_visible: true },
        }
    }

    pub fn user() -> Self {
        Self::new()
    }

    pub fn assistant() -> Self {
        Self::new()
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.content.push(MessageContentBlock::Text(text.into()));
        self
    }

    pub fn user_only(mut self) -> Self {
        self.metadata.agent_visible = false;
        self
    }
}
