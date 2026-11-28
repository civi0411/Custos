//! Zero-PII Push Gateway contract.
//!
//! Mobile devices never hold push provider API keys; `custos-daemon` dispatches
//! notifications while recording only aggregate counters (0 tokens, 0 titles, 0 bodies).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushCategory {
    PermitApproval,
    HookPermission,
    GpuCheckpoint,
    AutomationAlert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroPiiPushCommand {
    pub device_id: String,
    pub category: PushCategory,
    pub badge_count: u32,
}

#[derive(Debug, Default)]
pub struct PushDeliveryCounters {
    pub sent: AtomicU64,
    pub failed: AtomicU64,
    pub suppressed: AtomicU64,
}

impl PushDeliveryCounters {
    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.sent.load(Ordering::Relaxed),
            self.failed.load(Ordering::Relaxed),
            self.suppressed.load(Ordering::Relaxed),
        )
    }
}

#[async_trait]
pub trait ZeroPiiPushAdapter: Send + Sync {
    async fn send(&self, cmd: &ZeroPiiPushCommand) -> Result<(), String>;
}

pub struct ZeroPiiPushGateway {
    adapter: Arc<dyn ZeroPiiPushAdapter>,
    pub counters: PushDeliveryCounters,
}

impl ZeroPiiPushGateway {
    pub fn new(adapter: Arc<dyn ZeroPiiPushAdapter>) -> Self {
        Self {
            adapter,
            counters: PushDeliveryCounters::default(),
        }
    }

    pub async fn dispatch(&self, cmd: &ZeroPiiPushCommand) -> Result<(), String> {
        if cmd.device_id.trim().is_empty() {
            self.counters.suppressed.fetch_add(1, Ordering::Relaxed);
            return Err("device_id cannot be empty".into());
        }
        match self.adapter.send(cmd).await {
            Ok(()) => {
                self.counters.sent.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(err) => {
                self.counters.failed.fetch_add(1, Ordering::Relaxed);
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_delivery_counters_track_aggregates_without_pii() {
        let counters = PushDeliveryCounters::default();
        counters.sent.fetch_add(2, Ordering::Relaxed);
        counters.suppressed.fetch_add(1, Ordering::Relaxed);
        assert_eq!(counters.snapshot(), (2, 0, 1));
    }
}

