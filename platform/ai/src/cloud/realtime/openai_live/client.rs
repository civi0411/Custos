use std::sync::Arc;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use tokio::time::timeout;

use crate::cloud::{
    realtime::live::{LiveSession, LiveTransport},
    transport::http_status::handle_response,
};
use super::protocol::{event_id, required_nonempty_string, OpenAiLiveProtocol};
use super::session::{ConnectedOpenAiLiveSession, StartingOpenAiLiveSession};
use super::types::{
    OpenAiLiveConnectionKind, OpenAiLiveMessageRole, OpenAiLiveSessionConfig,
    OpenAiLiveSessionId, DEFAULT_OPENAI_LIVE_HTTP_URL, DEFAULT_OPENAI_LIVE_WEBSOCKET_URL,
    SESSION_START_TIMEOUT,
};

#[derive(Clone)]
pub struct OpenAiLiveClient {
    api_key: String,
    http_endpoint: String,
    websocket_endpoint: String,
}

impl OpenAiLiveClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            http_endpoint: DEFAULT_OPENAI_LIVE_HTTP_URL.into(),
            websocket_endpoint: DEFAULT_OPENAI_LIVE_WEBSOCKET_URL.into(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY is not set")?;
        if key.trim().is_empty() {
            bail!("OPENAI_API_KEY is empty");
        }
        Ok(Self::new(key))
    }

    pub fn with_http_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.http_endpoint = endpoint.into();
        self
    }

    pub fn with_websocket_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.websocket_endpoint = endpoint.into();
        self
    }

    pub fn websocket(&self, config: OpenAiLiveSessionConfig) -> OpenAiLiveWebSocketConnector {
        OpenAiLiveWebSocketConnector {
            client: self.clone(),
            config,
        }
    }

    pub fn existing_session(
        &self,
        session_id: OpenAiLiveSessionId,
    ) -> OpenAiLiveExistingSessionConnector {
        OpenAiLiveExistingSessionConnector {
            client: self.clone(),
            session_id,
        }
    }

    pub fn webrtc(&self, config: OpenAiLiveSessionConfig) -> OpenAiLiveWebRtcConnector {
        OpenAiLiveWebRtcConnector {
            client: self.clone(),
            config,
        }
    }

    pub(crate) fn connect_transport(
        &self,
        transport: Arc<dyn LiveTransport>,
        connection_kind: OpenAiLiveConnectionKind,
    ) -> StartingOpenAiLiveSession {
        let (session, events) = LiveSession::connect(Arc::new(OpenAiLiveProtocol), transport);
        StartingOpenAiLiveSession {
            session,
            events,
            pending_events: Default::default(),
            connection_kind,
        }
    }

    pub(crate) fn headers(&self) -> Vec<(String, String)> {
        vec![("Authorization".into(), format!("Bearer {}", self.api_key))]
    }

    pub(crate) fn session_json(
        config: &OpenAiLiveSessionConfig,
        websocket_audio: bool,
    ) -> Result<Value> {
        const RESERVED: &[&str] = &["model", "instructions", "input", "audio", "delegation"];
        if let Some(field) = config
            .extra_session_fields
            .keys()
            .find(|field| RESERVED.contains(&field.as_str()))
        {
            bail!("extra session field `{field}` conflicts with typed configuration");
        }
        let input_messages = config
            .input_messages
            .iter()
            .map(|message| {
                let role = match message.role {
                    OpenAiLiveMessageRole::User => "user",
                    OpenAiLiveMessageRole::Assistant => "assistant",
                    OpenAiLiveMessageRole::Developer => "developer",
                };
                let content_type = if matches!(message.role, OpenAiLiveMessageRole::Assistant) {
                    "output_text"
                } else {
                    "input_text"
                };
                json!({ "type": "message", "role": role, "content": [{ "type": content_type, "text": message.text }] })
            })
            .collect::<Vec<_>>();
        let mut session = json!({
            "model": config.model,
            "instructions": config.instructions,
            "input": input_messages,
            "delegation": { "type": "client" },
        });
        if websocket_audio || config.voice.is_some() {
            let mut audio = json!({});
            if websocket_audio {
                audio["format"] = json!({ "type": "audio/pcm", "rate": 24_000 });
            }
            if let Some(voice) = &config.voice {
                audio["output"] = json!({ "voice": voice });
            }
            session["audio"] = audio;
        }
        session
            .as_object_mut()
            .unwrap()
            .extend(config.extra_session_fields.clone());
        Ok(session)
    }
}

pub struct OpenAiLiveExistingSessionConnector {
    client: OpenAiLiveClient,
    session_id: OpenAiLiveSessionId,
}

