//! Backtesting, tearsheets, simulation engines, PBO and technical indicators.

use crate::api::*;
use custos_core::contracts::storage::QuantPort;
use custos_domain::quant::{
    calculate_dsr, calculate_pbo_cscv, detect_lookahead_bias, BacktestConfig, BacktestTearsheet,
    BarTimeframe, DualMovingAverageRule, GridConfig, GridMode, MonthlyReturn, NativeBacktestEngine,
    NativeDataPortal, NativeGridEngine, OhlcvBar, ProtectionSpec,
};

impl LocalApiDispatcher {
    pub(crate) async fn handle_quant_backtest(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_QUANT_BACKTEST_SUBMIT => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SqliteTaskStore not configured on daemon",
                        )
                    }
                };

                let strategy_id = match req.params.get("strategy_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.to_string(),
                    _ => {
                        return ApiResponse::error(
                            req.id,
                            "Missing or empty 'strategy_id' parameter",
                        )
                    }
                };
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.to_string(),
                    _ => return ApiResponse::error(req.id, "Missing or empty 'task_id' parameter"),
                };

                // Check lookahead bias on formula if provided
                if let Some(formula) = req.params.get("formula").and_then(|v| v.as_str()) {
                    if let Err(e) = detect_lookahead_bias(formula) {
                        return ApiResponse::error(
                            req.id,
                            format!("Lookahead bias check failed: {e}"),
                        );
                    }
                }

                // If explicit bars or symbol are provided, execute the native Rust simulation engine
                let raw_bars = req.params.get("bars").and_then(|v| v.as_array());
                let symbol_opt = req.params.get("symbol").and_then(|v| v.as_str());

                let maybe_bars: Option<Vec<OhlcvBar>> = if let Some(arr) = raw_bars {
                    let mut b_vec = Vec::new();
                    for item in arr {
                        if let (Some(t), Some(o), Some(h), Some(l), Some(c), Some(v)) = (
                            item.get("timestamp").and_then(|v| v.as_i64()),
                            item.get("open").and_then(|v| v.as_f64()),
                            item.get("high").and_then(|v| v.as_f64()),
                            item.get("low").and_then(|v| v.as_f64()),
                            item.get("close").and_then(|v| v.as_f64()),
                            item.get("volume").and_then(|v| v.as_f64()),
                        ) {
                            b_vec.push(OhlcvBar::new(t, o, h, l, c, v));
                        }
                    }
                    if b_vec.len() >= 5 {
                        Some(b_vec)
                    } else {
                        None
                    }
                } else if let Some(sym) = symbol_opt {
                    let tf = req
                        .params
                        .get("timeframe")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1d");
                    match store.quant().get_market_bars(sym, tf, 2000).await {
                        Ok(b) if b.len() >= 5 => Some(b),
                        _ => None,
                    }
                } else {
                    None
                };

                let sheet = if let Some(bars) = maybe_bars {
                    let initial_capital = req
                        .params
                        .get("initial_capital")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(100_000.0);
                    let commission = req
                        .params
                        .get("commission")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0005);
                    let slippage = req
                        .params
                        .get("slippage_bps")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(2.0);
                    let leverage = req
                        .params
                        .get("leverage")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0);

                    let prot_spec = if req.params.get("stop_loss_pct").is_some()
                        || req.params.get("take_profit_pct").is_some()
                    {
                        let sl = req
                            .params
                            .get("stop_loss_pct")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.05);
                        let tp = req
                            .params
                            .get("take_profit_pct")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.15);
                        let tr = req
                            .params
                            .get("trailing_stop_pct")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.03);
                        let tr_act = req
                            .params
                            .get("trailing_activation_pct")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.05);
                        Some(ProtectionSpec::new(sl, tp, tr, tr_act))
                    } else {
                        None
                    };

                    let cfg = BacktestConfig {
                        initial_capital,
                        commission_rate: commission,
                        slippage_bps: slippage,
                        leverage,
                        protection: prot_spec,
                    };

                    let fast = req
                        .params
                        .get("fast_period")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(5) as usize;
                    let slow = req
                        .params
                        .get("slow_period")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(20) as usize;
                    let target = req
                        .params
                        .get("target_weight")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0);

                    let rule = Box::new(DualMovingAverageRule::new(fast, slow, target));
                    match NativeBacktestEngine::run(&strategy_id, &bars, &cfg, rule) {
                        Ok(s) => s,
                        Err(e) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Native simulation failed: {e}"),
                            )
                        }
                    }
                } else {
                    let sharpe = req
                        .params
                        .get("annualized_sharpe")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let max_drawdown = req
                        .params
                        .get("max_drawdown")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.15);
                    let total_return = req
                        .params
                        .get("total_return")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.25);

                    let trials = req
                        .params
                        .get("trials_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1) as usize;
                    let sample_len = req
                        .params
                        .get("sample_length")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(252) as usize;
                    let skewness = req
                        .params
                        .get("skewness")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let kurtosis = req
                        .params
                        .get("kurtosis")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(3.0);
                    let variance = req
                        .params
                        .get("variance_of_trials")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0);

                    let dsr =
                        calculate_dsr(sharpe, trials, sample_len, skewness, kurtosis, variance);

                    let mut s = match BacktestTearsheet::new(
                        &strategy_id,
                        sharpe,
                        max_drawdown,
                        total_return,
                    ) {
                        Ok(sheet) => sheet.with_deflated_sharpe(dsr),
                        Err(e) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to create BacktestTearsheet: {e}"),
                            )
                        }
                    };

                    let vol = req
                        .params
                        .get("volatility")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.15);
                    let win_rate = req
                        .params
                        .get("win_rate")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.55);
                    let profit_factor = req
                        .params
                        .get("profit_factor")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.5);
                    let turnover = req
                        .params
                        .get("turnover")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.2);

                    let mut monthly = Vec::new();
                    if let Some(arr) = req.params.get("monthly_returns").and_then(|v| v.as_array())
                    {
                        for m in arr {
                            if let (Some(y), Some(mo), Some(r)) = (
                                m.get("year").and_then(|v| v.as_i64()),
                                m.get("month").and_then(|v| v.as_u64()),
                                m.get("return_pct").and_then(|v| v.as_f64()),
                            ) {
                                monthly.push(MonthlyReturn {
                                    year: y as i32,
                                    month: mo as u32,
                                    return_pct: r,
                                });
                            }
                        }
                    }
                    s = s.with_extended_metrics(vol, win_rate, profit_factor, turnover, monthly);
                    s
                };

                if let Err(e) = store.quant().save_tearsheet(&sheet, &task_id).await {
                    return ApiResponse::error(req.id, format!("Failed to persist tearsheet: {e}"));
                }

                ApiResponse::success(req.id, serde_json::to_value(&sheet).unwrap_or_default())
            }

            METHOD_QUANT_TEARSHEET_GET => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SqliteTaskStore not configured on daemon",
                        )
                    }
                };
                let strategy_id = match req.params.get("strategy_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'strategy_id' parameter"),
                };

                match store.quant().get_latest_tearsheet(strategy_id).await {
                    Ok(Some(sheet)) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(&sheet).unwrap_or_default(),
                    ),
                    Ok(None) => ApiResponse::error(
                        req.id,
                        format!("No tearsheet found for strategy '{strategy_id}'"),
                    ),
                    Err(e) => ApiResponse::error(req.id, format!("Error reading tearsheet: {e}")),
                }
            }

            METHOD_QUANT_GRID_EVALUATE => {
                let symbol = req
                    .params
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("BTCUSDT");
                let lower_price = req
                    .params
                    .get("lower_price")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(50_000.0);
                let upper_price = req
                    .params
                    .get("upper_price")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(80_000.0);
                let grid_count = req
                    .params
                    .get("grid_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;
                let mode_str = req
                    .params
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("arithmetic");
                let mode = match mode_str.to_lowercase().as_str() {
                    "geometric" => GridMode::Geometric,
                    _ => GridMode::Arithmetic,
                };
                let total_capital = req
                    .params
                    .get("total_capital")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(10_000.0);
                let current_price = req
                    .params
                    .get("current_price")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(65_000.0);
                let stop_loss = req.params.get("stop_loss_price").and_then(|v| v.as_f64());
                let take_profit = req.params.get("take_profit_price").and_then(|v| v.as_f64());

                let config = match GridConfig::new(
                    symbol,
                    lower_price,
                    upper_price,
                    grid_count,
                    mode,
                    total_capital,
                ) {
                    Ok(c) => c.with_protection(stop_loss, take_profit),
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid grid configuration: {e}"),
                        )
                    }
                };

                let mut engine = match NativeGridEngine::new(config) {
                    Ok(e) => e,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Failed to initialize grid engine: {e}"),
                        )
                    }
                };

                let actions = engine.evaluate_price(current_price);
                let status = engine.status();

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "symbol": symbol,
                        "levels": engine.levels,
                        "cells_count": engine.cells.len(),
                        "actions": actions,
                        "status": status,
                    }),
                )
            }

            METHOD_QUANT_INDICATORS_COMPUTE => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SqliteTaskStore not configured on daemon",
                        )
                    }
                };
                let symbol = match req.params.get("symbol").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty 'symbol' parameter"),
                };
                let timeframe_str = req
                    .params
                    .get("timeframe")
                    .and_then(|v| v.as_str())
                    .unwrap_or("1d");
                let tf = match BarTimeframe::from_str_loose(timeframe_str) {
                    Ok(t) => t,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid timeframe: {e}")),
                };
                let limit = req
                    .params
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(200) as usize;

                let bars = match store
                    .quant()
                    .get_market_bars(symbol, timeframe_str, limit)
                    .await
                {
                    Ok(b) if !b.is_empty() => b,
                    Ok(_) => {
                        return ApiResponse::error(
                            req.id,
                            format!("No historical bars found for {symbol} ({timeframe_str})"),
                        )
                    }
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Failed to read market bars: {e}"),
                        )
                    }
                };

                let portal = match NativeDataPortal::new(&bars, tf) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Failed to initialize data portal: {e}"),
                        )
                    }
                };

                let latest_ts = bars
                    .last()
                    .map(|b| b.timestamp + tf.duration_seconds())
                    .unwrap_or(0);

                let mut computed = serde_json::Map::new();

                let sma_20 = portal.calculate_sma(latest_ts, 20);
                let sma_50 = portal.calculate_sma(latest_ts, 50);
                let ema_20 = portal.calculate_ema(latest_ts, 20);
                let rsi_14 = portal.calculate_rsi(latest_ts, 14);
                let bollinger_20 = portal.calculate_bollinger_bands(latest_ts, 20, 2.0);
                let macd_12_26_9 = portal.calculate_macd(latest_ts, 12, 26, 9);
                let atr_14 = portal.calculate_atr(latest_ts, 14);
                let vwap = portal.calculate_vwap(latest_ts, 50);

                computed.insert(
                    "sma_20".into(),
                    serde_json::to_value(sma_20).unwrap_or(serde_json::Value::Null),
                );
                computed.insert(
                    "sma_50".into(),
                    serde_json::to_value(sma_50).unwrap_or(serde_json::Value::Null),
                );
                computed.insert(
                    "ema_20".into(),
                    serde_json::to_value(ema_20).unwrap_or(serde_json::Value::Null),
                );
                computed.insert(
                    "rsi_14".into(),
                    serde_json::to_value(rsi_14).unwrap_or(serde_json::Value::Null),
                );
                computed.insert(
                    "bollinger_bands_20_2".into(),
                    serde_json::to_value(bollinger_20).unwrap_or(serde_json::Value::Null),
                );
                computed.insert(
                    "macd_12_26_9".into(),
                    serde_json::to_value(macd_12_26_9).unwrap_or(serde_json::Value::Null),
                );
                computed.insert(
                    "atr_14".into(),
                    serde_json::to_value(atr_14).unwrap_or(serde_json::Value::Null),
                );
                computed.insert(
                    "vwap".into(),
                    serde_json::to_value(vwap).unwrap_or(serde_json::Value::Null),
                );

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "symbol": symbol,
                        "timeframe": timeframe_str,
                        "reference_timestamp": latest_ts,
                        "total_bars_evaluated": bars.len(),
                        "indicators": computed,
                    }),
                )
            }

            METHOD_QUANT_PBO_COMPUTE => {
                let matrix_val = match req.params.get("score_matrix").and_then(|v| v.as_array()) {
                    Some(arr) if !arr.is_empty() => arr,
                    _ => {
                        return ApiResponse::error(
                            req.id,
                            "Missing or invalid 'score_matrix' array",
                        )
                    }
                };

                let mut matrix: Vec<Vec<f64>> = Vec::with_capacity(matrix_val.len());
                for row_val in matrix_val {
                    if let Some(sub_arr) = row_val.as_array() {
                        let row: Vec<f64> = sub_arr.iter().filter_map(|v| v.as_f64()).collect();
                        matrix.push(row);
                    }
                }

                let max_comb = req
                    .params
                    .get("max_combinations")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2000) as usize;

                match calculate_pbo_cscv(&matrix, max_comb) {
                    Ok(result) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(result).unwrap_or(serde_json::Value::Null),
                    ),
                    Err(e) => ApiResponse::error(req.id, format!("PBO evaluation failed: {e}")),
                }
            }

            _ => ApiResponse::error(
                req.id,
                format!("Unknown quant backtest method: {}", req.method),
            ),
        }
    }
}
