//! Claude Code stream-json protocol normalization.

use custos_core::contracts::harness::RunObserver;
use serde_json::Value;
use std::sync::Arc;

pub fn normalize_claude(
    value: &Value,
    observer: &Arc<dyn RunObserver>,
    answer: &str,
) -> Option<String> {
    let mut text: Option<String> = None;
    let kind = value["type"].as_str().unwrap_or("");
    let event = value.get("event").unwrap_or(value);

    if event["type"].as_str() == Some("content_block_delta") {
        if event.pointer("/delta/type").and_then(Value::as_str) == Some("text_delta") {
            text = event
                .pointer("/delta/text")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if event.pointer("/delta/type").and_then(Value::as_str) == Some("input_json_delta") {
            observer.emit(
                "tool_input",
                serde_json::json!({"index":event["index"],"partial_json":event.pointer("/delta/partial_json")}),
            );
        }
        if event.pointer("/delta/type").and_then(Value::as_str) == Some("thinking_delta") {
            observer.emit("status", serde_json::json!({"status":"thinking"}));
        }
    }
    if event["type"].as_str() == Some("content_block_start")
        && event.pointer("/content_block/type").and_then(Value::as_str) == Some("tool_use")
    {
        observer.emit("status", serde_json::json!({"status":"tool-running"}));
        observer.emit("tool_input", event["content_block"].clone());
    }
    if kind == "assistant" {
        if let Some(blocks) = value.pointer("/message/content").and_then(Value::as_array) {
            for block in blocks {
                if block["type"].as_str() == Some("tool_use") {
                    observer.emit("status", serde_json::json!({"status":"tool-running"}));
                    observer.emit("tool_input", block.clone());
                }
            }
        }
    }
    if kind == "user" {
        if let Some(blocks) = value.pointer("/message/content").and_then(Value::as_array) {
            for block in blocks {
                if block["type"].as_str() == Some("tool_result") {
                    observer.emit("tool_output", block.clone());
                }
            }
        }
    }
    if kind == "result" {
        super::usage::emit_usage(observer, value["usage"].clone());
        if answer.is_empty() {
            text = value["result"].as_str().map(str::to_string);
        }
    }

    text
}
