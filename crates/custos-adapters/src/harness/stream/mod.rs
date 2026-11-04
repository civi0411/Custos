//! Modular streaming, normalization, and sub-process supervision for AI coding harnesses.

pub mod claude;
pub mod codex;
pub mod intents;
pub mod opencode;
pub mod plain;
pub mod process;
pub mod usage;

pub use intents::extract_observed_intents;
pub use process::{run_observed_process, run_observed_process_with_protocol};
pub use usage::emit_usage;

use super::spec::StreamProtocol;
use custos_core::contracts::harness::RunObserver;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Dispatches raw event payload to protocol-specific parser and emits normalized deltas.
pub fn normalize(
    value: &Value,
    protocol: StreamProtocol,
    observer: &Arc<dyn RunObserver>,
    parts: &mut HashMap<String, String>,
    answer: &mut String,
) {
    let text = match protocol {
        StreamProtocol::ClaudeJson => claude::normalize_claude(value, observer, answer),
        StreamProtocol::CodexJson => codex::normalize_codex(value, observer, parts, answer),
        StreamProtocol::OpenCodeJson => opencode::normalize_opencode(value, observer),
        StreamProtocol::PlainText => None,
    };

    if let Some(text) = text.filter(|text| !text.is_empty()) {
        if answer.len() + text.len() <= 2 * 1024 * 1024 {
            answer.push_str(&text);
            observer.emit("status", serde_json::json!({"status": "running"}));
            observer.emit("text_delta", serde_json::json!({"text": text}));
        }
    }
}

pub fn normalize_by_harness(
    value: &Value,
    harness: &str,
    observer: &Arc<dyn RunObserver>,
    parts: &mut HashMap<String, String>,
    answer: &mut String,
) {
    let protocol = match harness {
        "claude-code" => StreamProtocol::ClaudeJson,
        "codex" => StreamProtocol::CodexJson,
        "opencode" => StreamProtocol::OpenCodeJson,
        _ => StreamProtocol::PlainText,
    };
    normalize(value, protocol, observer, parts, answer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Observer {
        events: std::sync::Mutex<Vec<(String, Value)>>,
        cancel: std::sync::atomic::AtomicBool,
    }
    impl RunObserver for Observer {
        fn emit(&self, kind: &str, value: Value) {
            self.events.lock().unwrap().push((kind.into(), value));
        }
        fn is_cancelled(&self) -> bool {
            self.cancel.load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    #[test]
    fn native_messages_tools_and_unknown_logs() {
        let captured = Arc::new(Observer::default());
        let observer: Arc<dyn RunObserver> = captured.clone();
        let mut parts = HashMap::new();
        let mut answer = String::new();
        for value in [
            serde_json::json!({"type":"item.updated","item":{"type":"agent_message","id":"a","text":"Hello"}}),
            serde_json::json!({"type":"item.completed","item":{"type":"agent_message","id":"a","text":"Hello world"}}),
            serde_json::json!({"type":"item.completed","item":{"type":"command_execution","id":"tool","command":"echo test","aggregated_output":"test"}}),
        ] {
            normalize(&value, StreamProtocol::CodexJson, &observer, &mut parts, &mut answer);
        }
        assert_eq!(answer, "Hello world");
        assert!(captured
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "tool_output"));
        let count = captured.events.lock().unwrap().len();
        normalize(
            &serde_json::json!({"type":"log","text":"thinking"}),
            StreamProtocol::CodexJson,
            &observer,
            &mut parts,
            &mut answer,
        );
        assert_eq!(captured.events.lock().unwrap().len(), count);
        let intents = extract_observed_intents(
            r#"{"type":"item.completed","item":{"type":"command_execution","id":"tool","command":"echo test"}}"#,
            "codex",
            "test_task",
        );
        assert_eq!(intents.len(), 1);
        emit_usage(
            &observer,
            serde_json::json!({"input_tokens":2,"output_tokens":1,"access_token":"private-value","reasoning":"private chain"}),
        );
        let events = captured.events.lock().unwrap();
        let usage = &events.last().unwrap().1;
        assert_eq!(usage["total_tokens"], 3);
        assert!(usage.get("access_token").is_none());
        assert!(usage.get("reasoning").is_none());
    }

    #[test]
    fn test_claude_and_codex_event_shape_parity() {
        let codex_captured = Arc::new(Observer::default());
        let codex_observer: Arc<dyn RunObserver> = codex_captured.clone();
        let mut parts = HashMap::new();
        let mut codex_answer = String::new();
        for value in [
            serde_json::json!({"type":"item.started","item":{"type":"reasoning"}}),
            serde_json::json!({"type":"item.started","item":{"type":"command_execution","id":"c1","command":"ls"}}),
            serde_json::json!({"type":"item.completed","item":{"type":"command_execution","id":"c1","command":"ls","aggregated_output":"file.rs"}}),
            serde_json::json!({"type":"item.completed","item":{"type":"agent_message","id":"m1","text":"Done"}}),
            serde_json::json!({"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5}}),
        ] {
            normalize(&value, StreamProtocol::CodexJson, &codex_observer, &mut parts, &mut codex_answer);
        }

        let claude_captured = Arc::new(Observer::default());
        let claude_observer: Arc<dyn RunObserver> = claude_captured.clone();
        let mut claude_parts = HashMap::new();
        let mut claude_answer = String::new();
        for value in [
            serde_json::json!({"event":{"type":"content_block_delta","delta":{"type":"thinking_delta"}}}),
            serde_json::json!({"event":{"type":"content_block_start","content_block":{"type":"tool_use","name":"bash","input":{}}}}),
            serde_json::json!({"type":"user","message":{"content":[{"type":"tool_result","content":"file.rs"}]}}),
            serde_json::json!({"event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Done"}}}),
            serde_json::json!({"type":"result","usage":{"input_tokens":10,"output_tokens":5}}),
        ] {
            normalize(&value, StreamProtocol::ClaudeJson, &claude_observer, &mut claude_parts, &mut claude_answer);
        }

        assert_eq!(codex_answer, "Done");
        assert_eq!(claude_answer, "Done");

        let codex_kinds: Vec<String> = codex_captured.events.lock().unwrap().iter().map(|(k, _)| k.clone()).collect();
        let claude_kinds: Vec<String> = claude_captured.events.lock().unwrap().iter().map(|(k, _)| k.clone()).collect();

        assert!(codex_kinds.contains(&"status".to_string()));
        assert!(claude_kinds.contains(&"status".to_string()));
        assert!(codex_kinds.contains(&"tool_input".to_string()));
        assert!(claude_kinds.contains(&"tool_input".to_string()));
        assert!(codex_kinds.contains(&"tool_output".to_string()));
        assert!(claude_kinds.contains(&"tool_output".to_string()));
        assert!(codex_kinds.contains(&"usage".to_string()));
        assert!(claude_kinds.contains(&"usage".to_string()));
    }
}
