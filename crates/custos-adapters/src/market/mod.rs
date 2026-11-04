//! Market Data Adapters for Quant Workbench
//!
//! Provides reference-counted streaming, REST fallback, and normalized ingestion.

pub mod stream;

pub use stream::{
    MarketStreamCoordinator, MarketStreamError, StreamState, SubscriptionKey,
};
