use crate::connection::DbConnection;
use async_trait::async_trait;
use custos_core::contracts::storage::QuantPort;
use custos_domain::quant::{
    BacktestTearsheet, OhlcvBar, OrderDirection, OrderTicket, OrderTicketStatus, OrderType,
};
use custos_domain::DomainError;
use rusqlite::{params, OptionalExtension};

#[derive(Clone)]
pub struct QuantRepository {
    db: DbConnection,
}

impl QuantRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl QuantPort for QuantRepository {
    async fn save_tearsheet(
        &self,
        tearsheet: &BacktestTearsheet,
        task_id: &str,
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let payload =
            serde_json::to_string(tearsheet).map_err(|e| DomainError::Validation(e.to_string()))?;
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
                0.0, // sortino
                0.0, // var_95
                1,   // lookahead_clean
                payload,
                tearsheet.created_at,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn get_latest_tearsheet(
        &self,
        strategy_id: &str,
    ) -> Result<Option<BacktestTearsheet>, DomainError> {
        let conn = self.db.lock()?;
        let payload: Option<String> = conn.query_row(
            "SELECT tearsheet_payload FROM quant_backtest_tearsheets WHERE strategy_id = ?1 ORDER BY created_at DESC LIMIT 1",
            params![strategy_id],
            |row| row.get(0),
        ).optional().map_err(|e| DomainError::Validation(e.to_string()))?;

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
        let conn = self.db.lock()?;
        let now = chrono::Utc::now().timestamp();
        let payload_digest = custos_domain::digest(
            format!(
                "{}:{}:{}:{}:{}",
                ticket.id, ticket.strategy_id, ticket.symbol, ticket.direction, ticket.quantity
            )
            .as_bytes(),
        );
        let status_str = match ticket.status {
            OrderTicketStatus::Draft => "draft",
            OrderTicketStatus::AwaitingPermit => "awaiting_permit",
            OrderTicketStatus::Approved => "approved",
            OrderTicketStatus::Rejected => "rejected",
            OrderTicketStatus::Routed => "routed",
            OrderTicketStatus::Filled => "filled",
            OrderTicketStatus::Cancelled => "cancelled",
        };
        let dir_str = match ticket.direction {
            OrderDirection::Buy => "buy",
            OrderDirection::Sell => "sell",
            OrderDirection::SellShort => "sell_short",
            OrderDirection::BuyToCover => "buy_to_cover",
        };
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
                dir_str,
                type_str,
                ticket.quantity,
                ticket.limit_price,
                ticket.target_weight,
                ticket.estimated_slippage_bps,
                status_str,
                ticket.permit_id,
                payload_digest,
                ticket.created_at,
                now,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn list_order_tickets(
        &self,
        strategy_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<OrderTicket>, DomainError> {
        let conn = self.db.lock()?;
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
            .query_map(rusqlite::params_from_iter(params_vec.iter()), |row| {
                let id: String = row.get(0)?;
                let strategy_id: String = row.get(1)?;
                let symbol: String = row.get(2)?;
                let direction_str: String = row.get(3)?;
                let type_str: String = row.get(4)?;
                let quantity: f64 = row.get(5)?;
                let limit_price: Option<f64> = row.get(6)?;
                let target_weight: f64 = row.get(7)?;
                let estimated_slippage_bps: f64 = row.get(8)?;
                let status_str: String = row.get(9)?;
                let permit_id: Option<String> = row.get(10)?;
                let created_at: i64 = row.get(11)?;

                let direction = match direction_str.as_str() {
                    "sell" => OrderDirection::Sell,
                    "sell_short" => OrderDirection::SellShort,
                    "buy_to_cover" => OrderDirection::BuyToCover,
                    _ => OrderDirection::Buy,
                };
                let order_type = match type_str.as_str() {
                    "limit" => OrderType::Limit,
                    "stop" => OrderType::Stop,
                    "twap" => OrderType::Twap,
                    "vwap" => OrderType::Vwap,
                    _ => OrderType::Market,
                };
                let status = match status_str.as_str() {
                    "awaiting_permit" => OrderTicketStatus::AwaitingPermit,
                    "approved" => OrderTicketStatus::Approved,
                    "rejected" => OrderTicketStatus::Rejected,
                    "routed" => OrderTicketStatus::Routed,
                    "filled" => OrderTicketStatus::Filled,
                    "cancelled" => OrderTicketStatus::Cancelled,
                    _ => OrderTicketStatus::Draft,
                };

                Ok(OrderTicket {
                    id,
                    strategy_id,
                    symbol,
                    direction,
                    order_type,
                    quantity,
                    limit_price,
                    target_weight,
                    estimated_slippage_bps,
                    approved_by_human: status == OrderTicketStatus::Approved,
                    status,
                    permit_id,
                    created_at,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut tickets = Vec::new();
        for r in rows {
            tickets.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(tickets)
    }

    async fn get_order_ticket(&self, order_id: &str) -> Result<Option<OrderTicket>, DomainError> {
        let conn = self.db.lock()?;
        let res = conn.query_row(
            "SELECT id, strategy_id, symbol, direction, order_type, quantity, limit_price, target_weight, estimated_slippage_bps, status, permit_id, created_at FROM quant_order_tickets WHERE id = ?1",
            params![order_id],
            |row| {
                let id: String = row.get(0)?;
                let strategy_id: String = row.get(1)?;
                let symbol: String = row.get(2)?;
                let direction_str: String = row.get(3)?;
                let type_str: String = row.get(4)?;
                let quantity: f64 = row.get(5)?;
                let limit_price: Option<f64> = row.get(6)?;
                let target_weight: f64 = row.get(7)?;
                let estimated_slippage_bps: f64 = row.get(8)?;
                let status_str: String = row.get(9)?;
                let permit_id: Option<String> = row.get(10)?;
                let created_at: i64 = row.get(11)?;

                let direction = match direction_str.as_str() {
                    "sell" => OrderDirection::Sell,
                    "sell_short" => OrderDirection::SellShort,
                    "buy_to_cover" => OrderDirection::BuyToCover,
                    _ => OrderDirection::Buy,
                };
                let order_type = match type_str.as_str() {
                    "limit" => OrderType::Limit,
                    "stop" => OrderType::Stop,
                    "twap" => OrderType::Twap,
                    "vwap" => OrderType::Vwap,
                    _ => OrderType::Market,
                };
                let status = match status_str.as_str() {
                    "awaiting_permit" => OrderTicketStatus::AwaitingPermit,
                    "approved" => OrderTicketStatus::Approved,
                    "rejected" => OrderTicketStatus::Rejected,
                    "routed" => OrderTicketStatus::Routed,
                    "filled" => OrderTicketStatus::Filled,
                    "cancelled" => OrderTicketStatus::Cancelled,
                    _ => OrderTicketStatus::Draft,
                };

                Ok(OrderTicket {
                    id,
                    strategy_id,
                    symbol,
                    direction,
                    order_type,
                    quantity,
                    limit_price,
                    target_weight,
                    estimated_slippage_bps,
                    approved_by_human: status == OrderTicketStatus::Approved,
                    status,
                    permit_id,
                    created_at,
                })
            },
        ).optional().map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(res)
    }

    async fn update_order_ticket(&self, ticket: &OrderTicket) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let now = chrono::Utc::now().timestamp();
        let status_str = match ticket.status {
            OrderTicketStatus::Draft => "draft",
            OrderTicketStatus::AwaitingPermit => "awaiting_permit",
            OrderTicketStatus::Approved => "approved",
            OrderTicketStatus::Rejected => "rejected",
            OrderTicketStatus::Routed => "routed",
            OrderTicketStatus::Filled => "filled",
            OrderTicketStatus::Cancelled => "cancelled",
        };

        conn.execute(
            "UPDATE quant_order_tickets SET status = ?1, permit_id = ?2, updated_at = ?3 WHERE id = ?4",
            params![status_str, ticket.permit_id, now, ticket.id],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    async fn save_market_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        bars: &[OhlcvBar],
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO quant_market_bars (symbol, timeframe, timestamp, open, high, low, close, volume)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
            ).map_err(|e| DomainError::Validation(e.to_string()))?;

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
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT timestamp, open, high, low, close, volume FROM quant_market_bars
             WHERE symbol = ?1 AND timeframe = ?2
             ORDER BY timestamp ASC LIMIT ?3",
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
        Ok(bars)
    }
}
