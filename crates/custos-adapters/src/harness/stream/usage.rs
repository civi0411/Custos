//! Structured token usage emission to observers.

use custos_core::contracts::harness::RunObserver;
use serde_json::Value;
use std::sync::Arc;

pub fn emit_usage(observer: &Arc<dyn RunObserver>, usage: Value) {
    if !usage.is_object() {
        return;
    }
    let input = usage["input_tokens"]
        .as_u64()
        .or_else(|| usage["input"].as_u64());
    let output = usage["output_tokens"]
        .as_u64()
        .or_else(|| usage["output"].as_u64());
    let total = usage["total_tokens"]
        .as_u64()
        .or_else(|| usage["total"].as_u64())
        .or_else(|| input.zip(output).map(|(a, b)| a.saturating_add(b)));
    let mut public =
        serde_json::json!({"input_tokens":input,"output_tokens":output,"total_tokens":total});
    for key in [
        "cached_input_tokens",
        "cache_read_input_tokens",
        "cache_write_input_tokens",
        "cache_creation_input_tokens",
        "reasoning_output_tokens",
    ] {
        if let Some(count) = usage[key].as_u64() {
            public[key] = serde_json::json!(count);
        }
    }
    observer.emit("usage", public);
}
