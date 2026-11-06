//! Application Services for the Quant bounded context.
//!
//! Provides pure domain orchestration decoupled from transport and storage details:
//! - BacktestService: Simulations, tearsheets, indicators, grid, and PBO
//! - OrderExecutionService: Order creation, permits approval, emergency stop, orderbook
//! - MarketService: Factors discovery, bars ingestion/query, pre-trade gates, post-trade analysis

pub mod backtest;
pub mod market;
pub mod orders;

pub use backtest::BacktestService;
pub use market::MarketService;
pub use orders::OrderExecutionService;
