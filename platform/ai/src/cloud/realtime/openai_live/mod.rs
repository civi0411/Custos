//! OpenAI GPT-Live API.
//!
//! The client owns OpenAI configuration and protocol semantics. WebSocket and
//! WebRTC connectors own their distinct connection establishment flows.

mod client;
mod protocol;
mod session;
mod types;

#[cfg(test)]
mod tests;

pub use client::{
    OpenAiLiveClient, OpenAiLiveExistingSessionConnector, OpenAiLiveWebRtcConnector,
    OpenAiLiveWebRtcNegotiation, OpenAiLiveWebRtcRequest, OpenAiLiveWebSocketConnector,
    OpenAiLiveWebSocketRequest,
};
pub use protocol::OpenAiLiveProtocol;
pub use session::{ConnectedOpenAiLiveSession, StartingOpenAiLiveSession};
pub use types::{
    OpenAiLiveCommand, OpenAiLiveContext, OpenAiLiveContextChannel, OpenAiLiveDelegation,
    OpenAiLiveDelegationId, OpenAiLiveDelegationTarget, OpenAiLiveEvent, OpenAiLiveEventKind,
    OpenAiLiveMessage, OpenAiLiveMessageRole, OpenAiLiveSession, OpenAiLiveSessionConfig,
    OpenAiLiveSessionEvent, OpenAiLiveSessionId, DEFAULT_OPENAI_LIVE_HTTP_URL,
    DEFAULT_OPENAI_LIVE_WEBSOCKET_URL,
};
#[allow(unused_imports)]
pub(crate) use types::OpenAiLiveConnectionKind;
