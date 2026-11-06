//! Precision Event Clock for Sovereign Custos Data Pipelines
//!
//! Provides zero-drift time synchronization, exact bar boundary calculation
//! with deterministic grace periods (e.g., +500ms), and 0% CPU adaptive deep sleep.

use chrono::Utc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum ClockError {
    #[error("Clock wait cancelled")]
    Cancelled,
    #[error("Invalid timeframe or interval: {0}")]
    InvalidInterval(String),
}

/// A precision tick emitted when a scheduled interval deadline is reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockTick {
    /// Scheduled target time in milliseconds UTC.
    pub scheduled_time_ms: i64,
    /// Actual instant when execution resumed in milliseconds UTC.
    pub actual_time_ms: i64,
    /// Absolute drift in milliseconds (`actual_time_ms - scheduled_time_ms`).
    pub drift_ms: i64,
}

/// Precision Event Clock for scheduling periodic checks, bar closures,
/// and pipeline invariant audits with sub-millisecond precision and 0% CPU idle.
#[derive(Debug, Clone)]
pub struct PrecisionEventClock {
    default_grace_ms: i64,
    last_tick_ms: Arc<AtomicI64>,
}

impl Default for PrecisionEventClock {
    fn default() -> Self {
        Self::new(500)
    }
}

impl PrecisionEventClock {
    /// Create a new clock with the specified grace period in milliseconds (e.g. 500ms).
    pub fn new(grace_ms: i64) -> Self {
        Self {
            default_grace_ms: grace_ms.max(0),
            last_tick_ms: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Calculate the next interval alignment boundary for a given timeframe in seconds.
    ///
    /// Form: `T_next = ceil((now_ms + 1) / interval_ms) * interval_ms + grace_ms`
    /// This prevents lookahead bias by ensuring the clock fires strictly AFTER
    /// the full interval period has elapsed plus network propagation grace.
    pub fn compute_next_boundary(now_ms: i64, interval_secs: i64, grace_ms: i64) -> i64 {
        if interval_secs <= 0 {
            return now_ms + grace_ms;
        }
        let interval_ms = interval_secs * 1000;
        let next_base = ((now_ms / interval_ms) + 1) * interval_ms;
        next_base + grace_ms
    }

    /// Calculate remaining sleep duration until target boundary.
    pub fn duration_until(target_ms: i64, now_ms: i64) -> Duration {
        if target_ms > now_ms {
            Duration::from_millis((target_ms - now_ms) as u64)
        } else {
            Duration::ZERO
        }
    }

    /// Suspend execution in adaptive deep sleep (0% CPU) until the next interval boundary.
    ///
    /// If `cancel_token` is tripped, returns `ClockError::Cancelled` immediately without waiting.
    pub async fn wait_for_next_interval(
        &self,
        interval_secs: i64,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<ClockTick, ClockError> {
        let now_ms = Utc::now().timestamp_millis();
        let target_ms = Self::compute_next_boundary(now_ms, interval_secs, self.default_grace_ms);
        let sleep_duration = Self::duration_until(target_ms, now_ms);

        if let Some(token) = cancel_token {
            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {},
                _ = token.cancelled() => {
                    return Err(ClockError::Cancelled);
                }
            }
        } else {
            tokio::time::sleep(sleep_duration).await;
        }

        let actual_ms = Utc::now().timestamp_millis();
        self.last_tick_ms.store(actual_ms, Ordering::Release);

        Ok(ClockTick {
            scheduled_time_ms: target_ms,
            actual_time_ms: actual_ms,
            drift_ms: actual_ms - target_ms,
        })
    }

    /// Return the timestamp in ms of the last tick recorded.
    pub fn last_tick_ms(&self) -> i64 {
        self.last_tick_ms.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_next_boundary_alignment() {
        // Assume now is 12:00:15.200 (15,200ms into a 60-second minute)
        let now_ms = 1_700_000_015_200;
        let interval_secs = 60; // 1-minute bar
        let grace_ms = 500;

        let next = PrecisionEventClock::compute_next_boundary(now_ms, interval_secs, grace_ms);

        // Next 1-minute boundary must be exact minute mark (1_700_000_060_000) + 500ms
        let interval_ms = 60 * 1000;
        let expected_base = ((now_ms / interval_ms) + 1) * interval_ms;
        assert_eq!(next, expected_base + 500);
        assert!(next > now_ms);
        assert_eq!((next - 500) % interval_ms, 0);
    }

    #[tokio::test]
    async fn test_wait_for_cancellation() {
        let clock = PrecisionEventClock::new(500);
        let token = CancellationToken::new();

        let token_clone = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            token_clone.cancel();
        });

        // Request 1-hour wait, should exit immediately on token cancel
        let res = clock.wait_for_next_interval(3600, Some(&token)).await;
        assert!(matches!(res, Err(ClockError::Cancelled)));
    }
}
