//! Native Zero-Allocation Point-in-Time Data Portal
//!
//! Provides strict lookahead-free market data slicing, closed-bar invariant enforcement,
//! and high-performance in-memory slice inspection for algorithmic strategies in Rust.

use serde::{Deserialize, Serialize};
use super::engine::OhlcvBar;
use crate::error::DomainError;

/// Standard market bar timeframes with deterministic duration semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BarTimeframe {
    Min1,
    Min3,
    Min5,
    Min15,
    Min30,
    Hour1,
    Hour4,
    Day1,
    Week1,
}

impl BarTimeframe {
    /// Return timeframe duration in seconds.
    pub const fn duration_seconds(&self) -> i64 {
        match self {
            Self::Min1 => 60,
            Self::Min3 => 180,
            Self::Min5 => 300,
            Self::Min15 => 900,
            Self::Min30 => 1800,
            Self::Hour1 => 3600,
            Self::Hour4 => 14400,
            Self::Day1 => 86400,
            Self::Week1 => 604800,
        }
    }

    /// Parse a timeframe string (e.g. "1m", "1h", "1d").
    pub fn from_str_loose(s: &str) -> Result<Self, DomainError> {
        match s.trim().to_lowercase().as_str() {
            "1m" | "min1" => Ok(Self::Min1),
            "3m" | "min3" => Ok(Self::Min3),
            "5m" | "min5" => Ok(Self::Min5),
            "15m" | "min15" => Ok(Self::Min15),
            "30m" | "min30" => Ok(Self::Min30),
            "1h" | "hour1" => Ok(Self::Hour1),
            "4h" | "hour4" => Ok(Self::Hour4),
            "1d" | "day1" => Ok(Self::Day1),
            "1w" | "week1" => Ok(Self::Week1),
            other => Err(DomainError::Validation(format!("Unsupported timeframe: {other}"))),
        }
    }

    /// Check if a bar opened at `bar_open_ts` has completely closed by `reference_ts`.
    /// Invariant: bar_open_ts + duration <= reference_ts.
    #[inline]
    pub fn is_bar_closed(&self, bar_open_ts: i64, reference_ts: i64) -> bool {
        bar_open_ts.saturating_add(self.duration_seconds()) <= reference_ts
    }

    /// Compute the next bar close timestamp in seconds with a grace period offset in milliseconds.
    pub fn next_close_timestamp(&self, reference_ts: i64, grace_ms: i64) -> i64 {
        let dur = self.duration_seconds();
        if dur <= 0 {
            return reference_ts;
        }
        let completed_open = (reference_ts / dur) * dur;
        let next_close = completed_open + dur;
        let grace_sec = (grace_ms + 999) / 1000;
        next_close + grace_sec
    }
}

/// Zero-allocation Point-in-Time Data Portal over contiguous memory slices.
#[derive(Debug, Clone, Copy)]
pub struct NativeDataPortal<'a> {
    bars: &'a [OhlcvBar],
    timeframe: BarTimeframe,
}

impl<'a> NativeDataPortal<'a> {
    /// Create a portal wrapping a contiguous, timestamp-sorted slice of bars.
    pub fn new(bars: &'a [OhlcvBar], timeframe: BarTimeframe) -> Result<Self, DomainError> {
        // Verify sorted invariant
        for window in bars.windows(2) {
            if window[0].timestamp > window[1].timestamp {
                return Err(DomainError::Validation(
                    "Market bars slice must be sorted in strictly ascending chronological order".to_string(),
                ));
            }
        }
        Ok(Self { bars, timeframe })
    }

