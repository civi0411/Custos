use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct GatewayMetrics {
    pub total_requests: AtomicU64,
    pub total_tier_zero: AtomicU64,
    pub total_system_one: AtomicU64,
    pub total_system_two: AtomicU64,
    pub total_abstains: AtomicU64,
    pub total_human_gated: AtomicU64,
    latency_records: Arc<RwLock<HashMap<String, Vec<u64>>>>, // key -> latencies in ms
}

impl GatewayMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_tier(&self, tier_name: &str) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        match tier_name {
            "tier_zero" => self.total_tier_zero.fetch_add(1, Ordering::Relaxed),
            "system_one" => self.total_system_one.fetch_add(1, Ordering::Relaxed),
            "system_two" => self.total_system_two.fetch_add(1, Ordering::Relaxed),
            "abstain" => self.total_abstains.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };
    }

    pub fn record_human_gate(&self) {
        self.total_human_gated.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn record_latency(&self, route_key: &str, latency_ms: u64) {
        let mut map = self.latency_records.write().await;
        map.entry(route_key.to_string())
            .or_default()
            .push(latency_ms);
    }
}
