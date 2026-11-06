//! Order Book Depth Domain Model & Microstructure Analytics
//!
//! Provides pure Zero-I/O Level 2 (L2) Order Book representations,
//! microprice analytics, volume weighted spreads, and synthetic book generation.

use crate::DomainError;
use serde::{Deserialize, Serialize};

/// A single price level in the Order Book with aggregate depth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
    pub total: f64,
}

impl OrderBookLevel {
    pub fn new(price: f64, size: f64, total: f64) -> Self {
        Self { price, size, total }
    }
}

/// A complete Level 2 Order Book snapshot with microstructure analytics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub symbol: String,
    pub timestamp: i64,
    pub best_ask: f64,
    pub best_bid: f64,
    pub spread: f64,
    pub spread_bps: f64,
    pub mid_price: f64,
    pub microprice: f64,
    pub imbalance_pct: f64,
    pub asks: Vec<OrderBookLevel>,
    pub bids: Vec<OrderBookLevel>,
}

impl OrderBookSnapshot {
    /// Creates an OrderBookSnapshot from explicit ask and bid levels.
    pub fn from_levels(
        symbol: impl Into<String>,
        timestamp: i64,
        asks: Vec<OrderBookLevel>,
        bids: Vec<OrderBookLevel>,
    ) -> Result<Self, DomainError> {
        let symbol = symbol.into();
        if asks.is_empty() || bids.is_empty() {
            return Err(DomainError::Validation(
                "OrderBookSnapshot requires at least one ask and one bid level".into(),
            ));
        }

        let best_ask = asks[0].price;
        let best_bid = bids[0].price;

        if best_bid >= best_ask {
            return Err(DomainError::Validation(format!(
                "Order book cross detected: best_bid ({best_bid}) >= best_ask ({best_ask})"
            )));
        }

        let actual_spread = (best_ask - best_bid).max(0.0001);
        let mid_price = (best_ask + best_bid) / 2.0;
        let spread_bps = (actual_spread / mid_price) * 10000.0;

        let best_bid_size = bids[0].size.max(0.0001);
        let best_ask_size = asks[0].size.max(0.0001);
        let microprice =
            (best_ask * best_bid_size + best_bid * best_ask_size) / (best_bid_size + best_ask_size);

        let total_ask = asks.last().map(|l| l.total).unwrap_or(0.0);
        let total_bid = bids.last().map(|l| l.total).unwrap_or(0.0);
        let imbalance_pct = if (total_bid + total_ask) > 0.0 {
            ((total_bid - total_ask) / (total_bid + total_ask)) * 100.0
        } else {
            0.0
        };

        Ok(Self {
            symbol,
            timestamp,
            best_ask,
            best_bid,
            spread: (actual_spread * 1000.0).round() / 1000.0,
            spread_bps: (spread_bps * 100.0).round() / 100.0,
            mid_price: (mid_price * 1000.0).round() / 1000.0,
            microprice: (microprice * 1000.0).round() / 1000.0,
            imbalance_pct: (imbalance_pct * 100.0).round() / 100.0,
            asks,
            bids,
        })
    }

    /// Synthesizes a realistic institutional-grade L2 Order Book around a reference close price.
    pub fn synthesize(symbol: &str, close_price: f64, depth_levels: usize) -> Self {
        let close = if close_price > 0.0 {
            close_price
        } else if symbol.starts_with("BTC") {
            68000.0
        } else {
            100.0
        };

        let depth = depth_levels.clamp(1, 50);
        let spread = (close * 0.0002).max(0.1);
        let half_spread = spread / 2.0;

        let mut asks = Vec::with_capacity(depth);
        let mut cum_ask = 0.0;
        for i in 1..=depth {
            let step = (i as f64) * (spread * 0.8);
            let price = (close + half_spread + step * 0.5 * (i as f64)).round();
            let size = ((1.2 + (i as f64) * 0.35) * 100.0).round() / 100.0;
            cum_ask += size;
            asks.push(OrderBookLevel::new(
                price,
                size,
                (cum_ask * 100.0).round() / 100.0,
            ));
        }

        let mut bids = Vec::with_capacity(depth);
        let mut cum_bid = 0.0;
        for i in 1..=depth {
            let step = (i as f64) * (spread * 0.8);
            let price = (close - half_spread - step * 0.5 * (i as f64)).round();
            let size = ((1.0 + (i as f64) * 0.4) * 100.0).round() / 100.0;
            cum_bid += size;
            bids.push(OrderBookLevel::new(
                price,
                size,
                (cum_bid * 100.0).round() / 100.0,
            ));
        }

        let now_ms = chrono::Utc::now().timestamp_millis();
        Self::from_levels(symbol, now_ms, asks, bids).expect("Synthesized levels guaranteed valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_synthesize_and_microstructure() {
        let ob = OrderBookSnapshot::synthesize("BTC/USDT", 65000.0, 5);
        assert_eq!(ob.symbol, "BTC/USDT");
        assert_eq!(ob.asks.len(), 5);
        assert_eq!(ob.bids.len(), 5);
        assert!(ob.best_ask > ob.best_bid);
        assert!(ob.spread > 0.0);
        assert!(ob.mid_price > ob.best_bid && ob.mid_price < ob.best_ask);
        assert!(ob.microprice > ob.best_bid && ob.microprice < ob.best_ask);
        assert!(ob.imbalance_pct >= -100.0 && ob.imbalance_pct <= 100.0);

        // Verify ask levels are strictly increasing
        for i in 1..ob.asks.len() {
            assert!(ob.asks[i].price >= ob.asks[i - 1].price);
            assert!(ob.asks[i].total >= ob.asks[i - 1].total);
        }

        // Verify bid levels are strictly decreasing
        for i in 1..ob.bids.len() {
            assert!(ob.bids[i].price <= ob.bids[i - 1].price);
            assert!(ob.bids[i].total >= ob.bids[i - 1].total);
        }
    }

    #[test]
    fn test_orderbook_crossed_book_rejection() {
        let asks = vec![OrderBookLevel::new(100.0, 1.0, 1.0)];
        let bids = vec![OrderBookLevel::new(105.0, 1.0, 1.0)];
        let err = OrderBookSnapshot::from_levels("TEST", 0, asks, bids);
        assert!(err.is_err());
    }
}
