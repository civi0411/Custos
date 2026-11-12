//! Harness observer primitives: no-op discarding and cancellation wrapping.

use custos_core::contracts::harness::RunObserver;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Discards all emitted events.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopObserver;

impl RunObserver for NoopObserver {
    fn emit(&self, _kind: &str, _payload: Value) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Wraps an inner [`RunObserver`] and intercepts cancellation flags.
pub struct CancellableObserver {
    inner: Arc<dyn RunObserver>,
    cancelled: Arc<AtomicBool>,
}

impl CancellableObserver {
    pub fn new(inner: Arc<dyn RunObserver>, cancelled: Arc<AtomicBool>) -> Self {
        Self { inner, cancelled }
    }
}

impl RunObserver for CancellableObserver {
    fn emit(&self, kind: &str, payload: Value) {
        self.inner.emit(kind, payload);
    }
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed) || self.inner.is_cancelled()
    }
    fn run_id(&self) -> Option<&str> {
        self.inner.run_id()
    }
}
