//! Core streaming wire types and helpers.

use custos_provider::{ProviderEvent, ProviderEventType};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    OpenAi,
    Claude,
    Codex,
    Ollama,
    Google,
}

pub(crate) fn count(v: &Value, path: &str) -> Option<usize> {
    v.pointer(path)
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
}

pub(crate) fn event(id: &str, kind: ProviderEventType, text: Option<String>) -> ProviderEvent {
    let mut event = ProviderEvent::chunk(id, text.unwrap_or_default());
    event.event_type = kind;
    event
}

pub(crate) fn actual_model(wire: Wire, value: &Value) -> Option<String> {
    let model = match wire {
        Wire::OpenAi | Wire::Ollama => value.get("model"),
        Wire::Claude => value
            .pointer("/message/model")
            .or_else(|| value.get("model")),
        Wire::Codex => value
            .pointer("/response/model")
            .or_else(|| value.get("model")),
        Wire::Google => None,
    }
    .and_then(Value::as_str)?;
    Some(if matches!(wire, Wire::Ollama) {
        format!("ollama/{model}")
    } else {
        model.to_string()
    })
}
