use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rmcp::model::Role;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::cloud::realtime::live::LiveProtocol;
use super::types::{
    OpenAiLiveCommand, OpenAiLiveContext, OpenAiLiveContextChannel, OpenAiLiveDelegation,
    OpenAiLiveDelegationId, OpenAiLiveDelegationTarget, OpenAiLiveEvent, OpenAiLiveEventKind,
    OpenAiLiveSessionId,
};

pub struct OpenAiLiveProtocol;

impl LiveProtocol for OpenAiLiveProtocol {
    type Command = OpenAiLiveCommand;
    type Event = OpenAiLiveEvent;

    fn encode(&self, command: Self::Command) -> Result<Value> {
        match command {
            OpenAiLiveCommand::AppendContext {
                event_id,
                delegation_id,
                context,
            } => text_event(event_id, delegation_id, context),
            OpenAiLiveCommand::AppendAudio(audio) => {
                if audio.is_empty() {
                    bail!("audio payload is empty");
                }
                Ok(json!({
                    "type": "session.input_audio.append",
                    "event_id": event_id(),
                    "audio": BASE64.encode(audio),
                }))
            }
            OpenAiLiveCommand::MuteInput { event_id } => {
                Ok(json!({ "type": "session.input_audio.mute", "event_id": event_id }))
            }
            OpenAiLiveCommand::UnmuteInput { event_id } => {
                Ok(json!({ "type": "session.input_audio.unmute", "event_id": event_id }))
            }
            OpenAiLiveCommand::Close => {
                Ok(json!({ "type": "session.close", "event_id": event_id() }))
            }
        }
    }

    fn decode(&self, event: Value) -> Result<Self::Event> {
        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .context("OpenAI Live event is missing type")?;
        let kind = match event_type {
            "session.started" => OpenAiLiveEventKind::SessionStarted {
                event_id: required_string(event.get("event_id"), "event_id")?,
                session_id: OpenAiLiveSessionId(required_nonempty_string(
                    event.pointer("/session/id"),
                    "session.id",
                )?),
            },
            "session.input_transcript.delta" => transcript_delta(&event, Role::User)?,
            "session.output_transcript.delta" => transcript_delta(&event, Role::Assistant)?,
            "session.output_audio.delta" => OpenAiLiveEventKind::OutputAudioDelta {
                audio: BASE64.decode(
                    event
                        .get("delta")
                        .and_then(Value::as_str)
                        .context("session.output_audio.delta is missing delta")?,
                )?,
                start_ms: optional_u64(event.get("start_ms"), "start_ms")?,
                end_ms: optional_u64(event.get("end_ms"), "end_ms")?,
            },
            "session.delegation.created" => {
                let delegation = event
                    .get("delegation")
                    .context("session.delegation.created is missing delegation")?;
                OpenAiLiveEventKind::DelegationCreated {
                    event_id: required_string(event.get("event_id"), "event_id")?,
                    client_event_id: optional_string(
                        event.get("client_event_id"),
                        "client_event_id",
                    )?,
                    delegation: OpenAiLiveDelegation {
                        id: delegation
                            .get("id")
                            .and_then(Value::as_str)
                            .context("delegation is missing id")?
                            .to_owned()
                            .into(),
                        target: delegation_target(delegation)?,
                        offset_ms: required_u64(event.get("offset_ms"), "offset_ms")?,
                    },
                }
            }
            "session.instructions.appended" => {
                context_appended(&event, OpenAiLiveContextChannel::Instructions)?
            }
            "session.thinking.appended" => {
                context_appended(&event, OpenAiLiveContextChannel::Thinking)?
            }
            "session.commentary.appended" => {
                context_appended(&event, OpenAiLiveContextChannel::Commentary)?
            }
            "session.usage.updated" => OpenAiLiveEventKind::Usage {
                event_id: required_string(event.get("event_id"), "event_id")?,
                usage: required_object(event.get("usage"), "usage")?,
            },
            "session.input_audio.muted" => OpenAiLiveEventKind::InputMuted {
                event_id: required_string(event.get("event_id"), "event_id")?,
                client_event_id: optional_string(event.get("client_event_id"), "client_event_id")?,
            },
            "session.input_audio.unmuted" => OpenAiLiveEventKind::InputUnmuted {
                event_id: required_string(event.get("event_id"), "event_id")?,
                client_event_id: optional_string(event.get("client_event_id"), "client_event_id")?,
            },
            "session.closed" => OpenAiLiveEventKind::SessionClosed {
                event_id: required_string(event.get("event_id"), "event_id")?,
                client_event_id: optional_string(event.get("client_event_id"), "client_event_id")?,
                reason: required_string(event.get("reason"), "reason")?,
                session: required_object(event.get("session"), "session")?,
                usage: required_object(event.get("usage"), "usage")?,
            },
            "error" => live_error(&event, "/error")?,
            "info" => OpenAiLiveEventKind::Info {
                event_id: required_string(event.get("event_id"), "event_id")?,
                code: required_string(event.get("code"), "code")?,
                message: required_string(event.get("message"), "message")?,
                client_event_id: optional_string(event.get("client_event_id"), "client_event_id")?,
            },
            _ => OpenAiLiveEventKind::Other {
                event_type: event_type.into(),
            },
        };
        Ok(OpenAiLiveEvent {
            kind,
            raw: Some(event),
        })
    }

