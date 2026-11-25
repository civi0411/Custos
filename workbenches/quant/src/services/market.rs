//! Application Service orchestrating Market Data ingestion, Factor Discovery (DSL + BH-FDR),
//! Pre-trade Risk Gates, Evidence Quality Assessment, Strategies, Positions, and Post-trade Reviews.

use super::dto::*;
use crate::{
    assess_evidence_quality, compute_rank_ic, detect_lookahead_bias, BenjaminiHochberg,
    DecisionContext, DirectionMode, EvidenceQualitySummary, FactorClassification, FactorExpr,
    MarketCategory, OhlcvBar, OrderDirection, PostTradeAnalyzer, PostTradeReviewReport,
    PreTradeDecisionGate, QuantPort, QuantPositionRecord, QuantStrategyRecord, TradeFillRecord,
};
use custos_contracts::{new_id, DomainError};

pub struct MarketService;

fn compute_factor_stats(f_vals: &[f64], f_fwd: &[f64], window: usize) -> (f64, f64, f64, f64) {
    let min_len = f_vals.len().min(f_fwd.len());
    if min_len < window || window < 3 {
        let ic = compute_rank_ic(&f_vals[..min_len], &f_fwd[..min_len]);
        return (ic, 0.0, 0.0, 1.0);
    }

    let mut rolling_ics = Vec::new();
    for i in 0..=(min_len - window) {
        rolling_ics.push(compute_rank_ic(
            &f_vals[i..i + window],
            &f_fwd[i..i + window],
        ));
    }
    if rolling_ics.is_empty() {
        return (0.0, 0.0, 0.0, 1.0);
    }

    let n = rolling_ics.len() as f64;
    let mean = rolling_ics.iter().sum::<f64>() / n;
    let var = if rolling_ics.len() > 1 {
        rolling_ics.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };
    let std = var.sqrt();
    let ir = if std > 1e-6 { mean / std } else { 0.0 };

    let p_val = if std > 1e-6 {
        let z = (mean / (std / n.sqrt())).abs();
        let t = 1.0 / (1.0 + 0.2316419 * z);
        let d = 0.3989422804014327 * (-z * z / 2.0).exp();
        let cdf = 1.0
            - d * t
                * (0.319381530
                    + t * (-0.356563782
                        + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
        (2.0 * (1.0 - cdf)).clamp(0.0001, 1.0)
    } else {
        1.0
    };
    (mean, std, ir, p_val)
}

impl MarketService {
    pub async fn discover_factors(
        req: DiscoverFactorsRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<serde_json::Value, DomainError> {
        let bars = quant_port
            .get_market_bars(&req.symbol, &req.timeframe, 500)
            .await
            .unwrap_or_default();

        if bars.len() < 20 {
            return Err(DomainError::Validation(format!(
                "Insufficient market bars for '{}' ({}): at least 20 historical bars are required to compute empirical factor IC and FDR. Ingest historical bars first via v1.quant.market_bars.ingest.",
                req.symbol, req.timeframe
            )));
        }

        let n = bars.len();
        let mut forward_returns = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            forward_returns.push((bars[i + 1].close - bars[i].close) / bars[i].close);
        }

        let specs: Vec<(String, String, String, String)> = if let Some(custom) = req.formulas {
            custom
                .into_iter()
                .enumerate()
                .map(|(idx, formula)| {
                    (
                        format!("sig_custom_{}", idx + 1),
                        format!("Custom Factor #{}", idx + 1),
                        "User/Agent-authored factor expression".into(),
                        formula,
                    )
                })
                .collect()
        } else {
            vec![
                (
                    "sig_momentum_5d".into(),
                    "5-Bar Rolling Momentum".into(),
                    "Short-term trend persistence".into(),
                    "close / delay(close, 5) - 1.0".into(),
                ),
                (
                    "sig_volatility_10d".into(),
                    "10-Bar Normalized Volatility".into(),
                    "Low volatility risk premium".into(),
                    "ts_std(close, 10) / ts_mean(close, 10)".into(),
                ),
                (
                    "sig_mean_rev_z15".into(),
                    "15-Bar Mean-Reversion Z-Score".into(),
                    "Short-term price overextension mean reversion".into(),
                    "zscore(close, 15)".into(),
                ),
                (
                    "sig_wilder_rsi_14".into(),
                    "Wilder RSI(14) Contrarian".into(),
                    "Wilder-smoothed momentum exhaustion signal".into(),
                    "rsi(14)".into(),
                ),
                (
                    "sig_vol_momentum_10".into(),
                    "10-Bar Volume-Confirmed Momentum".into(),
                    "Breakout persistence amplified by relative volume".into(),
                    "volume_momentum(10)".into(),
                ),
            ]
        };

        let mut raw_stats = Vec::new();
        for (id, name, hypothesis, formula) in specs {
            let expr = FactorExpr::parse(&formula)?;
            let series = expr.eval_series(&bars);
            let mut f_vals = Vec::with_capacity(series.len());
            let mut f_fwd = Vec::with_capacity(series.len());
            for (bar_idx, val) in series {
                if bar_idx < forward_returns.len() {
                    f_vals.push(val);
                    f_fwd.push(forward_returns[bar_idx]);
                }
            }
            let stats = compute_factor_stats(&f_vals, &f_fwd, 10);
            raw_stats.push((id, name, hypothesis, formula, stats));
        }

        let p_values: Vec<f64> = raw_stats.iter().map(|s| s.4 .3).collect();
        let q_values = BenjaminiHochberg::compute_q_values(&p_values);

        let mut candidates = Vec::with_capacity(raw_stats.len());
        for (idx, (id, name, hypothesis, formula, (ic_mean, ic_std, ir, _))) in
            raw_stats.into_iter().enumerate()
        {
            let q_val = q_values.get(idx).copied().unwrap_or(1.0);
            let class = if q_val <= 0.05 && ir.abs() > 0.3 {
                FactorClassification::Alive
            } else if q_val <= 0.15 {
                FactorClassification::Candidate
            } else if ir.abs() > 0.3 {
                FactorClassification::Decaying
            } else {
                FactorClassification::Dead
            };
            let status = if q_val <= 0.05 {
                "verified"
            } else {
                "computed"
            };
            candidates.push(serde_json::json!({
                "id": id,
                "name": name,
                "hypothesis": hypothesis,
                "formula": formula,
                "ic_mean": (ic_mean * 1000.0).round() / 1000.0,
                "ic_std": (ic_std * 1000.0).round() / 1000.0,
                "ir": (ir * 100.0).round() / 100.0,
                "q_value": (q_val * 1000.0).round() / 1000.0,
                "classification": class,
                "lookahead_clean": detect_lookahead_bias(&formula).is_ok(),
                "status": status
            }));
        }

        let passed_fdr = q_values.iter().filter(|&&q| q <= 0.05).count();
        Ok(serde_json::json!({
            "symbol": req.symbol,
            "timeframe": req.timeframe,
            "total_evaluated": candidates.len(),
            "passed_fdr": passed_fdr,
            "candidates": candidates,
        }))
    }

    pub async fn ingest_market_bars(
        req: IngestMarketBarsRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<usize, DomainError> {
        if req.symbol.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'symbol' parameter".into(),
            ));
        }
        let market = MarketCategory::parse_explicit(&req.market)?;
        let b_vec: Vec<OhlcvBar> = req.bars.into_iter().map(Into::into).collect();
        let count = b_vec.len();
        quant_port
            .save_market_bars(&req.symbol, &req.timeframe, &b_vec)
            .await?;
        quant_port
            .save_market_bar_meta(&req.symbol, &req.timeframe, market.as_str(), &req.venue)
            .await?;
        Ok(count)
    }

    pub async fn get_market_bars(
        req: GetMarketBarsRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<Vec<OhlcvBar>, DomainError> {
        if req.symbol.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'symbol' parameter".into(),
            ));
        }
        quant_port
            .get_market_bars(&req.symbol, &req.timeframe, req.limit)
            .await
    }

    pub async fn save_strategy(
        req: SaveStrategyRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<QuantStrategyRecord, DomainError> {
        let market = MarketCategory::parse_explicit(&req.market)?;
        let direction_mode = DirectionMode::from_str_loose(&req.direction_mode);
        let record = QuantStrategyRecord::new(
            req.id,
            req.name,
            market,
            req.venue,
            req.symbol,
            req.timeframe,
            req.formula,
            direction_mode,
        )?;
        quant_port.save_strategy(&record).await?;
        Ok(record)
    }

    pub async fn list_strategies(
        quant_port: &dyn QuantPort,
    ) -> Result<Vec<QuantStrategyRecord>, DomainError> {
        quant_port.list_strategies().await
    }

    pub async fn list_positions(
        req: ListPositionsRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<Vec<QuantPositionRecord>, DomainError> {
        quant_port.list_positions(req.strategy_id.as_deref()).await
    }

    pub async fn list_fills(
        req: ListFillsRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<Vec<TradeFillRecord>, DomainError> {
        quant_port
            .list_fills(req.strategy_id.as_deref(), req.symbol.as_deref(), req.limit)
            .await
    }

    pub async fn assess_evidence(
        req: AssessEvidenceRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<EvidenceQualitySummary, DomainError> {
        let market = MarketCategory::parse_explicit(&req.market)?;
        let now = chrono::Utc::now().timestamp();
        let summary = assess_evidence_quality(&req.observations, &req.required_metrics, now);
        let snap_id = new_id("qev");
        quant_port
            .save_evidence_snapshot(
                &snap_id,
                &req.task_id,
                &req.symbol,
                market.as_str(),
                &summary,
            )
            .await?;
        Ok(summary)
    }

    pub async fn evaluate_gate_with_store(
        mut req: EvaluateGateRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<serde_json::Value, DomainError> {
        if req.daily_executed_notional <= 0.0 {
            let now = chrono::Utc::now().timestamp();
            let day_start = now - (now % 86_400);
            if let Ok(actual) = quant_port.daily_executed_notional(day_start).await {
                req.daily_executed_notional = actual;
            }
        }
        Self::evaluate_gate(req)
    }

    pub fn evaluate_gate(req: EvaluateGateRequest) -> Result<serde_json::Value, DomainError> {
        if req.symbol.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'symbol' parameter".into(),
            ));
        }
        if req.quantity <= 0.0 || req.estimated_price <= 0.0 {
            return Err(DomainError::Validation(
                "quantity and estimated_price must be positive numbers".into(),
            ));
        }
        let direction = match req.direction.to_lowercase().as_str() {
            "buy" => OrderDirection::Buy,
            "sell" => OrderDirection::Sell,
            "sell_short" => OrderDirection::SellShort,
            "buy_to_cover" => OrderDirection::BuyToCover,
            other => {
                return Err(DomainError::Validation(format!(
                    "Invalid direction: {other}"
                )))
            }
        };

        let ctx = DecisionContext::new(&req.symbol, direction, req.quantity, req.estimated_price)
            .map_err(|e| DomainError::Validation(format!("Invalid decision context: {e}")))?
            .with_risk_state(
                req.current_drawdown,
                req.consecutive_losses,
                req.daily_executed_notional,
                req.is_risk_reducing,
            );

        let gate = PreTradeDecisionGate::new(
            req.max_order_notional,
            req.max_daily_notional,
            req.max_drawdown_limit,
            req.max_consecutive_losses,
        );
        let outcome = gate.evaluate(&ctx);

        Ok(serde_json::json!({
            "is_allowed": outcome.is_allowed(),
            "outcome": outcome,
            "order_notional": ctx.order_notional(),
            "daily_executed_notional": req.daily_executed_notional,
            "symbol": req.symbol,
        }))
    }

    pub async fn compute_review_with_store(
        req: ComputeReviewRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<PostTradeReviewReport, DomainError> {
        let fills = if req.fills.is_empty() {
            let strat_filter = if req.strategy_id.trim().is_empty() {
                None
            } else {
                Some(req.strategy_id.as_str())
            };
            let sym_filter = if req.symbol == "ALL" {
                None
            } else {
                Some(req.symbol.as_str())
            };
            quant_port
                .list_fills(strat_filter, sym_filter, 2000)
                .await?
        } else {
            req.fills
        };
        PostTradeAnalyzer::analyze(&req.strategy_id, &req.symbol, &fills)
            .map_err(|e| DomainError::Validation(format!("Post-trade review analysis failed: {e}")))
    }

    pub fn compute_review(req: ComputeReviewRequest) -> Result<PostTradeReviewReport, DomainError> {
        PostTradeAnalyzer::analyze(&req.strategy_id, &req.symbol, &req.fills)
            .map_err(|e| DomainError::Validation(format!("Post-trade review analysis failed: {e}")))
    }
}
