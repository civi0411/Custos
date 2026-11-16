use std::sync::Arc;
use serde_json::json;
use tokio::sync::broadcast::error::RecvError;
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

use crate::cloud::realtime::browser_live_transport::BrowserLiveTransport;
use crate::cloud::realtime::live::{LiveProtocol, LiveSessionEvent};
use super::super::{
    OpenAiLiveClient, OpenAiLiveCommand, OpenAiLiveConnectionKind, OpenAiLiveEvent,
    OpenAiLiveEventKind, OpenAiLiveMessage, OpenAiLiveMessageRole, OpenAiLiveProtocol,
    OpenAiLiveSessionConfig, OpenAiLiveSessionId, DEFAULT_OPENAI_LIVE_WEBSOCKET_URL,
};

fn config() -> OpenAiLiveSessionConfig {
    OpenAiLiveSessionConfig {
        model: "gpt-live-1".into(),
        instructions: "help".into(),
        voice: None,
        input_messages: vec![],
        extra_session_fields: Default::default(),
    }
}

#[test]
fn existing_session_sideband_is_authenticated_without_primary_bootstrap() {
    let request = OpenAiLiveClient::new("test-key")
        .existing_session(OpenAiLiveSessionId("session/a".into()))
        .request()
        .unwrap();
    assert!(request.endpoint.ends_with("/session%2Fa/attach"));
    assert!(request
        .headers
        .iter()
        .any(|(name, value)| name == "Authorization" && value == "Bearer test-key"));
    assert!(request.initial_messages.is_empty());
}

#[test]
fn primary_websocket_bootstraps_with_session_start_and_pcm_audio() {
    let request = OpenAiLiveClient::new("test")
        .websocket(config())
        .request()
        .unwrap();
    assert_eq!(request.endpoint, DEFAULT_OPENAI_LIVE_WEBSOCKET_URL);
    assert_eq!(request.initial_messages[0]["type"], "session.start");
    assert_eq!(
        request.initial_messages[0]["session"]["audio"]["format"],
        json!({ "type": "audio/pcm", "rate": 24_000 })
    );
}

#[test]
fn initial_message_roles_use_valid_content_types() {
    let roles = [
        (OpenAiLiveMessageRole::Developer, "developer", "input_text"),
        (OpenAiLiveMessageRole::User, "user", "input_text"),
        (OpenAiLiveMessageRole::Assistant, "assistant", "output_text"),
    ];
    for (role, expected_role, expected_content_type) in roles {
        let mut config = config();
        config.input_messages.push(OpenAiLiveMessage {
            role,
            text: "hello".into(),
        });
        let session = OpenAiLiveClient::session_json(&config, false).unwrap();
        assert_eq!(session["input"][0]["role"], expected_role);
        assert_eq!(
            session["input"][0]["content"][0]["type"],
            expected_content_type
        );
    }
}

#[tokio::test]
async fn webrtc_negotiation_uses_ga_request_and_response_shapes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/live/sessions"))
        .and(header("authorization", "Bearer test-key"))
        .and(body_json(json!({
            "session": {
                "model": "gpt-live-1",
                "instructions": "help",
                "input": [],
                "delegation": { "type": "client" }
            },
            "transport": { "type": "webrtc", "sdp": "offer-sdp" }
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session": { "id": "session_1" },
            "transport": { "type": "webrtc", "sdp": "answer-sdp" }
        })))
        .mount(&server)
        .await;

    let negotiation = OpenAiLiveClient::new("test-key")
        .with_http_endpoint(format!("{}/v1/live/sessions", server.uri()))
        .webrtc(config())
        .negotiate("offer-sdp")
        .await
        .unwrap();

    assert_eq!(negotiation.session_id.0, "session_1");
    assert_eq!(negotiation.answer_sdp, "answer-sdp");
}

#[test]
fn session_identity_must_not_be_empty() {
    let error = OpenAiLiveProtocol
        .decode(json!({
            "type": "session.started",
            "event_id": "event_started_1",
            "session": { "id": "  " }
        }))
        .unwrap_err();

    assert!(error.to_string().contains("session.id is empty"));
}

#[tokio::test]
async fn ready_buffers_events_before_session_started() {
    let transport = Arc::new(BrowserLiveTransport::new());
    let connected = OpenAiLiveClient::new("test").connect_transport(
        transport.clone(),
        OpenAiLiveConnectionKind::PrimaryWebSocket,
    );
    transport
        .push_incoming(json!({
            "type": "session.usage.updated",
            "event_id": "event_usage_1",
            "usage": {}
        }))
        .await
        .unwrap();
    transport
        .push_incoming(json!({
            "type": "session.started",
            "event_id": "event_started_1",
            "session": { "id": "session_1" }
        }))
        .await
        .unwrap();

    let mut connected = connected.ready(None).await.unwrap();
    assert_eq!(connected.session_id().0, "session_1");
    assert!(matches!(
        connected.recv().await.unwrap(),
        LiveSessionEvent::Message(OpenAiLiveEvent {
            kind: OpenAiLiveEventKind::Usage { .. },
            ..
        })
    ));
}

#[tokio::test]
async fn ready_rejects_a_different_creation_identity() {
    let transport = Arc::new(BrowserLiveTransport::new());
    let starting = OpenAiLiveClient::new("test")
        .connect_transport(transport.clone(), OpenAiLiveConnectionKind::MediaControl);
    transport
        .push_incoming(json!({
            "type": "session.started",
            "event_id": "event_started_1",
            "session": { "id": "session_2" }
        }))
        .await
        .unwrap();

    let Err(error) = starting
        .ready(Some(OpenAiLiveSessionId("session_1".into())))
        .await
    else {
        panic!("expected a mismatched session identity");
    };

    assert!(error.to_string().contains("creation identity"));
}

#[tokio::test]
async fn authoritative_receiver_reports_lag() {
    let transport = Arc::new(BrowserLiveTransport::new());
    let mut connected = OpenAiLiveClient::new("test")
        .connect_transport(
            transport.clone(),
            OpenAiLiveConnectionKind::PrimaryWebSocket,
        )
        .connected(OpenAiLiveSessionId("session_1".into()));

    for sequence in 0..600 {
        transport
            .push_incoming(json!({ "type": "future.event", "sequence": sequence }))
            .await
            .unwrap();
    }

    assert!(matches!(connected.recv().await, Err(RecvError::Lagged(_))));
}

#[tokio::test]
async fn media_control_connection_rejects_primary_audio_commands() {
    let transport = Arc::new(BrowserLiveTransport::new());
    let connected = OpenAiLiveClient::new("test")
        .connect_transport(transport, OpenAiLiveConnectionKind::MediaControl)
        .connected(OpenAiLiveSessionId("session_1".into()));

    let error = connected
        .send(OpenAiLiveCommand::AppendAudio(vec![1]))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("only permitted"));
}