    fn close_command(&self) -> Option<Self::Command> {
        Some(OpenAiLiveCommand::Close)
    }

    fn is_close_acknowledgement(&self, event: &Self::Event) -> bool {
        matches!(event.kind, OpenAiLiveEventKind::SessionClosed { .. })
    }
}

pub(crate) fn transcript_delta(event: &Value, role: Role) -> Result<OpenAiLiveEventKind> {
    Ok(OpenAiLiveEventKind::TranscriptDelta {
        event_id: required_string(event.get("event_id"), "event_id")?,
        client_event_id: optional_string(event.get("client_event_id"), "client_event_id")?,
        role,
        delta: required_string(event.get("delta"), "delta")?,
        start_ms: required_u64(event.get("start_ms"), "start_ms")?,
        end_ms: required_u64(event.get("end_ms"), "end_ms")?,
    })
}

pub(crate) fn context_appended(
    event: &Value,
    channel: OpenAiLiveContextChannel,
) -> Result<OpenAiLiveEventKind> {
    Ok(OpenAiLiveEventKind::ContextAppended {
        event_id: required_string(event.get("event_id"), "event_id")?,
        channel,
        client_event_id: optional_string(event.get("client_event_id"), "client_event_id")?,
        start_ms: required_u64(event.get("start_ms"), "start_ms")?,
        end_ms: required_u64(event.get("end_ms"), "end_ms")?,
    })
}

pub(crate) fn delegation_target(item: &Value) -> Result<OpenAiLiveDelegationTarget> {
    match item
        .get("target")
        .and_then(Value::as_str)
        .context("delegation is missing target")?
    {
        "client" => Ok(OpenAiLiveDelegationTarget::Client),
        "responses" => Ok(OpenAiLiveDelegationTarget::Responses),
        target => Ok(OpenAiLiveDelegationTarget::Other(target.to_owned())),
    }
}

pub(crate) fn required_object(value: Option<&Value>, field: &str) -> Result<Value> {
    match value {
        Some(value) if value.is_object() => Ok(value.clone()),
        Some(_) => bail!("OpenAI Live event field {field} is not an object"),
        None => bail!("OpenAI Live event is missing {field}"),
    }
}

pub(crate) fn live_error(event: &Value, pointer: &str) -> Result<OpenAiLiveEventKind> {
    let error = event
        .pointer(pointer)
        .with_context(|| format!("OpenAI Live error is missing {pointer}"))?;
    Ok(OpenAiLiveEventKind::Error {
        event_id: required_string(event.get("event_id"), "event_id")?,
        error_type: required_string(error.get("type"), "error.type")?,
        code: required_string(error.get("code"), "error.code")?,
        message: required_string(error.get("message"), "error.message")?,
        parameter: optional_string(error.get("param"), "error.param")?,
        client_event_id: optional_string(error.get("client_event_id"), "error.client_event_id")?,
    })
}

pub(crate) fn required_string(value: Option<&Value>, field: &str) -> Result<String> {
    value
        .and_then(Value::as_str)
        .with_context(|| format!("OpenAI Live event is missing {field}"))
        .map(str::to_owned)
}

pub(crate) fn required_nonempty_string(value: Option<&Value>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    if value.trim().is_empty() {
        bail!("OpenAI Live field {field} is empty");
    }
    Ok(value)
}

pub(crate) fn optional_string(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.to_owned())),
        Some(_) => bail!("OpenAI Live event field {field} is not a string"),
    }
}

pub(crate) fn required_u64(value: Option<&Value>, field: &str) -> Result<u64> {
    value
        .and_then(Value::as_u64)
        .with_context(|| format!("OpenAI Live event is missing {field}"))
}

pub(crate) fn optional_u64(value: Option<&Value>, field: &str) -> Result<Option<u64>> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .with_context(|| format!("OpenAI Live event field {field} is not an unsigned integer"))
            .map(Some),
    }
}

pub(crate) fn event_id() -> String {
    format!("event_{}", Uuid::new_v4())
}

pub(crate) fn text_event(
    event_id: String,
    delegation_id: Option<OpenAiLiveDelegationId>,
    context: OpenAiLiveContext,
) -> Result<Value> {
    let kind = match context.channel {
        OpenAiLiveContextChannel::Instructions => "session.instructions.append",
        OpenAiLiveContextChannel::Thinking => "session.thinking.append",
        OpenAiLiveContextChannel::Commentary => "session.commentary.append",
    };
    if event_id.trim().is_empty() {
        bail!("event ID is empty");
    }
    if delegation_id
        .as_ref()
        .is_some_and(|delegation_id| delegation_id.0.trim().is_empty())
    {
        bail!("delegation ID is empty");
    }
    Ok(json!({
        "type": kind,
        "event_id": event_id,
        "delegation_id": delegation_id.map(|delegation_id| delegation_id.0),
        "content": context.text,
    }))
}
