//! OpenAI Codex exec JSON protocol normalization.

use custos_core::contracts::harness::RunObserver;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub fn normalize_codex(
    value: &Value,
    observer: &Arc<dyn RunObserver>,
    parts: &mut HashMap<String, String>,
    answer: &str,
) -> Option<String> {
    let mut text: Option<String> = None;
    let kind = value["type"].as_str().unwrap_or("");

    let item = &value["item"];
    let item_kind = item["type"].as_str().unwrap_or("");
    if matches!(kind, "item.started" | "item.updated" | "item.completed") {
        if item_kind == "agent_message" {
            let id = item["id"].as_str().unwrap_or("answer");
            let full = item["text"].as_str().unwrap_or("");
            let new_message = !parts.contains_key(id) && !answer.is_empty();
            let previous = parts.entry(id.into()).or_default();
            if let Some(delta) = full.strip_prefix(previous.as_str()) {
                text = Some(if new_message {
                    format!("\n\n{delta}")
                } else {
                    delta.into()
                });
            }
            *previous = full.into();
        } else if item_kind == "reasoning" {
            observer.emit("status", serde_json::json!({"status":"thinking"}));
        } else if matches!(
            item_kind,
            "command_execution" | "mcp_tool_call" | "file_change" | "web_search"
        ) {
            observer.emit("status", serde_json::json!({"status":"tool-running"}));
            observer.emit(
                if kind == "item.completed" {
                    "tool_output"
                } else {
                    "tool_input"
                },
                serde_json::json!({
                    "name": item_kind,
                    "call_id": item["id"],
                    "command": item["command"],
                    "output": item["aggregated_output"],
                    "changes": item["changes"]
                }),
            );
        }
    }
    if kind == "turn.completed" {
        super::usage::emit_usage(observer, value["usage"].clone());
    }

    text
}