impl OpenAiLiveExistingSessionConnector {
    pub fn request(self) -> Result<OpenAiLiveWebSocketRequest> {
        if self.session_id.0.trim().is_empty() {
            bail!("OpenAI Live session ID is empty");
        }
        Ok(OpenAiLiveWebSocketRequest {
            endpoint: format!(
                "{}/{}/attach",
                self.client.websocket_endpoint.trim_end_matches('/'),
                urlencoding::encode(&self.session_id.0)
            ),
            headers: self.client.headers(),
            initial_messages: Vec::new(),
        })
    }

    #[cfg(feature = "live-websocket")]
    pub async fn connect(self) -> Result<ConnectedOpenAiLiveSession> {
        let client = self.client.clone();
        let session_id = self.session_id.clone();
        let transport = Arc::new(
            crate::cloud::realtime::live_transport_websocket::WebSocketLiveTransport::connect(self.request()?)
                .await?,
        );
        Ok(client
            .connect_transport(transport, OpenAiLiveConnectionKind::MediaControl)
            .connected(session_id))
    }
}

pub struct OpenAiLiveWebSocketRequest {
    pub endpoint: String,
    pub headers: Vec<(String, String)>,
    pub initial_messages: Vec<Value>,
}

pub struct OpenAiLiveWebSocketConnector {
    client: OpenAiLiveClient,
    config: OpenAiLiveSessionConfig,
}

impl OpenAiLiveWebSocketConnector {
    pub fn request(self) -> Result<OpenAiLiveWebSocketRequest> {
        Ok(OpenAiLiveWebSocketRequest {
            endpoint: self.client.websocket_endpoint.clone(),
            headers: self.client.headers(),
            initial_messages: vec![json!({
                "type": "session.start",
                "event_id": event_id(),
                "session": OpenAiLiveClient::session_json(&self.config, true)?,
            })],
        })
    }

    #[cfg(feature = "live-websocket")]
    pub async fn connect(self) -> Result<ConnectedOpenAiLiveSession> {
        let client = self.client.clone();
        let transport = Arc::new(
            crate::cloud::realtime::live_transport_websocket::WebSocketLiveTransport::connect(self.request()?)
                .await?,
        );
        client
            .connect_transport(transport, OpenAiLiveConnectionKind::PrimaryWebSocket)
            .ready(None)
            .await
    }
}

pub struct OpenAiLiveWebRtcConnector {
    client: OpenAiLiveClient,
    config: OpenAiLiveSessionConfig,
}

pub struct OpenAiLiveWebRtcRequest {
    pub endpoint: String,
    pub headers: Vec<(String, String)>,
    pub offer_sdp: String,
    pub session: Value,
}

pub struct OpenAiLiveWebRtcNegotiation {
    pub answer_sdp: String,
    pub session_id: OpenAiLiveSessionId,
    client: OpenAiLiveClient,
}

impl OpenAiLiveWebRtcNegotiation {
    pub async fn bind(
        self,
        transport: Arc<dyn LiveTransport>,
    ) -> Result<ConnectedOpenAiLiveSession> {
        self.client
            .connect_transport(transport, OpenAiLiveConnectionKind::MediaControl)
            .ready(Some(self.session_id))
            .await
    }
}

impl OpenAiLiveWebRtcConnector {
    pub fn request(self, offer_sdp: impl Into<String>) -> Result<OpenAiLiveWebRtcRequest> {
        let offer_sdp = offer_sdp.into();
        if offer_sdp.trim().is_empty() {
            bail!("WebRTC SDP offer is empty");
        }
        Ok(OpenAiLiveWebRtcRequest {
            endpoint: self.client.http_endpoint.clone(),
            headers: self.client.headers(),
            offer_sdp,
            session: OpenAiLiveClient::session_json(&self.config, false)?,
        })
    }

    pub async fn negotiate(
        self,
        offer_sdp: impl Into<String>,
    ) -> Result<OpenAiLiveWebRtcNegotiation> {
        let client = self.client.clone();
        let request = self.request(offer_sdp)?;
        let mut http_request = reqwest::Client::new().post(request.endpoint).json(&json!({
            "session": request.session,
            "transport": { "type": "webrtc", "sdp": request.offer_sdp },
        }));
        for (name, value) in request.headers {
            http_request = http_request.header(name, value);
        }
        let result: Value = timeout(SESSION_START_TIMEOUT, async {
            let response = http_request.send().await?;
            handle_response(response).await
        })
        .await
        .context("OpenAI Live signaling timed out")??;
        let session_id = OpenAiLiveSessionId(required_nonempty_string(
            result.pointer("/session/id"),
            "session.id",
        )?);
        let answer_sdp =
            required_nonempty_string(result.pointer("/transport/sdp"), "transport.sdp")?;
        Ok(OpenAiLiveWebRtcNegotiation {
            answer_sdp,
            session_id,
            client,
        })
    }
}
