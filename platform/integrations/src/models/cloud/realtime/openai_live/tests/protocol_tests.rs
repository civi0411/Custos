use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rmcp::model::Role;
use serde_json::{json, Value};

use crate::cloud::realtime::live::LiveProtocol;
use super::super::{
    OpenAiLiveCommand, OpenAiLiveContext, OpenAiLiveContextChannel, OpenAiLiveDelegationId,
    OpenAiLiveDelegationTarget, OpenAiLiveEventKind, OpenAiLiveProtocol,
};

fn decode(event: Value) -> OpenAiLiveEventKind {
    OpenAiLiveProtocol.decode(event).unwrap().kind
}

#[test]
fn transcript_deltas_keep_role_and_timing() {
    let output = json!({
        "type": "session.output_transcript.delta",
        "event_id": "event_output_1",
        "delta": " It's five",
        "start_ms": 3200,
        "end_ms": 3400
    });
    let OpenAiLiveEventKind::TranscriptDelta {
        role,
        delta,
        start_ms,
        end_ms,
        ..
    } = decode(output)
    else {
        panic!("expected transcript delta");
    };
    assert_eq!(role, Role::Assistant);
    assert_eq!(delta, " It's five");
    assert_eq!((start_ms, end_ms), (3200, 3400));

    let input = json!({
        "type": "session.input_transcript.delta",
        "event_id": "event_input_1",
        "delta": "What",
        "start_ms": 400,
        "end_ms": 600
    });
    assert!(
        matches!(decode(input), OpenAiLiveEventKind::TranscriptDelta { role, .. }
        if role == Role::User)
    );
}

#[test]
fn delegation_keeps_identity_target_and_offset() {
    let delegation = json!({
        "type": "session.delegation.created",
        "event_id": "event_delegation_1",
        "offset_ms": 800,
        "delegation": { "id": "delegation_1", "type": "delegation", "target": "client" }
    });
    let OpenAiLiveEventKind::DelegationCreated { delegation, .. } = decode(delegation) else {
        panic!("expected delegation");
    };
    assert_eq!(delegation.id.0, "delegation_1");
    assert_eq!(delegation.target, OpenAiLiveDelegationTarget::Client);
    assert_eq!(delegation.offset_ms, 800);

    let unsupported = json!({
        "type": "session.delegation.created",
        "event_id": "event_delegation_2",
        "offset_ms": 900,
        "delegation": { "id": "delegation_2", "target": "future_target" }
    });
    let OpenAiLiveEventKind::DelegationCreated { delegation, .. } = decode(unsupported) else {
        panic!("expected delegation");
    };
    assert!(
        matches!(delegation.target, OpenAiLiveDelegationTarget::Other(target)
        if target == "future_target")
    );
}

#[test]
fn context_acknowledgements_and_errors_preserve_correlation() {
    let accepted = json!({ "type": "session.commentary.appended", "event_id": "event_accepted_1",
        "client_event_id": "client_event_1", "start_ms": 800, "end_ms": 1200 });
    let rejected = json!({ "type": "error", "event_id": "event_error_1", "error": { "type": "invalid_request_error",
        "code": "empty_array", "message": "content cannot be empty", "param": "content",
        "client_event_id": "client_event_2" } });

    let OpenAiLiveEventKind::ContextAppended {
        channel,
        client_event_id,
        start_ms,
        end_ms,
        ..
    } = decode(accepted)
    else {
        panic!("expected context acknowledgement");
    };
    assert!(matches!(channel, OpenAiLiveContextChannel::Commentary));
    assert_eq!(client_event_id.as_deref(), Some("client_event_1"));
    assert_eq!(start_ms, 800);
    assert_eq!(end_ms, 1200);

    let OpenAiLiveEventKind::Error {
        error_type,
        code,
        message,
        parameter,
        client_event_id,
        ..
    } = decode(rejected)
    else {
        panic!("expected error");
    };
    assert_eq!(error_type, "invalid_request_error");
    assert_eq!(code, "empty_array");
    assert_eq!(message, "content cannot be empty");
    assert_eq!(parameter.as_deref(), Some("content"));
    assert_eq!(client_event_id.as_deref(), Some("client_event_2"));
}

#[test]
fn close_mute_and_unknown_events_preserve_semantics() {
    let closed = json!({ "type": "session.closed", "event_id": "event_closed_1",
        "client_event_id": "client_close_1", "reason": "close_requested",
        "session": { "id": "session_1" }, "usage": { "seconds": 8 } });

    let OpenAiLiveEventKind::SessionClosed { reason, usage, .. } = decode(closed) else {
        panic!("expected session close");
    };
    assert_eq!(reason, "close_requested");
    assert_eq!(usage["seconds"], 8);
    assert!(usage.get("total_tokens").is_none());
    assert!(matches!(
        decode(json!({ "type": "session.input_audio.muted", "event_id": "event_muted_1" })),
        OpenAiLiveEventKind::InputMuted {
            client_event_id: None,
            ..
        }
    ));
    assert!(matches!(
        decode(json!({ "type": "session.input_audio.unmuted", "event_id": "event_unmuted_1" })),
        OpenAiLiveEventKind::InputUnmuted {
            client_event_id: None,
            ..
        }
    ));
    assert!(matches!(
        decode(json!({ "type": "future.event" })),
        OpenAiLiveEventKind::Other { event_type } if event_type == "future.event"
    ));
    assert_eq!(
        OpenAiLiveProtocol.encode(OpenAiLiveCommand::Close).unwrap()["type"],
        "session.close"
    );
}

#[test]
fn malformed_supported_event_is_rejected() {
    let error = OpenAiLiveProtocol
        .decode(json!({
            "type": "session.input_transcript.delta",
            "event_id": "event_transcript_1",
            "start_ms": 0,
            "end_ms": 100
        }))
        .unwrap_err();
    assert!(error.to_string().contains("delta"));
}

#[test]
fn output_audio_preserves_sideband_timing() {
    let event = json!({
        "type": "session.output_audio.delta",
        "delta": BASE64.encode([0_u8, 1, 2]),
        "start_ms": 100,
        "end_ms": 200
    });
    let OpenAiLiveEventKind::OutputAudioDelta {
        audio,
        start_ms,
        end_ms,
    } = decode(event)
    else {
        panic!("expected output audio");
    };
    assert_eq!(audio, vec![0, 1, 2]);
    assert_eq!(start_ms, Some(100));
    assert_eq!(end_ms, Some(200));
}

#[test]
fn encodes_context_with_optional_delegation() {
    let cases = [
        (
            OpenAiLiveContextChannel::Commentary,
            Some(OpenAiLiveDelegationId("delegation_1".into())),
            "session.commentary.append",
            json!("delegation_1"),
        ),
        (
            OpenAiLiveContextChannel::Instructions,
            None,
            "session.instructions.append",
            Value::Null,
        ),
    ];

    for (channel, delegation_id, expected_type, expected_delegation_id) in cases {
        let encoded = OpenAiLiveProtocol
            .encode(OpenAiLiveCommand::AppendContext {
                event_id: "client_event_1".into(),
                delegation_id,
                context: OpenAiLiveContext {
                    text: "context".into(),
                    channel,
                },
            })
            .unwrap();
        assert_eq!(encoded["type"], expected_type);
        assert_eq!(encoded["delegation_id"], expected_delegation_id);
        assert_eq!(encoded["event_id"], "client_event_1");
        assert_eq!(encoded["content"], "context");
    }
}
