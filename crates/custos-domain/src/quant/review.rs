//! Post-Trade Review & FIFO Cohort Decomposition
//!
//! Provides deterministic post-trade analysis, FIFO holding duration buckets,
//! exit reason cohorts, commission add-back decomposition, and 4-check diagnostics.

use crate::error::DomainError;
use crate::quant::order::OrderDirection;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeFillRecord {
    pub fill_id: String,
    pub order_id: String,
    pub symbol: String,
    pub direction: OrderDirection,
    pub price: f64,
    pub quantity: f64,
    pub commission: f64,
    pub timestamp: i64,
    pub exit_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoldingCohort {
    pub label: String,
    pub trade_count: usize,
    pub win_count: usize,
    pub win_rate: f64,
    pub gross_pnl: f64,
    pub net_pnl: f64,
    pub total_commission: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExitReasonCohort {
    pub reason: String,
    pub count: usize,
    pub net_pnl: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub code: String,       // "costs", "frequency", "exits", "position_risk"
    pub severity: String,   // "pass", "warning", "critical"
    pub summary: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostTradeReviewReport {
    pub strategy_id: String,
    pub symbol: String,
    pub total_trades: usize,
    pub gross_pnl: f64,
    pub total_commission: f64,
    pub net_pnl: f64,
    pub profit_factor: f64,
    pub win_loss_ratio: f64,
    pub holding_cohorts: Vec<HoldingCohort>,
    pub exit_cohorts: Vec<ExitReasonCohort>,
    pub diagnostics: Vec<DiagnosticCheck>,
}

#[derive(Debug, Clone)]
struct OpenPositionFill {
    price: f64,
    remaining_qty: f64,
    timestamp: i64,
    commission_per_unit: f64,
}

pub struct PostTradeAnalyzer;

impl PostTradeAnalyzer {
    /// Performs FIFO trade pairing, duration bucketing, and cohort decomposition.
    pub fn analyze(
        strategy_id: impl Into<String>,
        symbol: impl Into<String>,
        fills: &[TradeFillRecord],
    ) -> Result<PostTradeReviewReport, DomainError> {
        let strategy_id = strategy_id.into();
        let symbol = symbol.into();

        if fills.is_empty() {
            return Ok(PostTradeReviewReport {
                strategy_id,
                symbol,
                total_trades: 0,
                gross_pnl: 0.0,
                total_commission: 0.0,
                net_pnl: 0.0,
                profit_factor: 1.0,
                win_loss_ratio: 1.0,
                holding_cohorts: vec![],
                exit_cohorts: vec![],
                diagnostics: vec![],
            });
        }

        // Holding buckets: <5m (0..300), 5-30m (300..1800), 30-120m (1800..7200), >2h (7200+)
        let mut bucket_under_5m = (0usize, 0usize, 0.0f64, 0.0f64);   // count, win, gross, comm
        let mut bucket_5_to_30m = (0usize, 0usize, 0.0f64, 0.0f64);
        let mut bucket_30_to_120m = (0usize, 0usize, 0.0f64, 0.0f64);
        let mut bucket_over_2h = (0usize, 0usize, 0.0f64, 0.0f64);

        let mut exit_reason_map: std::collections::HashMap<String, (usize, f64)> = std::collections::HashMap::new();

        let mut open_longs: VecDeque<OpenPositionFill> = VecDeque::new();
        let mut open_shorts: VecDeque<OpenPositionFill> = VecDeque::new();

        let mut total_gross_pnl = 0.0;
        let mut total_commission = 0.0;
        let mut total_trades_paired = 0;
        let mut gross_wins = 0.0;
        let mut gross_losses = 0.0;
        let mut win_trades = 0;
        let mut loss_trades = 0;

        for fill in fills {
            total_commission += fill.commission;
            let comm_per_unit = if fill.quantity > 0.0 { fill.commission / fill.quantity } else { 0.0 };

            match fill.direction {
                OrderDirection::Buy => {
                    // Check if closing shorts first
                    let mut needed = fill.quantity;
                    while needed > 0.0 && !open_shorts.is_empty() {
                        let front = open_shorts.front_mut().unwrap();
                        let matched_qty = front.remaining_qty.min(needed);
                        let duration = (fill.timestamp - front.timestamp).max(0);
                        let gross_trade_pnl = (front.price - fill.price) * matched_qty; // Short profit when exit price < entry
                        let trade_comm = (front.commission_per_unit + comm_per_unit) * matched_qty;

                        total_gross_pnl += gross_trade_pnl;
                        total_trades_paired += 1;
                        if gross_trade_pnl > 0.0 {
                            gross_wins += gross_trade_pnl;
                            win_trades += 1;
                        } else {
                            gross_losses += gross_trade_pnl.abs();
                            loss_trades += 1;
                        }

                        // Duration cohort aggregation
                        let is_win = gross_trade_pnl > 0.0;
                        if duration < 300 {
                            bucket_under_5m.0 += 1;
                            if is_win { bucket_under_5m.1 += 1; }
                            bucket_under_5m.2 += gross_trade_pnl;
                            bucket_under_5m.3 += trade_comm;
                        } else if duration < 1800 {
                            bucket_5_to_30m.0 += 1;
                            if is_win { bucket_5_to_30m.1 += 1; }
                            bucket_5_to_30m.2 += gross_trade_pnl;
                            bucket_5_to_30m.3 += trade_comm;
                        } else if duration < 7200 {
                            bucket_30_to_120m.0 += 1;
                            if is_win { bucket_30_to_120m.1 += 1; }
                            bucket_30_to_120m.2 += gross_trade_pnl;
                            bucket_30_to_120m.3 += trade_comm;
                        } else {
                            bucket_over_2h.0 += 1;
                            if is_win { bucket_over_2h.1 += 1; }
                            bucket_over_2h.2 += gross_trade_pnl;
                            bucket_over_2h.3 += trade_comm;
                        }

                        // Exit reason aggregation
                        let reason = fill.exit_reason.clone().unwrap_or_else(|| "signal_exit".into());
                        let entry = exit_reason_map.entry(reason).or_insert((0, 0.0));
                        entry.0 += 1;
                        entry.1 += gross_trade_pnl - trade_comm;

                        front.remaining_qty -= matched_qty;
                        needed -= matched_qty;
                        if front.remaining_qty <= 1e-9 {
                            open_shorts.pop_front();
                        }
                    }

                    if needed > 0.0 {
                        open_longs.push_back(OpenPositionFill {
                            price: fill.price,
                            remaining_qty: needed,
                            timestamp: fill.timestamp,
                            commission_per_unit: comm_per_unit,
                        });
                    }
                }
                OrderDirection::Sell => {
                    // Check if closing longs first
                    let mut needed = fill.quantity;
                    while needed > 0.0 && !open_longs.is_empty() {
                        let front = open_longs.front_mut().unwrap();
                        let matched_qty = front.remaining_qty.min(needed);
                        let duration = (fill.timestamp - front.timestamp).max(0);
                        let gross_trade_pnl = (fill.price - front.price) * matched_qty;
                        let trade_comm = (front.commission_per_unit + comm_per_unit) * matched_qty;

                        total_gross_pnl += gross_trade_pnl;
                        total_trades_paired += 1;
                        if gross_trade_pnl > 0.0 {
                            gross_wins += gross_trade_pnl;
                            win_trades += 1;
                        } else {
                            gross_losses += gross_trade_pnl.abs();
                            loss_trades += 1;
                        }

                        let is_win = gross_trade_pnl > 0.0;
                        if duration < 300 {
                            bucket_under_5m.0 += 1;
                            if is_win { bucket_under_5m.1 += 1; }
                            bucket_under_5m.2 += gross_trade_pnl;
                            bucket_under_5m.3 += trade_comm;
                        } else if duration < 1800 {
                            bucket_5_to_30m.0 += 1;
                            if is_win { bucket_5_to_30m.1 += 1; }
                            bucket_5_to_30m.2 += gross_trade_pnl;
                            bucket_5_to_30m.3 += trade_comm;
                        } else if duration < 7200 {
                            bucket_30_to_120m.0 += 1;
                            if is_win { bucket_30_to_120m.1 += 1; }
                            bucket_30_to_120m.2 += gross_trade_pnl;
                            bucket_30_to_120m.3 += trade_comm;
                        } else {
                            bucket_over_2h.0 += 1;
                            if is_win { bucket_over_2h.1 += 1; }
                            bucket_over_2h.2 += gross_trade_pnl;
                            bucket_over_2h.3 += trade_comm;
                        }

                        let reason = fill.exit_reason.clone().unwrap_or_else(|| "signal_exit".into());
                        let entry = exit_reason_map.entry(reason).or_insert((0, 0.0));
                        entry.0 += 1;
                        entry.1 += gross_trade_pnl - trade_comm;

                        front.remaining_qty -= matched_qty;
                        needed -= matched_qty;
                        if front.remaining_qty <= 1e-9 {
                            open_longs.pop_front();
                        }
                    }

                    if needed > 0.0 {
                        open_shorts.push_back(OpenPositionFill {
                            price: fill.price,
                            remaining_qty: needed,
                            timestamp: fill.timestamp,
                            commission_per_unit: comm_per_unit,
                        });
                    }
                }
                _ => {}
            }
        }

        let net_pnl = total_gross_pnl - total_commission;
        let profit_factor = if gross_losses > 1e-6 { gross_wins / gross_losses } else { 1.0 };
        let avg_win = if win_trades > 0 { gross_wins / win_trades as f64 } else { 0.0 };
        let avg_loss = if loss_trades > 0 { gross_losses / loss_trades as f64 } else { 0.0 };
        let win_loss_ratio = if avg_loss > 1e-6 { avg_win / avg_loss } else { 1.0 };

        let make_cohort = |label: &str, data: (usize, usize, f64, f64)| -> HoldingCohort {
            let win_rate = if data.0 > 0 { data.1 as f64 / data.0 as f64 } else { 0.0 };
            HoldingCohort {
                label: label.into(),
                trade_count: data.0,
                win_count: data.1,
                win_rate,
                gross_pnl: data.2,
                net_pnl: data.2 - data.3,
                total_commission: data.3,
            }
        };

        let holding_cohorts = vec![
            make_cohort("<5m", bucket_under_5m),
            make_cohort("5-30m", bucket_5_to_30m),
            make_cohort("30-120m", bucket_30_to_120m),
            make_cohort(">2h", bucket_over_2h),
        ];

        let mut exit_cohorts: Vec<ExitReasonCohort> = exit_reason_map
            .into_iter()
            .map(|(reason, (count, pnl))| ExitReasonCohort { reason, count, net_pnl: pnl })
            .collect();
        exit_cohorts.sort_by_key(|a| std::cmp::Reverse(a.count));

        // 4 Diagnostic Checks
        let mut diagnostics = Vec::new();

        // 1. Costs check
        if total_commission > 0.25 * total_gross_pnl.abs() && total_commission > 10.0 {
            diagnostics.push(DiagnosticCheck {
                code: "costs".into(),
                severity: "warning".into(),
                summary: format!("Commissions (${:.2}) consumed {:.1}% of gross PnL", total_commission, (total_commission / total_gross_pnl.abs()) * 100.0),
                recommendation: "Consider switching to limit orders or widening take-profit targets to reduce turnover drag".into(),
            });
        } else {
            diagnostics.push(DiagnosticCheck {
                code: "costs".into(),
                severity: "pass".into(),
                summary: format!("Commissions (${:.2}) remain within sustainable boundaries", total_commission),
                recommendation: "Fee drag is nominal".into(),
            });
        }

        // 2. Frequency check
        if bucket_under_5m.0 > total_trades_paired / 2 && total_trades_paired > 5 {
            diagnostics.push(DiagnosticCheck {
                code: "frequency".into(),
                severity: "warning".into(),
                summary: format!("{:.1}% of trades held under 5 minutes with net PnL ${:.2}", (bucket_under_5m.0 as f64 / total_trades_paired as f64) * 100.0, bucket_under_5m.2 - bucket_under_5m.3),
                recommendation: "Ultra-short holding duration is susceptible to execution noise and slippage".into(),
            });
        } else {
            diagnostics.push(DiagnosticCheck {
                code: "frequency".into(),
                severity: "pass".into(),
                summary: "Trading cadence and holding horizons are balanced".into(),
                recommendation: "No over-trading detected".into(),
            });
        }

        // 3. Exits check
        let sl_cohort = exit_cohorts.iter().find(|e| e.reason == "stop_loss");
        if let Some(sl) = sl_cohort {
            if sl.count > total_trades_paired / 2 && sl.net_pnl < 0.0 {
                diagnostics.push(DiagnosticCheck {
                    code: "exits".into(),
                    severity: "warning".into(),
                    summary: format!("Stop-losses triggered in {}/{} trades (Net PnL ${:.2})", sl.count, total_trades_paired, sl.net_pnl),
                    recommendation: "Stop-loss distance may be too tight relative to prevailing market ATR".into(),
                });
            }
        }

        // 4. Position risk check
        if win_loss_ratio < 0.5 && win_trades > 0 {
            diagnostics.push(DiagnosticCheck {
                code: "position_risk".into(),
                severity: "critical".into(),
                summary: format!("Win/Loss ratio ({:.2}) indicates average loss is more than double average win", win_loss_ratio),
                recommendation: "Enforce strict risk-to-reward ratio >= 1.5:1 before signal trigger".into(),
            });
        } else {
            diagnostics.push(DiagnosticCheck {
                code: "position_risk".into(),
                severity: "pass".into(),
                summary: format!("Win/Loss ratio ({:.2}) and profit factor ({:.2}) remain positive", win_loss_ratio, profit_factor),
                recommendation: "Position risk sizing is healthy".into(),
            });
        }

        Ok(PostTradeReviewReport {
            strategy_id,
            symbol,
            total_trades: total_trades_paired,
            gross_pnl: total_gross_pnl,
            total_commission,
            net_pnl,
            profit_factor,
            win_loss_ratio,
            holding_cohorts,
            exit_cohorts,
            diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_trade_review_fifo_pairing() {
        let fills = vec![
            TradeFillRecord {
                fill_id: "f1".into(),
                order_id: "o1".into(),
                symbol: "BTCUSDT".into(),
                direction: OrderDirection::Buy,
                price: 50_000.0,
                quantity: 1.0,
                commission: 10.0,
                timestamp: 1000,
                exit_reason: None,
            },
            TradeFillRecord {
                fill_id: "f2".into(),
                order_id: "o2".into(),
                symbol: "BTCUSDT".into(),
                direction: OrderDirection::Sell,
                price: 52_000.0,
                quantity: 1.0,
                commission: 10.0,
                timestamp: 1200, // 200s duration -> <5m bucket
                exit_reason: Some("take_profit".into()),
            },
        ];

        let report = PostTradeAnalyzer::analyze("strat_1", "BTCUSDT", &fills).unwrap();
        assert_eq!(report.total_trades, 1);
        assert_eq!(report.gross_pnl, 2000.0);
        assert_eq!(report.total_commission, 20.0);
        assert_eq!(report.net_pnl, 1980.0);

        let under_5m = &report.holding_cohorts[0];
        assert_eq!(under_5m.label, "<5m");
        assert_eq!(under_5m.trade_count, 1);
        assert_eq!(under_5m.win_count, 1);
        assert_eq!(under_5m.net_pnl, 1980.0);

        assert_eq!(report.exit_cohorts.len(), 1);
        assert_eq!(report.exit_cohorts[0].reason, "take_profit");
        assert_eq!(report.exit_cohorts[0].net_pnl, 1980.0);
    }
}
