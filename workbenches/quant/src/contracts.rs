//! QuantPort — Quantitative Trading persistence and risk evaluation contract
//!
//! Provides traits for saving/retrieving backtest tearsheets, order tickets,
//! and market data bars for the Quant Workbench.

use async_trait::async_trait;
use custos_domain::DomainError;
use crate::domain::{BacktestTearsheet, OhlcvBar, OrderTicket};

/// Persistence Port for Quantitative Trading backtest tearsheets, order tickets, and market data bars.
#[async_trait]
pub trait QuantPort: Send + Sync {
    async fn save_tearsheet(
        &self,
        tearsheet: &BacktestTearsheet,
        task_id: &str,
    ) -> Result<(), DomainError>;

    async fn get_latest_tearsheet(
        &self,
        strategy_id: &str,
    ) -> Result<Option<BacktestTearsheet>, DomainError>;

    async fn save_order_ticket(
        &self,
        ticket: &OrderTicket,
        task_id: &str,
    ) -> Result<(), DomainError>;

    async fn list_order_tickets(
        &self,
        strategy_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<OrderTicket>, DomainError>;

    async fn get_order_ticket(
        &self,
        order_id: &str,
    ) -> Result<Option<OrderTicket>, DomainError>;

    async fn update_order_ticket(
        &self,
        ticket: &OrderTicket,
    ) -> Result<(), DomainError>;

    async fn save_market_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        bars: &[OhlcvBar],
    ) -> Result<(), DomainError>;

    async fn get_market_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: usize,
    ) -> Result<Vec<OhlcvBar>, DomainError>;
}
