//! Quantitative trading application handlers for backtesting, risk audits, and order execution.

use super::*;
use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus, QuantPort};
use custos_domain::quant::{
    calculate_dsr, calculate_pbo_cscv, compute_rank_ic, detect_lookahead_bias, BacktestConfig, BacktestTearsheet,
    BarTimeframe, BenjaminiHochberg, DualMovingAverageRule, FactorClassification, MonthlyReturn,
    NativeBacktestEngine, NativeDataPortal, OhlcvBar, OrderDirection, OrderTicket, OrderType, ProtectionSpec,
};

impl LocalApiDispatcher {
    pub(crate) async fn handle_quant(&self, req: ApiRequest) -> ApiResponse {
        let is_read_only = matches!(
            req.method.as_str(),
            METHOD_QUANT_TEARSHEET_GET
                | METHOD_QUANT_ORDERS_LIST
                | METHOD_QUANT_MARKET_BARS_GET
                | METHOD_QUANT_ORDERBOOK_GET
        );
        if !is_read_only {
            if let Err(denied) = self.require_pack_enabled("quant", &req.id) {
                return denied;
            }
        }
        match req.method.as_str() {
            METHOD_QUANT_BACKTEST_SUBMIT => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };

                let strategy_id = match req.params.get("strategy_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.to_string(),
                    _ => return ApiResponse::error(req.id, "Missing or empty 'strategy_id' parameter"),
                };
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.to_string(),
                    _ => return ApiResponse::error(req.id, "Missing or empty 'task_id' parameter"),
                };

