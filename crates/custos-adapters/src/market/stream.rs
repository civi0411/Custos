//! High-performance, Reference-Counted Market Data Stream Adapter
//!
//! Provides multiplexed stream subscription management, circuit breaking,
//! and automatic degradation to REST polling upon network failure.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use custos_domain::pipeline::{ClosedBarInvariant, DataModality, PipelineRecord};
use custos_domain::quant::portal::BarTimeframe;
use custos_domain::OhlcvBar;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Error)]
pub enum MarketStreamError {
    #[error("Validation invariant failed: {0}")]
    InvariantViolation(String),
    #[error("Subscription not found: {0}")]
    NotFound(String),
    #[error("Stream disconnected or failed: {0}")]
    TransportError(String),
}

/// Operational state of the market data stream adapter
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamState {
    Disconnected,
    Streaming,
    DegradedToRestPolling { consecutive_failures: u32, poll_interval_secs: u64 },
}

/// Shared subscription key
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionKey {
    pub symbol: String,
    pub timeframe: String,
}

impl SubscriptionKey {
    pub fn new(symbol: impl Into<String>, timeframe: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into().to_uppercase(),
            timeframe: timeframe.into().to_lowercase(),
        }
    }
}

/// Active subscription record with reference counting
#[derive(Debug, Clone)]
struct SubscriptionEntry {
    ref_count: usize,
}

/// High-throughput market stream coordinator with reference-counted subscriptions
/// and automated circuit breaker fallback to REST polling.
#[derive(Debug, Clone)]
pub struct MarketStreamCoordinator {
    subscriptions: Arc<RwLock<HashMap<SubscriptionKey, SubscriptionEntry>>>,
    sender: broadcast::Sender<PipelineRecord<OhlcvBar>>,
    consecutive_failures: Arc<AtomicU32>,
    failure_threshold: u32,
    poll_interval_secs: u64,
}

impl Default for MarketStreamCoordinator {
    fn default() -> Self {
        Self::new(3, 10, 1024)
    }
}

