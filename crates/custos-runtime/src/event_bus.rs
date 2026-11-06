//! Universal Pipeline In-Process Event Bus
//!
//! Provides typed, zero-copy broadcast distribution of domain pipeline events
//! across the 4 Custos workbenches (Quant, Engineering, Research, Copilot).

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;

#[derive(Debug, Error)]
pub enum EventBusError {
    #[error("Broadcast channel closed")]
    ChannelClosed,
    #[error("Receiver lagged behind by {0} messages")]
    Lagged(u64),
}

/// Typed Domain Pipeline Event across the 4 Sovereign Workbenches
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "modality", content = "payload")]
pub enum PipelineEvent {
    /// Quant / Market Data Workbench: Completed, validated historical or live bar
    #[serde(rename = "market_bar_closed")]
    MarketBarClosed {
        symbol: String,
        timeframe: String,
        close_time_ms: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    },
    /// Engineering Workbench: File parsed and AST updated with zero syntax errors
    #[serde(rename = "code_ast_changed")]
    CodeAstChanged {
        workspace_id: String,
        file_path: String,
        content_hash: String,
        symbol_count: usize,
    },
    /// Research Workbench: Claim validated against literature sources and grounded
    #[serde(rename = "science_claim_verified")]
    ScienceClaimVerified {
        claim_id: String,
        confidence: f64,
        source_count: usize,
    },
    /// Copilot / Autonomous Agent Workbench: Command dispatched via governed outbox
    #[serde(rename = "agent_outbox_dispatched")]
    AgentOutboxDispatched {
        command_id: String,
        target: String,
        method: String,
        status: String,
    },
    /// Infrastructure Heartbeat
    #[serde(rename = "pipeline_heartbeat")]
    PipelineHeartbeat {
        timestamp_ms: i64,
        active_subscribers: usize,
    },
}

impl PipelineEvent {
    /// Returns the canonical modality tag name for this event.
    pub fn modality_name(&self) -> &'static str {
        match self {
            Self::MarketBarClosed { .. } => "market_bar_closed",
            Self::CodeAstChanged { .. } => "code_ast_changed",
            Self::ScienceClaimVerified { .. } => "science_claim_verified",
            Self::AgentOutboxDispatched { .. } => "agent_outbox_dispatched",
            Self::PipelineHeartbeat { .. } => "pipeline_heartbeat",
        }
    }
}

/// Thread-safe in-process broadcast event bus for sovereign pipeline notifications
#[derive(Debug, Clone)]
pub struct PipelineEventBus {
    sender: broadcast::Sender<PipelineEvent>,
    published_count: Arc<AtomicU64>,
}

impl Default for PipelineEventBus {
    fn default() -> Self {
        Self::new(2048)
    }
}

impl PipelineEventBus {
    /// Create a new event bus with the given ring-buffer capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(16));
        Self {
            sender,
            published_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Subscribe to receive pipeline broadcast events.
    pub fn subscribe(&self) -> broadcast::Receiver<PipelineEvent> {
        self.sender.subscribe()
    }

    /// Broadcast an event to all active workbench listeners.
    /// Returns the number of active receivers that received the message.
    pub fn publish(&self, event: PipelineEvent) -> usize {
        self.published_count.fetch_add(1, Ordering::Relaxed);
        self.sender.send(event).unwrap_or(0)
    }

    /// Number of active broadcast subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Total count of events published through this bus since inception.
    pub fn total_published(&self) -> u64 {
        self.published_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_event_bus_broadcast() {
        let bus = PipelineEventBus::new(64);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 2);

        let event = PipelineEvent::MarketBarClosed {
            symbol: "BTCUSDT".to_string(),
            timeframe: "1m".to_string(),
            close_time_ms: 1_700_000_060_000,
            open: 65000.0,
            high: 65100.0,
            low: 64950.0,
            close: 65050.0,
            volume: 12.5,
        };

        let delivered = bus.publish(event.clone());
        assert_eq!(delivered, 2);

        let msg1 = rx1.recv().await.expect("rx1 should receive event");
        let msg2 = rx2.recv().await.expect("rx2 should receive event");

        assert_eq!(msg1, event);
        assert_eq!(msg2, event);
        assert_eq!(bus.total_published(), 1);
        assert_eq!(event.modality_name(), "market_bar_closed");
    }
}
