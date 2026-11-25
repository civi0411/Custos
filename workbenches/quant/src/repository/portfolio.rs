use super::{direction_str, parse_direction, QuantRepository};
use crate::domain::{
    EvidenceQualitySummary, QuantPositionRecord, QuantStrategyRecord, TradeFillRecord,
};
use custos_contracts::DomainError;
use rusqlite::{params, OptionalExtension};

impl QuantRepository {
    pub(super) fn save_market_bar_meta_sync(
        &self,
        symbol: &str,
        timeframe: &str,
        market: &str,
        venue: &str,
    ) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO quant_market_bar_meta (symbol, timeframe, market, venue, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(symbol, timeframe) DO UPDATE SET
                market = excluded.market,
                venue = excluded.venue,
                updated_at = excluded.updated_at",
            params![symbol, timeframe, market, venue, now],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub(super) fn save_strategy_sync(
        &self,
        strategy: &QuantStrategyRecord,
    ) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        conn.execute(
            "INSERT INTO quant_strategies (id, name, market, venue, symbol, timeframe, formula, direction_mode, status, latest_sharpe, latest_ic_mean, source_hash, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                market = excluded.market,
                venue = excluded.venue,
                symbol = excluded.symbol,
                timeframe = excluded.timeframe,
                formula = excluded.formula,
                direction_mode = excluded.direction_mode,
                status = excluded.status,
                latest_sharpe = COALESCE(excluded.latest_sharpe, quant_strategies.latest_sharpe),
                latest_ic_mean = COALESCE(excluded.latest_ic_mean, quant_strategies.latest_ic_mean),
                source_hash = excluded.source_hash,
                updated_at = excluded.updated_at",
            params![
                strategy.id,
                strategy.name,
                strategy.market,
                strategy.venue,
                strategy.symbol,
                strategy.timeframe,
                strategy.formula,
                strategy.direction_mode,
                strategy.status,
                strategy.latest_sharpe,
                strategy.latest_ic_mean,
                strategy.source_hash,
                strategy.created_at,
                strategy.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub(super) fn list_strategies_sync(&self) -> Result<Vec<QuantStrategyRecord>, DomainError> {
        let conn = self.lock_db()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, market, venue, symbol, timeframe, formula, direction_mode, status, latest_sharpe, latest_ic_mean, source_hash, created_at, updated_at
                 FROM quant_strategies ORDER BY updated_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(QuantStrategyRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    market: row.get(2)?,
                    venue: row.get(3)?,
                    symbol: row.get(4)?,
                    timeframe: row.get(5)?,
                    formula: row.get(6)?,
                    direction_mode: row.get(7)?,
                    status: row.get(8)?,
                    latest_sharpe: row.get(9)?,
                    latest_ic_mean: row.get(10)?,
                    source_hash: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(out)
    }

    pub(super) fn get_strategy_sync(
        &self,
        strategy_id: &str,
    ) -> Result<Option<QuantStrategyRecord>, DomainError> {
        let conn = self.lock_db()?;
        conn.query_row(
            "SELECT id, name, market, venue, symbol, timeframe, formula, direction_mode, status, latest_sharpe, latest_ic_mean, source_hash, created_at, updated_at
             FROM quant_strategies WHERE id = ?1",
            params![strategy_id],
            |row| {
                Ok(QuantStrategyRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    market: row.get(2)?,
                    venue: row.get(3)?,
                    symbol: row.get(4)?,
                    timeframe: row.get(5)?,
                    formula: row.get(6)?,
                    direction_mode: row.get(7)?,
                    status: row.get(8)?,
                    latest_sharpe: row.get(9)?,
                    latest_ic_mean: row.get(10)?,
                    source_hash: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            },
        )
        .optional()
        .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub(super) fn clear_backtest_fills_sync(&self, strategy_id: &str) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        conn.execute(
            "DELETE FROM quant_fills WHERE strategy_id = ?1 AND execution_mode = 'backtest'",
            params![strategy_id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub(super) fn save_fill_sync(
        &self,
        fill: &TradeFillRecord,
        strategy_id: &str,
        task_id: &str,
        execution_mode: &str,
    ) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        conn.execute(
            "INSERT OR REPLACE INTO quant_fills (fill_id, order_id, strategy_id, task_id, symbol, direction, price, quantity, commission, close_reason, execution_mode, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                fill.fill_id,
                fill.order_id,
                strategy_id,
                task_id,
                fill.symbol,
                direction_str(fill.direction),
                fill.price,
                fill.quantity,
                fill.commission,
                fill.exit_reason,
                execution_mode,
                fill.timestamp,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub(super) fn list_fills_sync(
        &self,
        strategy_id: Option<&str>,
        symbol: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TradeFillRecord>, DomainError> {
        // Never mix historical backtest fills into the same FIFO queue as paper/live fills.
        let live_or_paper = self.query_fills_by_mode(strategy_id, symbol, limit, true)?;
        if !live_or_paper.is_empty() {
            return Ok(live_or_paper);
        }
        self.query_fills_by_mode(strategy_id, symbol, limit, false)
    }

    fn query_fills_by_mode(
        &self,
        strategy_id: Option<&str>,
        symbol: Option<&str>,
        limit: usize,
        real_execution_only: bool,
    ) -> Result<Vec<TradeFillRecord>, DomainError> {
        let conn = self.lock_db()?;
        let mode_clause = if real_execution_only {
            "execution_mode IN ('paper', 'live')"
        } else {
            "execution_mode = 'backtest'"
        };
        let mut sql = format!(
            "SELECT fill_id, order_id, symbol, direction, price, quantity, commission, timestamp, close_reason FROM quant_fills WHERE {mode_clause}"
        );
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(s) = strategy_id {
            sql.push_str(" AND strategy_id = ?");
            params_vec.push(rusqlite::types::Value::Text(s.to_string()));
        }
        if let Some(sym) = symbol {
            sql.push_str(" AND symbol = ?");
            params_vec.push(rusqlite::types::Value::Text(sym.to_string()));
        }
        sql.push_str(" ORDER BY timestamp ASC LIMIT ?");
        params_vec.push(rusqlite::types::Value::Integer(limit.max(1) as i64));

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params_vec.iter()), |row| {
                let dir: String = row.get(3)?;
                Ok(TradeFillRecord {
                    fill_id: row.get(0)?,
                    order_id: row.get(1)?,
                    symbol: row.get(2)?,
                    direction: parse_direction(&dir),
                    price: row.get(4)?,
                    quantity: row.get(5)?,
                    commission: row.get(6)?,
                    timestamp: row.get(7)?,
                    exit_reason: row.get(8)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(out)
    }

    pub(super) fn upsert_position_sync(
        &self,
        pos: &QuantPositionRecord,
    ) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        conn.execute(
            "INSERT INTO quant_positions (id, strategy_id, symbol, market, direction, quantity, avg_entry_price, realized_pnl, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(strategy_id, symbol, direction) DO UPDATE SET
                quantity = excluded.quantity,
                avg_entry_price = excluded.avg_entry_price,
                realized_pnl = excluded.realized_pnl,
                updated_at = excluded.updated_at",
            params![
                pos.id,
                pos.strategy_id,
                pos.symbol,
                pos.market,
                pos.direction,
                pos.quantity,
                pos.avg_entry_price,
                pos.realized_pnl,
                pos.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub(super) fn list_positions_sync(
        &self,
        strategy_id: Option<&str>,
    ) -> Result<Vec<QuantPositionRecord>, DomainError> {
        let conn = self.lock_db()?;
        let mut sql = "SELECT id, strategy_id, symbol, market, direction, quantity, avg_entry_price, realized_pnl, updated_at FROM quant_positions WHERE quantity > 0".to_string();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(s) = strategy_id {
            sql.push_str(" AND strategy_id = ?");
            params_vec.push(rusqlite::types::Value::Text(s.to_string()));
        }
        sql.push_str(" ORDER BY updated_at DESC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params_vec.iter()), |row| {
                Ok(QuantPositionRecord {
                    id: row.get(0)?,
                    strategy_id: row.get(1)?,
                    symbol: row.get(2)?,
                    market: row.get(3)?,
                    direction: row.get(4)?,
                    quantity: row.get(5)?,
                    avg_entry_price: row.get(6)?,
                    realized_pnl: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(out)
    }

    pub(super) fn delete_position_sync(
        &self,
        strategy_id: &str,
        symbol: &str,
        direction: &str,
    ) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        conn.execute(
            "DELETE FROM quant_positions WHERE strategy_id = ?1 AND symbol = ?2 AND direction = ?3",
            params![strategy_id, symbol, direction],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub(super) fn daily_executed_notional_sync(
        &self,
        day_start_ts: i64,
    ) -> Result<f64, DomainError> {
        let conn = self.lock_db()?;
        let total: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(ABS(price * quantity)), 0.0) FROM quant_fills WHERE timestamp >= ?1 AND execution_mode IN ('paper', 'live')",
                params![day_start_ts],
                |row| row.get(0),
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(total)
    }

    pub(super) fn save_evidence_snapshot_sync(
        &self,
        id: &str,
        task_id: &str,
        symbol: &str,
        market: &str,
        summary: &EvidenceQualitySummary,
    ) -> Result<(), DomainError> {
        let conn = self.lock_db()?;
        let payload =
            serde_json::to_string(summary).map_err(|e| DomainError::Validation(e.to_string()))?;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT OR REPLACE INTO quant_evidence_snapshots (id, task_id, symbol, market, quality_score, confidence_cap, payload_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                task_id,
                symbol,
                market,
                summary.quality_score,
                summary.confidence_cap,
                payload,
                now,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }
}