impl MarketStreamCoordinator {
    /// Create a new coordinator with given circuit breaker threshold and buffer capacity.
    pub fn new(failure_threshold: u32, poll_interval_secs: u64, channel_capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(channel_capacity.max(64));
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            sender,
            consecutive_failures: Arc::new(AtomicU32::new(0)),
            failure_threshold: failure_threshold.max(1),
            poll_interval_secs: poll_interval_secs.max(1),
        }
    }

    /// Current operational state of the adapter.
    pub fn state(&self) -> StreamState {
        let failures = self.consecutive_failures.load(Ordering::Relaxed);
        if failures >= self.failure_threshold {
            StreamState::DegradedToRestPolling {
                consecutive_failures: failures,
                poll_interval_secs: self.poll_interval_secs,
            }
        } else {
            StreamState::Streaming
        }
    }

    /// Record a transport failure. If failures exceed threshold, automatically degrades state.
    pub fn record_failure(&self) -> StreamState {
        self.consecutive_failures.fetch_add(1, Ordering::SeqCst);
        self.state()
    }

    /// Record successful message receipt, resetting consecutive failure count.
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
    }

    /// Subscribe to a symbol/timeframe stream.
    /// Returns `(is_first_subscriber, receiver)`
    pub async fn subscribe(
        &self,
        symbol: &str,
        timeframe: &str,
    ) -> (bool, broadcast::Receiver<PipelineRecord<OhlcvBar>>) {
        let key = SubscriptionKey::new(symbol, timeframe);
        let mut subs = self.subscriptions.write().await;
        let entry = subs.entry(key).or_insert(SubscriptionEntry { ref_count: 0 });
        let is_first = entry.ref_count == 0;
        entry.ref_count += 1;
        (is_first, self.sender.subscribe())
    }

    /// Unsubscribe from a symbol/timeframe stream.
    /// Returns `true` if this was the last subscriber and the feed can be closed.
    pub async fn unsubscribe(&self, symbol: &str, timeframe: &str) -> bool {
        let key = SubscriptionKey::new(symbol, timeframe);
        let mut subs = self.subscriptions.write().await;
        if let Some(entry) = subs.get_mut(&key) {
            if entry.ref_count > 1 {
                entry.ref_count -= 1;
                false
            } else {
                subs.remove(&key);
                true
            }
        } else {
            false
        }
    }

    /// Total number of distinct active market subscriptions.
    pub async fn active_subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }

    /// Ingest a candidate market bar from WebSocket or REST poll.
    /// Validates `ClosedBarInvariant` against `reference_ts` before promoting to Silver and broadcasting.
    #[allow(clippy::too_many_arguments)]
    pub fn ingest_bar(
        &self,
        symbol: &str,
        timeframe: &str,
        bar_open_ts: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        reference_ts: i64,
    ) -> Result<PipelineRecord<OhlcvBar>, MarketStreamError> {
        let tf = BarTimeframe::from_str_loose(timeframe)
            .map_err(|e| MarketStreamError::InvariantViolation(e.to_string()))?;

        let bar = OhlcvBar::new(bar_open_ts, open, high, low, close, volume);

        let bronze = PipelineRecord::new_bronze(
            format!("{}:{}:{}", symbol.to_uppercase(), timeframe.to_lowercase(), bar_open_ts),
            DataModality::Market,
            bar,
            bar_open_ts,
        );

        // Verify closed bar invariant strictly against current wall clock reference
        let invariant = ClosedBarInvariant::new(bar_open_ts, tf.duration_seconds(), reference_ts);
        let silver = bronze
            .promote_to_silver(&invariant)
            .map_err(|e| MarketStreamError::InvariantViolation(e.to_string()))?;

        // Broadcast to downstream subscribers
        self.record_success();
        let _ = self.sender.send(silver.clone());

        Ok(silver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_market_stream_ref_counting_and_circuit_breaker() {
        let coordinator = MarketStreamCoordinator::new(3, 5, 32);

        // Subscriptions
        let (first1, mut rx1) = coordinator.subscribe("BTCUSDT", "1m").await;
        assert!(first1);
        let (first2, _rx2) = coordinator.subscribe("BTCUSDT", "1m").await;
        assert!(!first2); // Shared connection
        assert_eq!(coordinator.active_subscription_count().await, 1);

        // Unsubscribe one
        let closed1 = coordinator.unsubscribe("BTCUSDT", "1m").await;
        assert!(!closed1); // Still 1 subscriber
        let closed2 = coordinator.unsubscribe("BTCUSDT", "1m").await;
        assert!(closed2); // Closed last subscriber
        assert_eq!(coordinator.active_subscription_count().await, 0);

        // Ingest unclosed bar (open=1000, 1m bar closes at 1060, reference is 1050) -> must fail invariant
        let res_unclosed = coordinator.ingest_bar(
            "BTCUSDT", "1m", 1000, 65000.0, 65100.0, 64900.0, 65050.0, 10.0, 1050,
        );
        assert!(res_unclosed.is_err());

        // Ingest valid closed bar (open=1000, 1m bar closes at 1060, reference is 1061) -> succeeds & broadcasts
        let res_closed = coordinator.ingest_bar(
            "BTCUSDT", "1m", 1000, 65000.0, 65100.0, 64900.0, 65050.0, 10.0, 1061,
        );
        assert!(res_closed.is_ok());
        let received = rx1.recv().await.expect("Must receive closed bar");
        assert_eq!(received.payload.close, 65050.0);
        assert!(received.is_verified);

        // Circuit breaker test
        assert_eq!(coordinator.state(), StreamState::Streaming);
        coordinator.record_failure();
        coordinator.record_failure();
        assert_eq!(coordinator.state(), StreamState::Streaming);
        let degraded = coordinator.record_failure();
        assert!(matches!(degraded, StreamState::DegradedToRestPolling { consecutive_failures: 3, .. }));
        coordinator.record_success();
        assert_eq!(coordinator.state(), StreamState::Streaming);
    }
}
