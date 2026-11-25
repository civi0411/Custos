//! QuantPort — Quantitative Trading persistence and risk evaluation contract
//!
//! Provides traits for saving/retrieving strategies, backtest tearsheets, order tickets,
//! fills, positions, evidence snapshots, and market data bars for the Quant Workbench.

use crate::domain::{
    BacktestTearsheet, EvidenceQualitySummary, OhlcvBar, OrderTicket, QuantPositionRecord,
    QuantStrategyRecord, TradeFillRecord,
};
use async_trait::async_trait;
use custos_contracts::DomainError;

/// Persistence Port for Quantitative Trading state (activated strictly when 'quant' pack is installed).
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

    async fn get_order_ticket(&self, order_id: &str) -> Result<Option<OrderTicket>, DomainError>;

    async fn update_order_ticket(&self, ticket: &OrderTicket) -> Result<(), DomainError>;

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

    async fn save_market_bar_meta(
        &self,
        symbol: &str,
        timeframe: &str,
        market: &str,
        venue: &str,
    ) -> Result<(), DomainError>;

    async fn save_strategy(&self, strategy: &QuantStrategyRecord) -> Result<(), DomainError>;

    async fn list_strategies(&self) -> Result<Vec<QuantStrategyRecord>, DomainError>;

    async fn get_strategy(
        &self,
        strategy_id: &str,
    ) -> Result<Option<QuantStrategyRecord>, DomainError>;

    async fn clear_backtest_fills(&self, _strategy_id: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn save_fill(
        &self,
        fill: &TradeFillRecord,
        strategy_id: &str,
        task_id: &str,
        execution_mode: &str,
    ) -> Result<(), DomainError>;

    async fn list_fills(
        &self,
        strategy_id: Option<&str>,
        symbol: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TradeFillRecord>, DomainError>;

    async fn upsert_position(&self, pos: &QuantPositionRecord) -> Result<(), DomainError>;

    async fn list_positions(
        &self,
        strategy_id: Option<&str>,
    ) -> Result<Vec<QuantPositionRecord>, DomainError>;

    async fn delete_position(
        &self,
        strategy_id: &str,
        symbol: &str,
        direction: &str,
    ) -> Result<(), DomainError>;

    async fn daily_executed_notional(&self, day_start_ts: i64) -> Result<f64, DomainError>;

    async fn save_evidence_snapshot(
        &self,
        id: &str,
        task_id: &str,
        symbol: &str,
        market: &str,
        summary: &EvidenceQualitySummary,
    ) -> Result<(), DomainError>;
}