    /// Return all closed bars visible at `reference_ts`.
    /// Guarantees ZERO lookahead: any bar that has not finished closing is excluded.
    #[inline]
    pub fn slice_point_in_time(&self, reference_ts: i64) -> &'a [OhlcvBar] {
        let duration = self.timeframe.duration_seconds();
        // A bar is closed only if bar.timestamp + duration <= reference_ts
        // Which is equivalent to bar.timestamp <= reference_ts - duration
        let max_open_ts = reference_ts.saturating_sub(duration);
        let idx = self.bars.partition_point(|bar| bar.timestamp <= max_open_ts);
        &self.bars[..idx]
    }

    /// Return the most recently closed bar visible at `reference_ts`, if any.
    #[inline]
    pub fn current_closed_bar(&self, reference_ts: i64) -> Option<&'a OhlcvBar> {
        let visible = self.slice_point_in_time(reference_ts);
        visible.last()
    }

    /// Return the last `count` closed bars visible at `reference_ts`.
    #[inline]
    pub fn history_window(&self, reference_ts: i64, count: usize) -> &'a [OhlcvBar] {
        let visible = self.slice_point_in_time(reference_ts);
        let start = visible.len().saturating_sub(count);
        &visible[start..]
    }

    /// Total count of underlying bars.
    pub fn total_bars(&self) -> usize {
        self.bars.len()
    }

    /// Calculate Simple Moving Average (SMA) on the visible closed slice.
    pub fn calculate_sma(&self, reference_ts: i64, period: usize) -> Option<f64> {
        if period == 0 {
            return None;
        }
        let window = self.history_window(reference_ts, period);
        if window.len() < period {
            return None;
        }
        let sum: f64 = window.iter().map(|b| b.close).sum();
        Some(sum / period as f64)
    }

    /// Calculate Exponential Moving Average (EMA) on the visible closed slice.
    pub fn calculate_ema(&self, reference_ts: i64, period: usize) -> Option<f64> {
        if period == 0 {
            return None;
        }
        let window = self.history_window(reference_ts, period * 2);
        if window.len() < period {
            return None;
        }
        let alpha = 2.0 / (period as f64 + 1.0);
        let mut ema = window[0].close;
        for bar in &window[1..] {
            ema = alpha * bar.close + (1.0 - alpha) * ema;
        }
        Some(ema)
    }

    /// Calculate Relative Strength Index (RSI) on the visible closed slice.
    pub fn calculate_rsi(&self, reference_ts: i64, period: usize) -> Option<f64> {
        if period == 0 {
            return None;
        }
        let window = self.history_window(reference_ts, period + 1);
        if window.len() <= period {
            return None;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for w in window.windows(2) {
            let change = w[1].close - w[0].close;
            if change > 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
        }

        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }

    /// Calculate Bollinger Bands on the visible closed slice.
    pub fn calculate_bollinger_bands(&self, reference_ts: i64, period: usize, std_mult: f64) -> Option<BollingerBandsResult> {
        let middle = self.calculate_sma(reference_ts, period)?;
        let window = self.history_window(reference_ts, period);
        let variance: f64 = window.iter().map(|b| (b.close - middle).powi(2)).sum::<f64>() / period as f64;
        let std_dev = variance.sqrt();
        let upper = middle + std_mult * std_dev;
        let lower = middle - std_mult * std_dev;
        let bandwidth = if middle != 0.0 { (upper - lower) / middle } else { 0.0 };

        Some(BollingerBandsResult { middle, upper, lower, bandwidth })
    }

    /// Calculate MACD (Moving Average Convergence Divergence) on the visible closed slice.
    pub fn calculate_macd(&self, reference_ts: i64, fast_period: usize, slow_period: usize, signal_period: usize) -> Option<MacdResult> {
        if fast_period >= slow_period || signal_period == 0 {
            return None;
        }
        let fast_ema = self.calculate_ema(reference_ts, fast_period)?;
        let slow_ema = self.calculate_ema(reference_ts, slow_period)?;
        let macd = fast_ema - slow_ema;

        let prev_fast = self.history_window(reference_ts, 2).first().and_then(|b| self.calculate_ema(b.timestamp, fast_period)).unwrap_or(fast_ema);
        let prev_slow = self.history_window(reference_ts, 2).first().and_then(|b| self.calculate_ema(b.timestamp, slow_period)).unwrap_or(slow_ema);
        let prev_macd = prev_fast - prev_slow;

        let alpha_sig = 2.0 / (signal_period as f64 + 1.0);
        let signal = alpha_sig * macd + (1.0 - alpha_sig) * prev_macd;
        let histogram = macd - signal;

        Some(MacdResult { macd, signal, histogram })
    }

    /// Calculate Average True Range (ATR) for volatility and risk-adjusted position sizing.
    pub fn calculate_atr(&self, reference_ts: i64, period: usize) -> Option<f64> {
        if period == 0 {
            return None;
        }
        let window = self.history_window(reference_ts, period + 1);
        if window.len() <= period {
            return None;
        }

        let mut tr_sum = 0.0;
        for i in 1..window.len() {
            let high_low = window[i].high - window[i].low;
            let high_prev_close = (window[i].high - window[i - 1].close).abs();
            let low_prev_close = (window[i].low - window[i - 1].close).abs();
            let true_range = high_low.max(high_prev_close).max(low_prev_close);
            tr_sum += true_range;
        }

        Some(tr_sum / period as f64)
    }

    /// Calculate Volume Weighted Average Price (VWAP) on visible closed slice.
    pub fn calculate_vwap(&self, reference_ts: i64, max_bars: usize) -> Option<f64> {
        let window = self.history_window(reference_ts, max_bars);
        if window.is_empty() {
            return None;
        }

        let mut cum_tp_vol = 0.0;
        let mut cum_vol = 0.0;

        for bar in window {
            let typical_price = (bar.high + bar.low + bar.close) / 3.0;
            cum_tp_vol += typical_price * bar.volume;
            cum_vol += bar.volume;
        }

        if cum_vol == 0.0 {
            None
        } else {
            Some(cum_tp_vol / cum_vol)
        }
    }
}

/// Bollinger Bands calculation outcome
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BollingerBandsResult {
    pub middle: f64,
    pub upper: f64,
    pub lower: f64,
    pub bandwidth: f64,
}

/// MACD calculation outcome
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MacdResult {
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}

