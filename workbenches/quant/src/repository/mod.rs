mod portfolio;

use crate::contracts::QuantPort;
use crate::domain::{
    BacktestTearsheet, EvidenceQualitySummary, OhlcvBar, OrderDirection, OrderTicket,
    OrderTicketStatus, OrderType, QuantPositionRecord, QuantStrategyRecord, TradeFillRecord,
};
use async_trait::async_trait;
use custos_contracts::DomainError;
use rusqlite::{params, OptionalExtension};
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<rusqlite::Connection>>;

#[derive(Clone)]
pub struct QuantRepository {
    db: DbConnection,
}

impl QuantRepository {
    pub fn try_new(db: DbConnection) -> Result<Self, DomainError> {
        let repo = Self { db };
        repo.ensure_tables()?;
        Ok(repo)
    }

    pub fn new(db: DbConnection) -> Self {
        let repo = Self { db };
        if let Err(err) = repo.ensure_tables() {
            tracing::error!(error = %err, "Failed to initialize quant workbench schema");
        }
        repo
    }

    pub fn ensure_tables(&self) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        conn.execute_batch(include_str!(
            "../../migrations/0042_quant_live_pipeline.sql"
        ))
        .map_err(|e| {
            DomainError::Validation(format!("Failed to initialize quant base schema: {e}"))
        })?;
        conn.execute_batch(include_str!(
            "../../migrations/0043_quant_production_schema.sql"
        ))
        .map_err(|e| {
            DomainError::Validation(format!("Failed to initialize quant production schema: {e}"))
        })?;
        Ok(())
    }

    pub(crate) fn lock_db(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, DomainError> {
        self.db
            .lock()
            .map_err(|e| DomainError::Validation(format!("Database mutex poisoned: {e}")))
    }
}

pub(crate) fn parse_direction(raw: &str) -> OrderDirection {
    match raw {
        "sell" => OrderDirection::Sell,
        "sell_short" => OrderDirection::SellShort,
        "buy_to_cover" => OrderDirection::BuyToCover,
        _ => OrderDirection::Buy,
    }
}

pub(crate) fn direction_str(dir: OrderDirection) -> &'static str {
    match dir {
        OrderDirection::Buy => "buy",
        OrderDirection::Sell => "sell",
        OrderDirection::SellShort => "sell_short",
        OrderDirection::BuyToCover => "buy_to_cover",
    }
}

fn status_str(status: OrderTicketStatus) -> &'static str {
    match status {
        OrderTicketStatus::Draft => "draft",
        OrderTicketStatus::AwaitingPermit => "awaiting_permit",
        OrderTicketStatus::Approved => "approved",
        OrderTicketStatus::Rejected => "rejected",
        OrderTicketStatus::Routed => "routed",
        OrderTicketStatus::Filled => "filled",
        OrderTicketStatus::Cancelled => "cancelled",
    }
}

fn parse_status(raw: &str) -> OrderTicketStatus {
    match raw {
        "awaiting_permit" => OrderTicketStatus::AwaitingPermit,
        "approved" => OrderTicketStatus::Approved,
        "rejected" => OrderTicketStatus::Rejected,
        "routed" => OrderTicketStatus::Routed,
        "filled" => OrderTicketStatus::Filled,
        "cancelled" => OrderTicketStatus::Cancelled,
        _ => OrderTicketStatus::Draft,
    }
}

fn row_to_order_ticket(row: &rusqlite::Row<'_>) -> rusqlite::Result<OrderTicket> {
    let direction_s: String = row.get(3)?;
    let type_s: String = row.get(4)?;
    let status_s: String = row.get(9)?;
    let permit_id: Option<String> = row.get(10)?;
    let status = parse_status(&status_s);
    let order_type = match type_s.as_str() {
        "limit" => OrderType::Limit,
        "stop" => OrderType::Stop,
        "twap" => OrderType::Twap,
        "vwap" => OrderType::Vwap,
        _ => OrderType::Market,
    };
    let approved_by_human = permit_id.is_some()
        || matches!(
            status,
            OrderTicketStatus::Approved | OrderTicketStatus::Routed | OrderTicketStatus::Filled
        );

    Ok(OrderTicket {
        id: row.get(0)?,
        strategy_id: row.get(1)?,
        symbol: row.get(2)?,
        direction: parse_direction(&direction_s),
        order_type,
        quantity: row.get(5)?,
        limit_price: row.get(6)?,
        target_weight: row.get(7)?,
        estimated_slippage_bps: row.get(8)?,
        approved_by_human,
        status,
        permit_id,
        created_at: row.get(11)?,
    })
}

