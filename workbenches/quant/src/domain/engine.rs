//! Native High-Performance Simulation & Backtest Engine
//!
//! Models discrete-time, zero-I/O bar processing with strict next-bar open execution
//! (zero same-bar lookahead), intrabar protection (SL/TP/Trailing Stop), realistic slippage,
//! transaction costs, FIFO trade fills, and equity curve accounting.

use super::backtest::{BacktestTearsheet, EquityPoint, MonthlyReturn};
use super::order::OrderDirection;
use super::review::{CloseReason, TradeFillRecord};
use super::risk::ProtectionSpec;
use chrono::{DateTime, Datelike};
use custos_contracts::DomainError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OhlcvBar {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl OhlcvBar {
    pub fn new(timestamp: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub initial_capital: f64,
    pub commission_rate: f64,
    pub slippage_bps: f64,
    pub leverage: f64,
    pub protection: Option<ProtectionSpec>,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_capital: 100_000.0,
            commission_rate: 0.0005, // 5 bps
            slippage_bps: 2.0,       // 2 bps
            leverage: 1.0,
            protection: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignalAction {
    Hold,
    TargetPercent(f64),
    ClosePosition,
}

pub trait StrategyRule: Send + Sync {
    fn on_bar(&mut self, bar_idx: usize, history: &[OhlcvBar]) -> SignalAction;
}

/// Canonical Dual Moving Average trend-following strategy rule
#[derive(Debug, Clone)]
pub struct DualMovingAverageRule {
    pub fast_period: usize,
    pub slow_period: usize,
    pub target_pct: f64,
}

impl DualMovingAverageRule {
    pub fn new(fast_period: usize, slow_period: usize, target_pct: f64) -> Self {
        Self {
            fast_period: fast_period.max(1),
            slow_period: slow_period.max(fast_period + 1),
            target_pct: target_pct.clamp(0.0, 1.0),
        }
    }
}

impl StrategyRule for DualMovingAverageRule {
    fn on_bar(&mut self, bar_idx: usize, history: &[OhlcvBar]) -> SignalAction {
        if bar_idx < self.slow_period {
            return SignalAction::Hold;
        }

        let fast_slice = &history[bar_idx + 1 - self.fast_period..=bar_idx];
        let slow_slice = &history[bar_idx + 1 - self.slow_period..=bar_idx];

        let fast_sma: f64 =
            fast_slice.iter().map(|b| b.close).sum::<f64>() / self.fast_period as f64;
        let slow_sma: f64 =
            slow_slice.iter().map(|b| b.close).sum::<f64>() / self.slow_period as f64;

        if fast_sma > slow_sma {
            SignalAction::TargetPercent(self.target_pct)
        } else {
            SignalAction::TargetPercent(0.0)
        }
    }
}

#[derive(Debug, Clone)]
pub struct BacktestSimulationOutput {
    pub tearsheet: BacktestTearsheet,
    pub fills: Vec<TradeFillRecord>,
    pub period_returns: Vec<f64>,
}

pub struct NativeBacktestEngine;

impl NativeBacktestEngine {
    pub fn run(
        strategy_id: &str,
        bars: &[OhlcvBar],
        config: &BacktestConfig,
        rule: Box<dyn StrategyRule>,
    ) -> Result<BacktestTearsheet, DomainError> {
        Ok(Self::run_detailed(strategy_id, "UNKNOWN", bars, config, rule, 1)?.tearsheet)
    }

    /// Runs a zero-lookahead simulation: signals evaluated at bar `i` close execute at
    /// bar `i + 1` open; intrabar SL/TP/Trailing Stop are checked against `low`/`high`.
    pub fn run_detailed(
        strategy_id: &str,
        symbol: &str,
        bars: &[OhlcvBar],
        config: &BacktestConfig,
        mut rule: Box<dyn StrategyRule>,
        trials_count: usize,
    ) -> Result<BacktestSimulationOutput, DomainError> {
        if bars.len() < 5 {
            return Err(DomainError::Validation(
                "Insufficient bar history for backtest simulation (minimum 5 bars required)".into(),
            ));
        }

        let mut cash = config.initial_capital;
        let mut position_qty = 0.0_f64;
        let mut entry_price = 0.0_f64;
        let mut entry_comm_total = 0.0_f64;
        let mut favorable_extreme = 0.0_f64;
        let mut trailing_armed = false;

        let mut peak_equity = config.initial_capital;
        let mut max_drawdown = 0.0_f64;
        let mut prev_equity = config.initial_capital;
        let mut period_returns = Vec::with_capacity(bars.len());
        let mut equity_curve = Vec::with_capacity(bars.len());
        let mut fills: Vec<TradeFillRecord> = Vec::new();
        let mut monthly_bounds: BTreeMap<(i32, u32), (f64, f64)> = BTreeMap::new();

        let mut trades_count = 0usize;
        let mut winning_trades = 0usize;
        let mut gross_profit = 0.0_f64;
        let mut gross_loss = 0.0_f64;
        let mut pending_action = SignalAction::Hold;

        let mut fill_seq = 0usize;

        for (i, bar) in bars.iter().enumerate() {
            // 1. Execute signal decided at previous bar's close on THIS bar's open (zero lookahead)
            if i > 0 {
                let open_px = if bar.open > 0.0 { bar.open } else { bar.close };
                match pending_action {
                    SignalAction::TargetPercent(target_pct) => {
                        let total_equity = cash + position_qty * open_px;
                        let target_notional =
                            total_equity * target_pct.clamp(-config.leverage, config.leverage);
                        let target_qty = target_notional / open_px;
                        let delta_qty = target_qty - position_qty;

                        if delta_qty.abs() * open_px > 5.0 {
                            let exec_price = if delta_qty > 0.0 {
                                open_px * (1.0 + config.slippage_bps * 1e-4)
                            } else {
                                open_px * (1.0 - config.slippage_bps * 1e-4)
                            };
                            let cost = delta_qty * exec_price;
                            let comm = cost.abs() * config.commission_rate;
                            cash -= cost + comm;
                            fill_seq += 1;

                            let closing_existing = position_qty.abs() > 1e-6
                                && position_qty.signum() != delta_qty.signum();
                            let direction = if delta_qty > 0.0 {
                                if closing_existing {
                                    OrderDirection::BuyToCover
                                } else {
                                    OrderDirection::Buy
                                }
                            } else if closing_existing {
                                OrderDirection::Sell
                            } else {
                                OrderDirection::SellShort
                            };

                            if closing_existing {
                                let closed_qty = position_qty.abs().min(delta_qty.abs());
                                let close_ratio = closed_qty / position_qty.abs();
                                let allocated_open_comm = entry_comm_total * close_ratio;
                                let net_pnl =
                                    (exec_price - entry_price) * closed_qty * position_qty.signum()
                                        - allocated_open_comm
                                        - comm;
                                trades_count += 1;
                                if net_pnl > 0.0 {
                                    winning_trades += 1;
                                    gross_profit += net_pnl;
                                } else {
                                    gross_loss += net_pnl.abs();
                                }
                                entry_comm_total =
                                    (entry_comm_total - allocated_open_comm).max(0.0);
                            }

                            let new_qty = position_qty + delta_qty;
                            if position_qty.abs() < 1e-6
                                || position_qty.signum() != new_qty.signum()
                            {
                                entry_price = if new_qty.abs() > 1e-6 {
                                    exec_price
                                } else {
                                    0.0
                                };
                                entry_comm_total = if new_qty.abs() > 1e-6 { comm } else { 0.0 };
                                favorable_extreme = entry_price;
                                trailing_armed = false;
                            } else if position_qty.signum() == delta_qty.signum() {
                                entry_price = (position_qty.abs() * entry_price
                                    + delta_qty.abs() * exec_price)
                                    / new_qty.abs();
                                entry_comm_total += comm;
                            }
                            position_qty = if new_qty.abs() < 1e-9 { 0.0 } else { new_qty };

                            fills.push(TradeFillRecord {
                                fill_id: format!("sim_fill_{strategy_id}_{fill_seq}"),
                                order_id: format!("sim_ord_{strategy_id}_{fill_seq}"),
                                symbol: symbol.to_string(),
                                direction,
                                price: exec_price,
                                quantity: delta_qty.abs(),
                                commission: comm,
                                timestamp: bar.timestamp,
                                exit_reason: closing_existing
                                    .then(|| CloseReason::SignalTrigger.as_str().to_string()),
                            });
                        }
                    }
                    SignalAction::ClosePosition => {
                        if position_qty.abs() > 1e-6 {
                            let exec_price = open_px
                                * (1.0 - config.slippage_bps * 1e-4 * position_qty.signum());
                            let proceeds = position_qty * exec_price;
                            let comm = proceeds.abs() * config.commission_rate;
                            cash += proceeds - comm;
                            let net_pnl =
                                (exec_price - entry_price) * position_qty - entry_comm_total - comm;
                            trades_count += 1;
                            if net_pnl > 0.0 {
                                winning_trades += 1;
                                gross_profit += net_pnl;
                            } else {
                                gross_loss += net_pnl.abs();
                            }
                            fill_seq += 1;
                            fills.push(TradeFillRecord {
                                fill_id: format!("sim_fill_{strategy_id}_{fill_seq}"),
                                order_id: format!("sim_ord_{strategy_id}_{fill_seq}"),
                                symbol: symbol.to_string(),
                                direction: if position_qty > 0.0 {
                                    OrderDirection::Sell
                                } else {
                                    OrderDirection::BuyToCover
                                },
                                price: exec_price,
                                quantity: position_qty.abs(),
                                commission: comm,
                                timestamp: bar.timestamp,
                                exit_reason: Some(CloseReason::SignalTrigger.as_str().to_string()),
                            });
                            position_qty = 0.0;
                            entry_price = 0.0;
                            entry_comm_total = 0.0;
                            trailing_armed = false;
                        }
                    }
                    SignalAction::Hold => {}
                }
            }

            // 2. Intrabar Protection evaluation (pessimistic order: Stop Loss -> Trailing Stop -> Take Profit)
            if let Some(ref prot) = config.protection {
                if position_qty.abs() > 1e-6 && entry_price > 0.0 {
                    let is_long = position_qty > 0.0;
                    if is_long {
                        favorable_extreme = favorable_extreme.max(bar.high);
                    } else {
                        favorable_extreme = if favorable_extreme <= 0.0 {
                            bar.low
                        } else {
                            favorable_extreme.min(bar.low)
                        };
                    }

                    let favorable_gain = if is_long {
                        (favorable_extreme - entry_price) / entry_price
                    } else {
                        (entry_price - favorable_extreme) / entry_price
                    };
                    if prot.trailing_stop_pct > 0.0
                        && favorable_gain >= prot.trailing_activation_pct
                    {
                        trailing_armed = true;
                    }

                    let sl_trigger_px = if prot.stop_loss_pct > 0.0 {
                        Some(if is_long {
                            entry_price * (1.0 - prot.stop_loss_pct)
                        } else {
                            entry_price * (1.0 + prot.stop_loss_pct)
                        })
                    } else {
                        None
                    };
                    let trail_trigger_px = if trailing_armed && prot.trailing_stop_pct > 0.0 {
                        Some(if is_long {
                            favorable_extreme * (1.0 - prot.trailing_stop_pct)
                        } else {
                            favorable_extreme * (1.0 + prot.trailing_stop_pct)
                        })
                    } else {
                        None
                    };
                    let tp_trigger_px = if prot.take_profit_pct > 0.0 {
                        Some(if is_long {
                            entry_price * (1.0 + prot.take_profit_pct)
                        } else {
                            entry_price * (1.0 - prot.take_profit_pct)
                        })
                    } else {
                        None
                    };

                    let triggered = sl_trigger_px
                        .filter(|&px| {
                            if is_long {
                                bar.low <= px
                            } else {
                                bar.high >= px
                            }
                        })
                        .map(|px| (px, CloseReason::ServerStopLoss))
                        .or_else(|| {
                            trail_trigger_px
                                .filter(|&px| {
                                    if is_long {
                                        bar.low <= px
                                    } else {
                                        bar.high >= px
                                    }
                                })
                                .map(|px| (px, CloseReason::TrailingStop))
                        })
                        .or_else(|| {
                            tp_trigger_px
                                .filter(|&px| {
                                    if is_long {
                                        bar.high >= px
                                    } else {
                                        bar.low <= px
                                    }
                                })
                                .map(|px| (px, CloseReason::ServerTakeProfit))
                        });

                    if let Some((raw_exit_px, close_reason)) = triggered {
                        let slippage_factor =
                            1.0 - config.slippage_bps * 1e-4 * position_qty.signum();
                        let exit_price = raw_exit_px * slippage_factor;
                        let proceeds = position_qty * exit_price;
                        let comm = proceeds.abs() * config.commission_rate;
                        cash += proceeds - comm;

                        let net_pnl =
                            (exit_price - entry_price) * position_qty - entry_comm_total - comm;
                        trades_count += 1;
                        if net_pnl > 0.0 {
                            winning_trades += 1;
                            gross_profit += net_pnl;
                        } else {
                            gross_loss += net_pnl.abs();
                        }
                        fill_seq += 1;
                        fills.push(TradeFillRecord {
                            fill_id: format!("sim_fill_{strategy_id}_{fill_seq}"),
                            order_id: format!("sim_ord_{strategy_id}_{fill_seq}"),
                            symbol: symbol.to_string(),
                            direction: if is_long {
                                OrderDirection::Sell
                            } else {
                                OrderDirection::BuyToCover
                            },
                            price: exit_price,
                            quantity: position_qty.abs(),
                            commission: comm,
                            timestamp: bar.timestamp,
                            exit_reason: Some(close_reason.as_str().to_string()),
                        });
                        position_qty = 0.0;
                        entry_price = 0.0;
                        entry_comm_total = 0.0;
                        trailing_armed = false;
                    }
                }
            }

            // 3. Evaluate strategy rule strictly on closed history `bars[..=i]` for NEXT bar execution
            pending_action = rule.on_bar(i, &bars[..=i]);

            // 4. Mark-to-market equity, drawdown & monthly tracking
            let current_equity = cash + position_qty * bar.close;
            peak_equity = peak_equity.max(current_equity);
            let dd = if peak_equity > 0.0 {
                (peak_equity - current_equity) / peak_equity
            } else {
                0.0
            };
            max_drawdown = max_drawdown.max(dd);

            if i > 0 {
                let period_ret = if prev_equity > 0.0 {
                    (current_equity - prev_equity) / prev_equity
                } else {
                    0.0
                };
                period_returns.push(period_ret);
            }
            prev_equity = current_equity;
            equity_curve.push(EquityPoint {
                timestamp: bar.timestamp,
                equity: current_equity,
                drawdown: dd,
            });

            if let Some(dt) = DateTime::from_timestamp(bar.timestamp, 0) {
                let ym = (dt.year(), dt.month());
                monthly_bounds
                    .entry(ym)
                    .and_modify(|(_, end_eq)| *end_eq = current_equity)
                    .or_insert((current_equity, current_equity));
            }
        }

        let total_return = (prev_equity - config.initial_capital) / config.initial_capital;
        let mean_ret = if !period_returns.is_empty() {
            period_returns.iter().sum::<f64>() / period_returns.len() as f64
        } else {
            0.0
        };
        let var_ret = if period_returns.len() > 1 {
            period_returns
                .iter()
                .map(|r| (r - mean_ret).powi(2))
                .sum::<f64>()
                / (period_returns.len() - 1) as f64
        } else {
            0.0
        };
        let std_ret = var_ret.sqrt();
        let annualized_sharpe = if std_ret > 1e-8 {
            (mean_ret / std_ret) * (252.0_f64).sqrt()
        } else {
            0.0
        };
        let win_rate = if trades_count > 0 {
            winning_trades as f64 / trades_count as f64
        } else {
            0.0
        };
        let profit_factor = if gross_loss > 1e-6 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            gross_profit
        } else {
            0.0
        };

        let monthly_returns: Vec<MonthlyReturn> = monthly_bounds
            .into_iter()
            .map(|((year, month), (start_eq, end_eq))| MonthlyReturn {
                year,
                month,
                return_pct: if start_eq > 1e-9 {
                    (end_eq - start_eq) / start_eq
                } else {
                    0.0
                },
            })
            .collect();

        let dsr = super::backtest::calculate_dsr(
            annualized_sharpe,
            trials_count.max(1),
            period_returns.len().max(2),
            0.0,
            3.0,
            1.0,
        );
        let inst = super::metrics::compute_institutional_suite(&period_returns, None, 252.0);

        let tearsheet = BacktestTearsheet::new(
            strategy_id,
            annualized_sharpe,
            max_drawdown.clamp(0.0, 1.0),
            total_return,
        )?
        .with_deflated_sharpe(dsr)
        .with_extended_metrics(
            std_ret * (252.0_f64).sqrt(),
            win_rate,
            profit_factor,
            trades_count as f64,
            monthly_returns,
        )
        .with_institutional_metrics(
            inst.sortino_ratio,
            inst.downside_deviation,
            inst.omega_ratio,
            inst.cvar_95,
        )
        .with_provenance(
            true,
            bars.first().map(|b| b.timestamp),
            bars.last().map(|b| b.timestamp),
            bars.len(),
            equity_curve,
        );

        Ok(BacktestSimulationOutput {
            tearsheet,
            fills,
            period_returns,
        })
    }
}
