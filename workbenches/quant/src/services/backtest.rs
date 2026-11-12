//! Application Service orchestrating Quantitative Backtesting, Indicators, PBO, and Grid Engines.

use super::dto::*;
use crate::{
    calculate_dsr, calculate_pbo_cscv, detect_lookahead_bias, BacktestConfig, BacktestTearsheet,
    BarTimeframe, DualMovingAverageRule, GridConfig, GridMode, MonthlyReturn, NativeBacktestEngine,
    NativeDataPortal, NativeGridEngine, OhlcvBar, PboResult, ProtectionSpec, QuantPort,
};
use custos_domain::DomainError;

pub struct BacktestService;

impl BacktestService {
    pub async fn submit_backtest(
        req: SubmitBacktestRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<BacktestTearsheet, DomainError> {
        if req.strategy_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'strategy_id' parameter".into(),
            ));
        }
        if req.task_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'task_id' parameter".into(),
            ));
        }

        // Validate lookahead bias on formula if provided
        if let Some(formula) = &req.formula {
            detect_lookahead_bias(formula).map_err(|e| {
                DomainError::Validation(format!("Lookahead bias check failed: {e}"))
            })?;
        }

        // Resolve historical bars
        let maybe_bars: Option<Vec<OhlcvBar>> = if let Some(raw_bars) = req.bars {
            let b_vec: Vec<OhlcvBar> = raw_bars.into_iter().map(Into::into).collect();
            if b_vec.len() >= 5 {
                Some(b_vec)
            } else {
                None
            }
        } else if let Some(sym) = &req.symbol {
            match quant_port.get_market_bars(sym, &req.timeframe, 2000).await {
                Ok(b) if b.len() >= 5 => Some(b),
                _ => None,
            }
        } else {
            None
        };

        let sheet = if let Some(bars) = maybe_bars {
            let prot_spec = if req.stop_loss_pct.is_some() || req.take_profit_pct.is_some() {
                Some(ProtectionSpec::new(
                    req.stop_loss_pct.unwrap_or(0.05),
                    req.take_profit_pct.unwrap_or(0.15),
                    req.trailing_stop_pct.unwrap_or(0.03),
                    req.trailing_activation_pct.unwrap_or(0.05),
                ))
            } else {
                None
            };

            let cfg = BacktestConfig {
                initial_capital: req.initial_capital,
                commission_rate: req.commission,
                slippage_bps: req.slippage_bps,
                leverage: req.leverage,
                protection: prot_spec,
            };

            let rule = Box::new(DualMovingAverageRule::new(
                req.fast_period,
                req.slow_period,
                req.target_weight,
            ));
            NativeBacktestEngine::run(&req.strategy_id, &bars, &cfg, rule)
                .map_err(|e| DomainError::Validation(format!("Native simulation failed: {e}")))?
        } else {
            let dsr = calculate_dsr(
                req.annualized_sharpe,
                req.trials_count,
                req.sample_length,
                req.skewness,
                req.kurtosis,
                req.variance_of_trials,
            );

            let s = BacktestTearsheet::new(
                &req.strategy_id,
                req.annualized_sharpe,
                req.max_drawdown,
                req.total_return,
            )
            .map_err(|e| {
                DomainError::Validation(format!("Failed to create BacktestTearsheet: {e}"))
            })?
            .with_deflated_sharpe(dsr);

            let monthly: Vec<MonthlyReturn> = req
                .monthly_returns
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect();

            s.with_extended_metrics(
                req.volatility,
                req.win_rate,
                req.profit_factor,
                req.turnover,
                monthly,
            )
        };

        quant_port.save_tearsheet(&sheet, &req.task_id).await?;
        Ok(sheet)
    }

    pub async fn get_tearsheet(
        strategy_id: &str,
        quant_port: &dyn QuantPort,
    ) -> Result<Option<BacktestTearsheet>, DomainError> {
        if strategy_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'strategy_id' parameter".into(),
            ));
        }
        quant_port.get_latest_tearsheet(strategy_id).await
    }

    pub fn evaluate_grid(req: GridEvaluateRequest) -> Result<serde_json::Value, DomainError> {
        let mode = match req.mode.to_lowercase().as_str() {
            "geometric" => GridMode::Geometric,
            _ => GridMode::Arithmetic,
        };

        let config = GridConfig::new(
            &req.symbol,
            req.lower_price,
            req.upper_price,
            req.grid_count,
            mode,
            req.total_capital,
        )
        .map_err(|e| DomainError::Validation(format!("Invalid grid configuration: {e}")))?
        .with_protection(req.stop_loss_price, req.take_profit_price);

        let mut engine = NativeGridEngine::new(config).map_err(|e| {
            DomainError::Validation(format!("Failed to initialize grid engine: {e}"))
        })?;

        let actions = engine.evaluate_price(req.current_price);
        let status = engine.status();

        Ok(serde_json::json!({
            "symbol": req.symbol,
            "levels": engine.levels,
            "cells_count": engine.cells.len(),
            "actions": actions,
            "status": status,
        }))
    }

    pub async fn compute_indicators(
        req: ComputeIndicatorsRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<serde_json::Value, DomainError> {
        if req.symbol.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'symbol' parameter".into(),
            ));
        }

        let tf = BarTimeframe::from_str_loose(&req.timeframe)
            .map_err(|e| DomainError::Validation(format!("Invalid timeframe: {e}")))?;

        let bars = quant_port
            .get_market_bars(&req.symbol, &req.timeframe, req.limit)
            .await?;

        if bars.is_empty() {
            return Err(DomainError::Validation(format!(
                "No historical bars found for {} ({})",
                req.symbol, req.timeframe
            )));
        }

        let portal = NativeDataPortal::new(&bars, tf).map_err(|e| {
            DomainError::Validation(format!("Failed to initialize data portal: {e}"))
        })?;

        let latest_ts = bars
            .last()
            .map(|b| b.timestamp + tf.duration_seconds())
            .unwrap_or(0);

        let mut computed = serde_json::Map::new();
        computed.insert(
            "sma_20".into(),
            serde_json::to_value(portal.calculate_sma(latest_ts, 20))
                .unwrap_or(serde_json::Value::Null),
        );
        computed.insert(
            "sma_50".into(),
            serde_json::to_value(portal.calculate_sma(latest_ts, 50))
                .unwrap_or(serde_json::Value::Null),
        );
        computed.insert(
            "ema_20".into(),
            serde_json::to_value(portal.calculate_ema(latest_ts, 20))
                .unwrap_or(serde_json::Value::Null),
        );
        computed.insert(
            "rsi_14".into(),
            serde_json::to_value(portal.calculate_rsi(latest_ts, 14))
                .unwrap_or(serde_json::Value::Null),
        );
        computed.insert(
            "bollinger_bands_20_2".into(),
            serde_json::to_value(portal.calculate_bollinger_bands(latest_ts, 20, 2.0))
                .unwrap_or(serde_json::Value::Null),
        );
        computed.insert(
            "macd_12_26_9".into(),
            serde_json::to_value(portal.calculate_macd(latest_ts, 12, 26, 9))
                .unwrap_or(serde_json::Value::Null),
        );
        computed.insert(
            "atr_14".into(),
            serde_json::to_value(portal.calculate_atr(latest_ts, 14))
                .unwrap_or(serde_json::Value::Null),
        );
        computed.insert(
            "vwap".into(),
            serde_json::to_value(portal.calculate_vwap(latest_ts, 50))
                .unwrap_or(serde_json::Value::Null),
        );

        Ok(serde_json::json!({
            "symbol": req.symbol,
            "timeframe": req.timeframe,
            "reference_timestamp": latest_ts,
            "total_bars_evaluated": bars.len(),
            "indicators": computed,
        }))
    }

    pub fn compute_pbo(req: ComputePboRequest) -> Result<PboResult, DomainError> {
        if req.score_matrix.is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'score_matrix' array".into(),
            ));
        }
        calculate_pbo_cscv(&req.score_matrix, req.max_combinations)
            .map_err(|e| DomainError::Validation(format!("PBO evaluation failed: {e}")))
    }
}
