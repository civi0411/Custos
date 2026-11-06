//! OpenCode JSON protocol normalization.

use custos_core::contracts::harness::RunObserver;
use serde_json::Value;
use std::sync::Arc;

pub fn normalize_opencode(value: &Value, observer: &Arc<dyn RunObserver>) -> Option<String> {
    let mut text: Option<String> = None;
    let kind = value["type"].as_str().unwrap_or("");

    if kind == "text" {
        text = value
            .pointer("/part/text")
            .and_then(Value::as_str)
            .map(str::to_string);
    }
    if kind == "reasoning" {
        observer.emit("status", serde_json::json!({"status":"thinking"}));
    }
    if kind == "tool_use" {
        observer.emit("status", serde_json::json!({"status":"tool-running"}));
        observer.emit(
            "tool_input",
            serde_json::json!({
                "name": value.pointer("/part/tool"),
                "input": value.pointer("/part/state/input")
            }),
        );
        if value.pointer("/part/state/output").is_some() {
            observer.emit(
                "tool_output",
                serde_json::json!({"output": value.pointer("/part/state/output")}),
            );
        }
    }
    if kind == "step_finish" {
        observer.emit(
            "usage",
            value
                .pointer("/part/tokens")
                .cloned()
                .unwrap_or(Value::Null),
        );
    }

    text
}
