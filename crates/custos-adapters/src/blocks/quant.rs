//! Quant Workbench Adapter Block
//!
//! Provides concrete boundary implementations for real-time and polling market data streams,
//! exchange quotes, and order routing telemetry.

pub use crate::market::stream::{
    MarketStreamCoordinator, MarketStreamError, StreamState, SubscriptionKey,
};
