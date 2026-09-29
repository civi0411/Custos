use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySpan {
    pub trace_id: String,
    pub session_id: String,
    pub task_id: Option<String>,
    pub tier: String,
    pub provider_id: String,
    pub model_id: String,
    pub latency_ms: u64,
    pub tokens_used: u64,
    pub cost_usd: f32,
    pub timestamp: String,
}

#[derive(Default, Clone)]
pub struct GatewayTracer {
    spans: Arc<RwLock<Vec<GatewaySpan>>>,
}

impl GatewayTracer {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn record_span(
        &self,
        trace_id: String,
        session_id: String,
        task_id: Option<String>,
        tier: String,
        provider_id: String,
        model_id: String,
        latency_ms: u64,
        tokens_used: u64,
        cost_usd: f32,
    ) {
        let span = GatewaySpan {
            trace_id,
            session_id,
            task_id,
            tier,
            provider_id,
            model_id,
            latency_ms,
            tokens_used,
            cost_usd,
            timestamp: Utc::now().to_rfc3339(),
        };

        let mut lock = self.spans.write().await;
        lock.push(span);
    }

    pub async fn recent_spans(&self, limit: usize) -> Vec<GatewaySpan> {
        let lock = self.spans.read().await;
        lock.iter().rev().take(limit).cloned().collect()
    }
}
