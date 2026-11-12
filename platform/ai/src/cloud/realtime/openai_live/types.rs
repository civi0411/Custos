use std::collections::BTreeMap;
use std::time::Duration;
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cloud::realtime::live::{LiveSession, LiveSessionEvent};
use super::protocol::OpenAiLiveProtocol;

pub const DEFAULT_OPENAI_LIVE_HTTP_URL: &str = "https://api.openai.com/v1/live/sessions";
pub const DEFAULT_OPENAI_LIVE_WEBSOCKET_URL: &str = "wss://api.openai.com/v1/live/sessions";
pub(crate) const SESSION_START_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiLiveSessionConfig {
    pub model: String,
    pub instructions: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(default)]
    pub input_messages: Vec<OpenAiLiveMessage>,
    #[serde(default)]
    pub extra_session_fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiLiveMessage {
    pub role: OpenAiLiveMessageRole,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAiLiveMessageRole {
    User,
    Assistant,
    Developer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAiLiveContextChannel {
    Instructions,
    Thinking,
    Commentary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiLiveContext {
    pub text: String,
    pub channel: OpenAiLiveContextChannel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpenAiLiveSessionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpenAiLiveDelegationId(pub String);

impl From<String> for OpenAiLiveDelegationId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for OpenAiLiveDelegationId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAiLiveDelegationTarget {
    Client,
    Responses,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiLiveDelegation {
    pub id: OpenAiLiveDelegationId,
    pub target: OpenAiLiveDelegationTarget,
    pub offset_ms: u64,
}

pub enum OpenAiLiveCommand {
    AppendContext {
        event_id: String,
        delegation_id: Option<OpenAiLiveDelegationId>,
        context: OpenAiLiveContext,
    },
    AppendAudio(Vec<u8>),
    MuteInput {
        event_id: String,
    },
    UnmuteInput {
        event_id: String,
    },
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiLiveEvent {
    pub kind: OpenAiLiveEventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OpenAiLiveEventKind {
    SessionStarted {
        event_id: String,
        session_id: OpenAiLiveSessionId,
    },
    TranscriptDelta {
        event_id: String,
        client_event_id: Option<String>,
        role: Role,
        delta: String,
        start_ms: u64,
        end_ms: u64,
    },
    OutputAudioDelta {
        audio: Vec<u8>,
        start_ms: Option<u64>,
        end_ms: Option<u64>,
    },
    DelegationCreated {
        event_id: String,
        client_event_id: Option<String>,
        delegation: OpenAiLiveDelegation,
    },
    ContextAppended {
        event_id: String,
        channel: OpenAiLiveContextChannel,
        client_event_id: Option<String>,
        start_ms: u64,
        end_ms: u64,
    },
    Usage {
        event_id: String,
        usage: Value,
    },
    Error {
        event_id: String,
        error_type: String,
        code: String,
        message: String,
        parameter: Option<String>,
        client_event_id: Option<String>,
    },
    Info {
        event_id: String,
        code: String,
        message: String,
        client_event_id: Option<String>,
    },
    InputMuted {
        event_id: String,
        client_event_id: Option<String>,
    },
    InputUnmuted {
        event_id: String,
        client_event_id: Option<String>,
    },
    SessionClosed {
        event_id: String,
        client_event_id: Option<String>,
        reason: String,
        session: Value,
        usage: Value,
    },
    Other {
        event_type: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OpenAiLiveConnectionKind {
    PrimaryWebSocket,
    MediaControl,
}

pub type OpenAiLiveSession = LiveSession<OpenAiLiveProtocol>;
pub type OpenAiLiveSessionEvent = LiveSessionEvent<OpenAiLiveEvent>;