#[async_trait]
impl QuantPort for QuantRepository {
    async fn save_tearsheet(
        &self,
        tearsheet: &BacktestTearsheet,
        task_id: &str,
    ) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        let payload =
            serde_json::to_string(tearsheet).map_err(|e| DomainError::Validation(e.to_string()))?;
        let lookahead_flag = if tearsheet.lookahead_clean { 1 } else { 0 };
        conn.execute(
            "INSERT INTO quant_backtest_tearsheets (id, strategy_id, task_id, annualized_sharpe, deflated_sharpe_ratio, max_drawdown, calmar_ratio, sortino_ratio, var_95, lookahead_clean, tearsheet_payload, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                tearsheet.id,
                tearsheet.strategy_id,
                task_id,
                tearsheet.annualized_sharpe,
                tearsheet.deflated_sharpe,
                tearsheet.max_drawdown,
                tearsheet.calmar_ratio,
                tearsheet.sortino_ratio,
                tearsheet.cvar_95,
                lookahead_flag,
                payload,
                tearsheet.created_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn get_latest_tearsheet(
        &self,
        strategy_id: &str,
    ) -> Result<Option<BacktestTearsheet>, DomainError> {
        let conn = self.lock_db()?;
        let payload: Option<String> = conn
            .query_row(
                "SELECT tearsheet_payload FROM quant_backtest_tearsheets WHERE strategy_id = ?1 ORDER BY created_at DESC LIMIT 1",
                params![strategy_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        match payload {
            Some(json) => {
                let sheet: BacktestTearsheet = serde_json::from_str(&json).map_err(|e| {
                    DomainError::Validation(format!("Corrupt tearsheet payload: {e}"))
                })?;
                Ok(Some(sheet))
            }
            None => Ok(None),
        }
    }

    async fn save_order_ticket(
        &self,
        ticket: &OrderTicket,
        task_id: &str,
    ) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        let now = chrono::Utc::now().timestamp();
        let payload_digest = custos_contracts::digest(
            format!(
                "{}:{}:{}:{}:{}",
                ticket.id, ticket.strategy_id, ticket.symbol, ticket.direction, ticket.quantity
            )
            .as_bytes(),
        );
        let type_str = match ticket.order_type {
            OrderType::Market => "market",
            OrderType::Limit => "limit",
            OrderType::Stop => "stop",
            OrderType::Twap => "twap",
            OrderType::Vwap => "vwap",
        };
        conn.execute(
            "INSERT INTO quant_order_tickets (id, strategy_id, task_id, symbol, direction, order_type, quantity, limit_price, target_weight, estimated_slippage_bps, status, permit_id, payload_digest, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                permit_id = excluded.permit_id,
                updated_at = excluded.updated_at",
            params![
                ticket.id,
                ticket.strategy_id,
                task_id,
                ticket.symbol,
                direction_str(ticket.direction),
                type_str,
                ticket.quantity,
                ticket.limit_price,
                ticket.target_weight,
                ticket.estimated_slippage_bps,
                status_str(ticket.status),
                ticket.permit_id,
                payload_digest,
                ticket.created_at,
                now,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn list_order_tickets(
        &self,
        strategy_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<OrderTicket>, DomainError> {
        let conn = self.lock_db()?;
        let mut sql = "SELECT id, strategy_id, symbol, direction, order_type, quantity, limit_price, target_weight, estimated_slippage_bps, status, permit_id, created_at FROM quant_order_tickets WHERE 1=1".to_string();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(strat) = strategy_id {
            sql.push_str(" AND strategy_id = ?");
            params_vec.push(rusqlite::types::Value::Text(strat.to_string()));
        }
        if let Some(task) = task_id {
            sql.push_str(" AND task_id = ?");
            params_vec.push(rusqlite::types::Value::Text(task.to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter()),
                row_to_order_ticket,
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut tickets = Vec::new();
        for r in rows {
            tickets.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(tickets)
    }

    async fn get_order_ticket(&self, order_id: &str) -> Result<Option<OrderTicket>, DomainError> {
        let conn = self.lock_db()?;
        conn.query_row(
            "SELECT id, strategy_id, symbol, direction, order_type, quantity, limit_price, target_weight, estimated_slippage_bps, status, permit_id, created_at FROM quant_order_tickets WHERE id = ?1",
            params![order_id],
            row_to_order_ticket,
        )
        .optional()
        .map_err(|e| DomainError::Validation(e.to_string()))
    }

    async fn update_order_ticket(&self, ticket: &OrderTicket) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE quant_order_tickets SET status = ?1, permit_id = ?2, updated_at = ?3 WHERE id = ?4",
            params![status_str(ticket.status), ticket.permit_id, now, ticket.id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn save_market_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        bars: &[OhlcvBar],
    ) -> Result<(), DomainError> {
        let mut conn = self.lock_db()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO quant_market_bars (symbol, timeframe, timestamp, open, high, low, close, volume)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            for bar in bars {
                stmt.execute(params![
                    symbol,
                    timeframe,
                    bar.timestamp,
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.volume,
                ])
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            }
        }
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn get_market_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: usize,
    ) -> Result<Vec<OhlcvBar>, DomainError> {
        let conn = self.lock_db()?;
        let mut stmt = conn
            .prepare(
                "SELECT timestamp, open, high, low, close, volume FROM quant_market_bars
                 WHERE symbol = ?1 AND timeframe = ?2
                 ORDER BY timestamp DESC LIMIT ?3",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = stmt
            .query_map(params![symbol, timeframe, limit as i64], |row| {
                Ok(OhlcvBar {
                    timestamp: row.get(0)?,
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    volume: row.get(5)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut bars = Vec::new();
        for r in rows {
            bars.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        bars.reverse();
        Ok(bars)
    }

    async fn save_market_bar_meta(
        &self,
        symbol: &str,
        timeframe: &str,
        market: &str,
        venue: &str,
    ) -> Result<(), DomainError> {
        self.save_market_bar_meta_sync(symbol, timeframe, market, venue)
    }

    async fn save_strategy(&self, strategy: &QuantStrategyRecord) -> Result<(), DomainError> {
        self.save_strategy_sync(strategy)
    }

    async fn list_strategies(&self) -> Result<Vec<QuantStrategyRecord>, DomainError> {
        self.list_strategies_sync()
    }

    async fn get_strategy(
        &self,
        strategy_id: &str,
    ) -> Result<Option<QuantStrategyRecord>, DomainError> {
        self.get_strategy_sync(strategy_id)
    }

    async fn clear_backtest_fills(&self, strategy_id: &str) -> Result<(), DomainError> {
        self.clear_backtest_fills_sync(strategy_id)
    }

    async fn save_fill(
        &self,
        fill: &TradeFillRecord,
        strategy_id: &str,
        task_id: &str,
        execution_mode: &str,
    ) -> Result<(), DomainError> {
        self.save_fill_sync(fill, strategy_id, task_id, execution_mode)
    }

    async fn list_fills(
        &self,
        strategy_id: Option<&str>,
        symbol: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TradeFillRecord>, DomainError> {
        self.list_fills_sync(strategy_id, symbol, limit)
    }

    async fn upsert_position(&self, pos: &QuantPositionRecord) -> Result<(), DomainError> {
        self.upsert_position_sync(pos)
    }

    async fn list_positions(
        &self,
        strategy_id: Option<&str>,
    ) -> Result<Vec<QuantPositionRecord>, DomainError> {
        self.list_positions_sync(strategy_id)
    }

    async fn delete_position(
        &self,
        strategy_id: &str,
        symbol: &str,
        direction: &str,
    ) -> Result<(), DomainError> {
        self.delete_position_sync(strategy_id, symbol, direction)
    }

    async fn daily_executed_notional(&self, day_start_ts: i64) -> Result<f64, DomainError> {
        self.daily_executed_notional_sync(day_start_ts)
    }

    async fn save_evidence_snapshot(
        &self,
        id: &str,
        task_id: &str,
        symbol: &str,
        market: &str,
        summary: &EvidenceQualitySummary,
    ) -> Result<(), DomainError> {
        self.save_evidence_snapshot_sync(id, task_id, symbol, market, summary)
    }
}