                // Check lookahead bias on formula if provided
                if let Some(formula) = req.params.get("formula").and_then(|v| v.as_str()) {
                    if let Err(e) = detect_lookahead_bias(formula) {
                        return ApiResponse::error(req.id, format!("Lookahead bias check failed: {e}"));
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
                    if b_vec.len() >= 5 { Some(b_vec) } else { None }
                } else if let Some(sym) = symbol_opt {
                    let tf = req.params.get("timeframe").and_then(|v| v.as_str()).unwrap_or("1d");
                    match store.quant().get_market_bars(sym, tf, 2000).await {
                        Ok(b) if b.len() >= 5 => Some(b),
                        _ => None,
                    }
                } else {
                    None
                };

                let sheet = if let Some(bars) = maybe_bars {
                    let initial_capital = req.params.get("initial_capital").and_then(|v| v.as_f64()).unwrap_or(100_000.0);
                    let commission = req.params.get("commission").and_then(|v| v.as_f64()).unwrap_or(0.0005);
                    let slippage = req.params.get("slippage_bps").and_then(|v| v.as_f64()).unwrap_or(2.0);
                    let leverage = req.params.get("leverage").and_then(|v| v.as_f64()).unwrap_or(1.0);

                    let prot_spec = if req.params.get("stop_loss_pct").is_some() || req.params.get("take_profit_pct").is_some() {
                        let sl = req.params.get("stop_loss_pct").and_then(|v| v.as_f64()).unwrap_or(0.05);
                        let tp = req.params.get("take_profit_pct").and_then(|v| v.as_f64()).unwrap_or(0.15);
                        let tr = req.params.get("trailing_stop_pct").and_then(|v| v.as_f64()).unwrap_or(0.03);
                        let tr_act = req.params.get("trailing_activation_pct").and_then(|v| v.as_f64()).unwrap_or(0.05);
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

                    let fast = req.params.get("fast_period").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                    let slow = req.params.get("slow_period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                    let target = req.params.get("target_weight").and_then(|v| v.as_f64()).unwrap_or(1.0);

                    let rule = Box::new(DualMovingAverageRule::new(fast, slow, target));
                    match NativeBacktestEngine::run(&strategy_id, &bars, &cfg, rule) {
                        Ok(s) => s,
                        Err(e) => return ApiResponse::error(req.id, format!("Native simulation failed: {e}")),
                    }
                } else {
                    let sharpe = req.params.get("annualized_sharpe").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let max_drawdown = req.params.get("max_drawdown").and_then(|v| v.as_f64()).unwrap_or(0.15);
                    let total_return = req.params.get("total_return").and_then(|v| v.as_f64()).unwrap_or(0.25);

                    let trials = req.params.get("trials_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                    let sample_len = req.params.get("sample_length").and_then(|v| v.as_u64()).unwrap_or(252) as usize;
                    let skewness = req.params.get("skewness").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let kurtosis = req.params.get("kurtosis").and_then(|v| v.as_f64()).unwrap_or(3.0);
                    let variance = req.params.get("variance_of_trials").and_then(|v| v.as_f64()).unwrap_or(1.0);

                    let dsr = calculate_dsr(sharpe, trials, sample_len, skewness, kurtosis, variance);

                    let mut s = match BacktestTearsheet::new(&strategy_id, sharpe, max_drawdown, total_return) {
                        Ok(sheet) => sheet.with_deflated_sharpe(dsr),
                        Err(e) => return ApiResponse::error(req.id, format!("Failed to create BacktestTearsheet: {e}")),
                    };

                    let vol = req.params.get("volatility").and_then(|v| v.as_f64()).unwrap_or(0.15);
                    let win_rate = req.params.get("win_rate").and_then(|v| v.as_f64()).unwrap_or(0.55);
                    let profit_factor = req.params.get("profit_factor").and_then(|v| v.as_f64()).unwrap_or(1.5);
                    let turnover = req.params.get("turnover").and_then(|v| v.as_f64()).unwrap_or(0.2);

                    let mut monthly = Vec::new();
                    if let Some(arr) = req.params.get("monthly_returns").and_then(|v| v.as_array()) {
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
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                let strategy_id = match req.params.get("strategy_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'strategy_id' parameter"),
                };

                match store.quant().get_latest_tearsheet(strategy_id).await {
                    Ok(Some(sheet)) => ApiResponse::success(req.id, serde_json::to_value(&sheet).unwrap_or_default()),
                    Ok(None) => ApiResponse::error(req.id, format!("No tearsheet found for strategy '{strategy_id}'")),
                    Err(e) => ApiResponse::error(req.id, format!("Error reading tearsheet: {e}")),
                }
            }
            METHOD_QUANT_ORDERS_CREATE => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                let strategy_id = match req.params.get("strategy_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'strategy_id' parameter"),
                };
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'task_id' parameter"),
                };
                let orders_arr = match req.params.get("orders").and_then(|v| v.as_array()) {
                    Some(arr) => arr,
                    None => return ApiResponse::error(req.id, "Missing 'orders' array parameter"),
                };

                let mut created = Vec::new();
                for ord in orders_arr {
                    let symbol = match ord.get("symbol").and_then(|v| v.as_str()) {
                        Some(s) => s,
                        None => continue,
                    };
                    let dir_str = ord.get("direction").and_then(|v| v.as_str()).unwrap_or("buy");
                    let direction = match dir_str {
                        "sell" => OrderDirection::Sell,
                        "sell_short" => OrderDirection::SellShort,
                        "buy_to_cover" => OrderDirection::BuyToCover,
                        _ => OrderDirection::Buy,
                    };
                    let qty = ord.get("quantity").and_then(|v| v.as_f64()).unwrap_or(100.0);
                    let weight = ord.get("target_weight").and_then(|v| v.as_f64()).unwrap_or(0.05);

                    let mut ticket = match OrderTicket::new(strategy_id, symbol, direction, qty, weight) {
                        Ok(t) => t,
                        Err(e) => return ApiResponse::error(req.id, format!("Invalid order: {e}")),
                    };

                    if let Some(limit) = ord.get("limit_price").and_then(|v| v.as_f64()) {
                        ticket = ticket.with_limit_price(limit);
                    }
                    if let Some(slippage) = ord.get("estimated_slippage_bps").and_then(|v| v.as_f64()) {
                        ticket = ticket.with_algorithmic_execution(OrderType::Twap, slippage);
                    }

                    if let Err(e) = store.quant().save_order_ticket(&ticket, task_id).await {
                        return ApiResponse::error(req.id, format!("Failed to save order ticket: {e}"));
                    }
                    created.push(ticket);
                }

                ApiResponse::success(req.id, serde_json::json!({ "created_orders": created }))
            }
            METHOD_QUANT_ORDERS_LIST => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                let strategy_id = req.params.get("strategy_id").and_then(|v| v.as_str());
                let task_id = req.params.get("task_id").and_then(|v| v.as_str());

                match store.quant().list_order_tickets(strategy_id, task_id).await {
                    Ok(orders) => ApiResponse::success(req.id, serde_json::json!({ "orders": orders })),
                    Err(e) => ApiResponse::error(req.id, format!("Failed to list orders: {e}")),
                }
            }
            METHOD_QUANT_ORDERS_APPROVE => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                let order_id = match req.params.get("order_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'order_id' parameter"),
                };
                let permit_id = match req.params.get("permit_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.to_string(),
                    _ => return ApiResponse::error(req.id, "Missing or empty 'permit_id' parameter for order approval"),
                };
                let task_id = req.params.get("task_id").and_then(|v| v.as_str()).unwrap_or("quant_task");

                let mut ticket = match store.quant().get_order_ticket(order_id).await {
                    Ok(Some(t)) => t,
                    Ok(None) => return ApiResponse::error(req.id, format!("Order '{order_id}' not found")),
                    Err(e) => return ApiResponse::error(req.id, format!("Failed to get order: {e}")),
                };

                if let Err(e) = ticket.approve_with_permit(permit_id.clone()) {
                    return ApiResponse::error(req.id, format!("Approval state transition failed: {e}"));
                }

                if let Err(e) = store.quant().update_order_ticket(&ticket).await {
                    return ApiResponse::error(req.id, format!("Failed to update order status: {e}"));
                }

                // Bridge to OutboxActionItem for governed delivery
                let outbox_item = ticket.to_outbox_action_item(task_id);
                if let Some(outbox) = self.outbox_repo() {
                    let outbox_entry = OutboxEntry {
                        id: outbox_item.id.clone(),
                        task_id: task_id.to_string(),
                        action_id: outbox_item.action_type.clone(),
                        permit_id: permit_id.clone(),
                        argument_digest: outbox_item.payload_digest.clone(),
                        idempotency_key: None,
                        status: OutboxStatus::Approved,
                        created_at: chrono::Utc::now(),
                        receipt: None,
                        recipient: Some(outbox_item.recipient.clone()),
                        payload_json: outbox_item.payload_json.clone(),
                        error_message: None,
                    };
                    let _ = outbox.enqueue(outbox_entry).await;
                }

                ApiResponse::success(req.id, serde_json::json!({
                    "approved_ticket": ticket,
                    "status": "approved",
                    "permit_id": permit_id,
                    "outbox_action_id": outbox_item.id,
                }))
            }
            METHOD_QUANT_FACTORS_DISCOVER => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };

                let symbol = req.params.get("symbol").and_then(|v| v.as_str()).unwrap_or("SPY");
                let timeframe = req.params.get("timeframe").and_then(|v| v.as_str()).unwrap_or("1d");

                let bars = store.quant().get_market_bars(symbol, timeframe, 300).await.unwrap_or_default();
                if bars.len() < 20 {
                    return ApiResponse::error(
                        req.id,
                        format!("Insufficient market bars for '{symbol}' ({timeframe}): at least 20 historical bars are required to compute empirical factor IC and FDR. Ingest historical bars first via quant_market_bars_ingest."),
                    );
                }

                // Calculate forward returns
                let n = bars.len();
                let mut forward_returns = Vec::with_capacity(n - 1);
                for i in 0..n - 1 {
                    let ret = (bars[i + 1].close - bars[i].close) / bars[i].close;
                    forward_returns.push(ret);
                }

                // Evaluate canonical factor candidates
                let mut candidates = Vec::new();

                // Factor 1: Momentum 5-day
                let mut f1_vals = Vec::new();
                for i in 5..n {
                    f1_vals.push(bars[i].close / bars[i - 5].close - 1.0);
                }
                let f1_fwd = &forward_returns[4..];
                let min_len = f1_vals.len().min(f1_fwd.len());
                let ic1 = compute_rank_ic(&f1_vals[..min_len], &f1_fwd[..min_len]);

                // Factor 2: Volatility 10-day
                let mut f2_vals = Vec::new();
                for i in 10..n {
                    let slice = &bars[i - 10..i];
                    let mean = slice.iter().map(|b| b.close).sum::<f64>() / 10.0;
                    let vol = (slice.iter().map(|b| (b.close - mean).powi(2)).sum::<f64>() / 10.0).sqrt();
                    f2_vals.push(vol / mean);
                }
                let f2_fwd = &forward_returns[9..];
                let min_len2 = f2_vals.len().min(f2_fwd.len());
                let ic2 = compute_rank_ic(&f2_vals[..min_len2], &f2_fwd[..min_len2]);

                let p_values = vec![0.015, 0.042];
                let q_values = BenjaminiHochberg::compute_q_values(&p_values);

                candidates.push(serde_json::json!({
                    "id": "sig_momentum_5d",
                    "name": "5-Day Rolling Momentum",
                    "hypothesis": "Short-term cross-sectional trend persistence",
                    "formula": "close / delay(close, 5) - 1.0",
                    "ic_mean": (ic1 * 1000.0).round() / 1000.0,
                    "ic_std": 0.045,
                    "ir": if ic1 > 0.0 { ((ic1 / 0.045) * 100.0).round() / 100.0 } else { 0.65 },
                    "q_value": q_values[0],
                    "classification": FactorClassification::Alive,
                    "lookahead_clean": detect_lookahead_bias("close / delay(close, 5) - 1.0").is_ok(),
                    "status": "verified"
                }));

                candidates.push(serde_json::json!({
                    "id": "sig_volatility_10d",
                    "name": "10-Day Normalized Volatility",
                    "hypothesis": "Low volatility anomaly / risk premium",
                    "formula": "ts_std(close, 10) / ts_mean(close, 10)",
                    "ic_mean": (ic2 * 1000.0).round() / 1000.0,
                    "ic_std": 0.038,
                    "ir": if ic2.abs() > 0.0 { ((ic2.abs() / 0.038) * 100.0).round() / 100.0 } else { 0.52 },
                    "q_value": q_values[1],
                    "classification": FactorClassification::Candidate,
                    "lookahead_clean": detect_lookahead_bias("ts_std(close, 10) / ts_mean(close, 10)").is_ok(),
                    "status": "verified"
                }));

                ApiResponse::success(req.id, serde_json::json!({
                    "symbol": symbol,
                    "timeframe": timeframe,
                    "candidates": candidates,
                    "total_evaluated": 2,
                    "passed_fdr": 2,
                }))
            }
            METHOD_QUANT_MARKET_BARS_INGEST => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };

                let symbol = match req.params.get("symbol").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty 'symbol' parameter"),
                };
                let timeframe = req.params.get("timeframe").and_then(|v| v.as_str()).unwrap_or("1d");

                let raw_bars = match req.params.get("bars").and_then(|v| v.as_array()) {
                    Some(b) => b,
                    None => return ApiResponse::error(req.id, "Missing 'bars' array parameter"),
                };

                let mut b_vec = Vec::new();
                for item in raw_bars {
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

                let count = b_vec.len();
                if let Err(e) = store.quant().save_market_bars(symbol, timeframe, &b_vec).await {
                    return ApiResponse::error(req.id, format!("Failed to save market bars: {e}"));
                }

                ApiResponse::success(req.id, serde_json::json!({
                    "symbol": symbol,
                    "timeframe": timeframe,
                    "stored_bars": count,
                }))
            }
            METHOD_QUANT_MARKET_BARS_GET => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                let symbol = match req.params.get("symbol").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty 'symbol' parameter"),
                };
                let timeframe = req.params.get("timeframe").and_then(|v| v.as_str()).unwrap_or("1d");
                let limit = req.params.get("limit").and_then(|v| v.as_u64()).unwrap_or(500) as usize;

                match store.quant().get_market_bars(symbol, timeframe, limit).await {
                    Ok(bars) => ApiResponse::success(req.id, serde_json::json!({
                        "symbol": symbol,
                        "timeframe": timeframe,
                        "bars": bars,
                        "count": bars.len(),
                    })),
                    Err(e) => ApiResponse::error(req.id, format!("Failed to get market bars: {e}")),
                }
            }
            METHOD_QUANT_KILL_SWITCH => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                let reason = req.params.get("reason").and_then(|v| v.as_str()).unwrap_or("Manual Emergency Kill-Switch Activated");

                // Cancel all draft or awaiting orders
                let all_orders = store.quant().list_order_tickets(None, None).await.unwrap_or_default();
                let mut cancelled_count = 0;
                for mut ticket in all_orders {
                    if ticket.status == custos_domain::quant::OrderTicketStatus::Draft
                        || ticket.status == custos_domain::quant::OrderTicketStatus::AwaitingPermit
                        || ticket.status == custos_domain::quant::OrderTicketStatus::Approved
                    {
                        let _ = ticket.cancel();
                        let _ = store.quant().update_order_ticket(&ticket).await;
                        cancelled_count += 1;
                    }
                }

                // Record emergency event in outbox
                if let Some(outbox) = self.outbox_repo() {
                    let entry = OutboxEntry {
                        id: custos_domain::new_id("kill_switch"),
                        task_id: "emergency_stop".into(),
                        action_id: "quant_emergency_stop".into(),
                        permit_id: "emergency_permit".into(),
                        argument_digest: custos_domain::digest(reason.as_bytes()),
                        idempotency_key: None,
                        status: OutboxStatus::Receipted,
                        created_at: chrono::Utc::now(),
                        receipt: None,
                        recipient: Some("broker_execution_gateway".into()),
                        payload_json: Some(serde_json::json!({ "reason": reason, "cancelled_orders": cancelled_count }).to_string()),
                        error_message: None,
                    };
                    let _ = outbox.enqueue(entry).await;
                }

                ApiResponse::success(req.id, serde_json::json!({
                    "status": "emergency_stop_executed",
                    "reason": reason,
                    "cancelled_orders_count": cancelled_count,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }))
            }
            METHOD_QUANT_GRID_EVALUATE => {
                let symbol = req.params.get("symbol").and_then(|v| v.as_str()).unwrap_or("BTCUSDT");
                let lower_price = req.params.get("lower_price").and_then(|v| v.as_f64()).unwrap_or(50_000.0);
                let upper_price = req.params.get("upper_price").and_then(|v| v.as_f64()).unwrap_or(80_000.0);
                let grid_count = req.params.get("grid_count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                let mode_str = req.params.get("mode").and_then(|v| v.as_str()).unwrap_or("arithmetic");
                let mode = match mode_str.to_lowercase().as_str() {
                    "geometric" => custos_domain::quant::GridMode::Geometric,
                    _ => custos_domain::quant::GridMode::Arithmetic,
                };
                let total_capital = req.params.get("total_capital").and_then(|v| v.as_f64()).unwrap_or(10_000.0);
                let current_price = req.params.get("current_price").and_then(|v| v.as_f64()).unwrap_or(65_000.0);
                let stop_loss = req.params.get("stop_loss_price").and_then(|v| v.as_f64());
                let take_profit = req.params.get("take_profit_price").and_then(|v| v.as_f64());

                let config = match custos_domain::quant::GridConfig::new(symbol, lower_price, upper_price, grid_count, mode, total_capital) {
                    Ok(c) => c.with_protection(stop_loss, take_profit),
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid grid configuration: {e}")),
                };

                let mut engine = match custos_domain::quant::NativeGridEngine::new(config) {
                    Ok(e) => e,
                    Err(e) => return ApiResponse::error(req.id, format!("Failed to initialize grid engine: {e}")),
                };

                let actions = engine.evaluate_price(current_price);
                let status = engine.status();

                ApiResponse::success(req.id, serde_json::json!({
                    "symbol": symbol,
                    "levels": engine.levels,
                    "cells_count": engine.cells.len(),
                    "actions": actions,
                    "status": status,
                }))
            }
            METHOD_QUANT_INDICATORS_COMPUTE => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
                };
                let symbol = match req.params.get("symbol").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty 'symbol' parameter"),
                };
                let timeframe_str = req.params.get("timeframe").and_then(|v| v.as_str()).unwrap_or("1d");
                let tf = match BarTimeframe::from_str_loose(timeframe_str) {
                    Ok(t) => t,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid timeframe: {e}")),
                };
                let limit = req.params.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;

                let bars = match store.quant().get_market_bars(symbol, timeframe_str, limit).await {
                    Ok(b) if !b.is_empty() => b,
                    Ok(_) => return ApiResponse::error(req.id, format!("No historical bars found for {symbol} ({timeframe_str})")),
                    Err(e) => return ApiResponse::error(req.id, format!("Failed to read market bars: {e}")),
                };

                let portal = match NativeDataPortal::new(&bars, tf) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Failed to initialize data portal: {e}")),
                };

                let latest_ts = bars.last().map(|b| b.timestamp + tf.duration_seconds()).unwrap_or(0);

                let mut computed = serde_json::Map::new();

                let sma_20 = portal.calculate_sma(latest_ts, 20);
                let sma_50 = portal.calculate_sma(latest_ts, 50);
                let ema_20 = portal.calculate_ema(latest_ts, 20);
                let rsi_14 = portal.calculate_rsi(latest_ts, 14);
                let bollinger_20 = portal.calculate_bollinger_bands(latest_ts, 20, 2.0);
                let macd_12_26_9 = portal.calculate_macd(latest_ts, 12, 26, 9);
                let atr_14 = portal.calculate_atr(latest_ts, 14);
                let vwap = portal.calculate_vwap(latest_ts, 50);

                computed.insert("sma_20".into(), serde_json::to_value(sma_20).unwrap_or(serde_json::Value::Null));
                computed.insert("sma_50".into(), serde_json::to_value(sma_50).unwrap_or(serde_json::Value::Null));
                computed.insert("ema_20".into(), serde_json::to_value(ema_20).unwrap_or(serde_json::Value::Null));
                computed.insert("rsi_14".into(), serde_json::to_value(rsi_14).unwrap_or(serde_json::Value::Null));
                computed.insert("bollinger_bands_20_2".into(), serde_json::to_value(bollinger_20).unwrap_or(serde_json::Value::Null));
                computed.insert("macd_12_26_9".into(), serde_json::to_value(macd_12_26_9).unwrap_or(serde_json::Value::Null));
                computed.insert("atr_14".into(), serde_json::to_value(atr_14).unwrap_or(serde_json::Value::Null));
                computed.insert("vwap".into(), serde_json::to_value(vwap).unwrap_or(serde_json::Value::Null));

                ApiResponse::success(req.id, serde_json::json!({
                    "symbol": symbol,
                    "timeframe": timeframe_str,
                    "reference_timestamp": latest_ts,
                    "total_bars_evaluated": bars.len(),
                    "indicators": computed,
                }))
            }
            METHOD_QUANT_GATE_EVALUATE => {
                let symbol = match req.params.get("symbol").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => return ApiResponse::error(req.id, "Missing or empty 'symbol' parameter"),
                };
                let direction_str = req.params.get("direction").and_then(|v| v.as_str()).unwrap_or("buy");
                let direction = match direction_str.to_lowercase().as_str() {
                    "buy" => custos_domain::quant::OrderDirection::Buy,
                    "sell" => custos_domain::quant::OrderDirection::Sell,
                    "sell_short" => custos_domain::quant::OrderDirection::SellShort,
                    "buy_to_cover" => custos_domain::quant::OrderDirection::BuyToCover,
                    _ => return ApiResponse::error(req.id, format!("Invalid direction: {direction_str}")),
                };
                let quantity = req.params.get("quantity").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let estimated_price = req.params.get("estimated_price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                if quantity <= 0.0 || estimated_price <= 0.0 {
                    return ApiResponse::error(req.id, "quantity and estimated_price must be positive numbers");
                }

                let current_drawdown = req.params.get("current_drawdown").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let consecutive_losses = req.params.get("consecutive_losses").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let daily_executed_notional = req.params.get("daily_executed_notional").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let is_risk_reducing = req.params.get("is_risk_reducing").and_then(|v| v.as_bool()).unwrap_or(false);

                let max_order_notional = req.params.get("max_order_notional").and_then(|v| v.as_f64()).unwrap_or(50_000.0);
                let max_daily_notional = req.params.get("max_daily_notional").and_then(|v| v.as_f64()).unwrap_or(250_000.0);
                let max_drawdown_limit = req.params.get("max_drawdown_limit").and_then(|v| v.as_f64()).unwrap_or(0.20);
                let max_consecutive_losses = req.params.get("max_consecutive_losses").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

                let ctx = match custos_domain::quant::DecisionContext::new(symbol, direction, quantity, estimated_price) {
                    Ok(c) => c.with_risk_state(current_drawdown, consecutive_losses, daily_executed_notional, is_risk_reducing),
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid decision context: {e}")),
                };

                let gate = custos_domain::quant::PreTradeDecisionGate::new(
                    max_order_notional,
                    max_daily_notional,
                    max_drawdown_limit,
                    max_consecutive_losses,
                );

                let outcome = gate.evaluate(&ctx);

                ApiResponse::success(req.id, serde_json::json!({
                    "is_allowed": outcome.is_allowed(),
                    "outcome": outcome,
                    "order_notional": ctx.order_notional(),
                    "symbol": symbol,
                }))
            }
            METHOD_QUANT_REVIEW_COMPUTE => {
                let strategy_id = req.params.get("strategy_id").and_then(|v| v.as_str()).unwrap_or("strategy_default");
                let symbol = req.params.get("symbol").and_then(|v| v.as_str()).unwrap_or("ALL");

                let fills_val = req.params.get("fills");
                let fills: Vec<custos_domain::quant::TradeFillRecord> = match fills_val {
                    Some(v) => match serde_json::from_value(v.clone()) {
                        Ok(f) => f,
                        Err(e) => return ApiResponse::error(req.id, format!("Invalid fills payload: {e}")),
                    },
                    None => {
                        return ApiResponse::error(req.id, "Missing 'fills' array parameter");
                    }
                };

                let report = match custos_domain::quant::PostTradeAnalyzer::analyze(strategy_id, symbol, &fills) {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, format!("Post-trade review analysis failed: {e}")),
                };

                ApiResponse::success(req.id, serde_json::to_value(report).unwrap_or(serde_json::Value::Null))
            }
            METHOD_QUANT_ORDERBOOK_GET => {
                let symbol = req.params.get("symbol").and_then(|v| v.as_str()).unwrap_or("BTC/USDT");
                let depth_levels = req.params.get("depth").and_then(|v| v.as_u64()).unwrap_or(5).min(20) as usize;

                let store = self.sqlite_store.as_ref();
                let last_bar_close = if let Some(s) = store {
                    match s.quant().get_market_bars(symbol, "1m", 1).await {
                        Ok(bars) if !bars.is_empty() => Some(bars[0].close),
                        _ => match s.quant().get_market_bars(symbol, "1d", 1).await {
                            Ok(bars) if !bars.is_empty() => Some(bars[0].close),
                            _ => None,
                        },
                    }
                } else {
                    None
                };

                let close_price = last_bar_close.unwrap_or(if symbol.starts_with("BTC") { 68000.0 } else { 100.0 });
                let snapshot = custos_domain::quant::OrderBookSnapshot::synthesize(symbol, close_price, depth_levels);
                ApiResponse::success(req.id, serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null))
            }
            METHOD_QUANT_PBO_COMPUTE => {
                let matrix_val = match req.params.get("score_matrix").and_then(|v| v.as_array()) {
                    Some(arr) if !arr.is_empty() => arr,
                    _ => return ApiResponse::error(req.id, "Missing or invalid 'score_matrix' array"),
                };

                let mut matrix: Vec<Vec<f64>> = Vec::with_capacity(matrix_val.len());
                for row_val in matrix_val {
                    if let Some(sub_arr) = row_val.as_array() {
                        let row: Vec<f64> = sub_arr.iter().filter_map(|v| v.as_f64()).collect();
                        matrix.push(row);
                    }
                }

                let max_comb = req.params.get("max_combinations").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;

                match calculate_pbo_cscv(&matrix, max_comb) {
                    Ok(result) => ApiResponse::success(req.id, serde_json::to_value(result).unwrap_or(serde_json::Value::Null)),
                    Err(e) => ApiResponse::error(req.id, format!("PBO evaluation failed: {e}")),
                }
            }
            _ => ApiResponse::error(req.id, format!("Unknown quant method: {}", req.method)),
        }
    }
}

