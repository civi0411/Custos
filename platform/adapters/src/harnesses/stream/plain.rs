//! Plain-text stream output handler.

use custos_core::contracts::harness::RunObserver;
use std::sync::Arc;

pub fn emit_plain_delta(observer: &Arc<dyn RunObserver>, text: &str) {
    if !text.is_empty() {
        observer.emit("text_delta", serde_json::json!({"text": text}));
    }
}
