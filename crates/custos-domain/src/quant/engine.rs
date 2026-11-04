//! Native High-Performance Simulation & Backtest Engine
//!
//! Models discrete-time, zero-I/O bar processing, realistic slippage, transaction costs,
//! position accounting, and drawdown tracking for algorithmic strategies in Rust.

use serde::{Deserialize, Serialize};
use crate::error::DomainError;
use super::backtest::BacktestTearsheet;
use super::risk::ProtectionSpec;

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

        let fast_sma: f64 = fast_slice.iter().map(|b| b.close).sum::<f64>() / self.fast_period as f64;
        let slow_sma: f64 = slow_slice.iter().map(|b| b.close).sum::<f64>() / self.slow_period as f64;

        if fast_sma > slow_sma {
            SignalAction::TargetPercent(self.target_pct)
        } else {
            SignalAction::TargetPercent(0.0)
        }
    }
}

pub struct NativeBacktestEngine;

impl NativeBacktestEngine {
    pub fn run(
        strategy_id: &str,
        bars: &[OhlcvBar],
        config: &BacktestConfig,
        mut rule: Box<dyn StrategyRule>,
    ) -> Result<BacktestTearsheet, DomainError> {
        if bars.len() < 5 {
            return Err(DomainError::Validation(
                "Insufficient bar history for backtest simulation (minimum 5 bars required)".into(),
            ));
        }

        let mut cash: f64 = config.initial_capital;
        let mut position_qty: f64 = 0.0;
        let mut entry_price: f64 = 0.0;
        let mut peak_equity: f64 = config.initial_capital;
        let mut max_drawdown: f64 = 0.0;
        let mut daily_returns: Vec<f64> = Vec::with_capacity(bars.len());
        let mut prev_equity: f64 = config.initial_capital;
        let mut trades_count: usize = 0;
        let mut winning_trades: usize = 0;
        let mut gross_profit: f64 = 0.0;
        let mut gross_loss: f64 = 0.0;

        for i in 1..bars.len() {
            let current_bar = bars[i];
            let current_price = current_bar.close;

            // 1. Position Protection evaluation (Stop Loss / Take Profit / Trailing Stop)
            if let Some(ref prot) = config.protection {
                if position_qty.abs() > 1e-6 && entry_price > 0.0 {
                    let pnl_pct = (current_price - entry_price) / entry_price * position_qty.signum();
                    if (prot.stop_loss_pct > 0.0 && pnl_pct <= -prot.stop_loss_pct)
                        || (prot.take_profit_pct > 0.0 && pnl_pct >= prot.take_profit_pct)
                    {
                        let slippage_factor = 1.0 - config.slippage_bps * 1e-4 * position_qty.signum();
                        let exit_price = current_price * slippage_factor;
                        let proceeds = position_qty * exit_price;
                        let commission = proceeds.abs() * config.commission_rate;
                        cash += proceeds - commission;

                        let trade_pnl = (exit_price - entry_price) * position_qty;
                        if trade_pnl > 0.0 {
                            winning_trades += 1;
                            gross_profit += trade_pnl;
                        } else {
                            gross_loss += trade_pnl.abs();
                        }
                        trades_count += 1;
                        position_qty = 0.0;
                        entry_price = 0.0;
                    }
                }
            }

            // 2. Strategy Rule evaluation
            let action = rule.on_bar(i, &bars[..=i]);
            match action {
                SignalAction::TargetPercent(target_pct) => {
                    let total_equity = cash + position_qty * current_price;
                    let target_notional = total_equity * target_pct.clamp(-config.leverage, config.leverage);
                    let target_qty = target_notional / current_price;
                    let delta_qty = target_qty - position_qty;

                    if delta_qty.abs() * current_price > 5.0 {
                        let exec_price = if delta_qty > 0.0 {
                            current_price * (1.0 + config.slippage_bps * 1e-4)
                        } else {
                            current_price * (1.0 - config.slippage_bps * 1e-4)
                        };
                        let cost = delta_qty * exec_price;
                        let comm = cost.abs() * config.commission_rate;
                        cash -= cost + comm;

                        if position_qty.abs() < 1e-6 {
                            entry_price = exec_price;
                        } else if position_qty.signum() == delta_qty.signum() {
                            entry_price = (position_qty * entry_price + cost) / (position_qty + delta_qty);
                        } else {
                            trades_count += 1;
                        }
                        position_qty += delta_qty;
                    }
                }
                SignalAction::ClosePosition => {
                    if position_qty.abs() > 1e-6 {
                        let exec_price = current_price * (1.0 - config.slippage_bps * 1e-4 * position_qty.signum());
                        let proceeds = position_qty * exec_price;
                        let comm = proceeds.abs() * config.commission_rate;
                        cash += proceeds - comm;
                        trades_count += 1;
                        position_qty = 0.0;
                        entry_price = 0.0;
                    }
                }
                SignalAction::Hold => {}
            }

            // 3. Mark to Market Equity & Drawdown accounting
            let current_equity = cash + position_qty * current_price;
            peak_equity = peak_equity.max(current_equity);
            let dd = if peak_equity > 0.0 {
                (peak_equity - current_equity) / peak_equity
            } else {
                0.0
            };
            max_drawdown = max_drawdown.max(dd);

            let period_ret = if prev_equity > 0.0 {
                (current_equity - prev_equity) / prev_equity
            } else {
                0.0
            };
            daily_returns.push(period_ret);
            prev_equity = current_equity;
        }

        // 4. Performance calculation
        let total_return = (prev_equity - config.initial_capital) / config.initial_capital;
        let mean_ret = if !daily_returns.is_empty() {
            daily_returns.iter().sum::<f64>() / daily_returns.len() as f64
        } else {
            0.0
        };
        let var_ret = if daily_returns.len() > 1 {
            daily_returns.iter().map(|r| (r - mean_ret).powi(2)).sum::<f64>() / (daily_returns.len() - 1) as f64
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
            0.5
        };
        let profit_factor = if gross_loss > 1e-6 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            2.0
        } else {
            1.0
        };

        // 5. Deflated Sharpe Ratio (Multiple testing correction)
        let dsr = super::backtest::calculate_dsr(annualized_sharpe, 1, daily_returns.len(), 0.0, 3.0, 1.0);

        // 6. Institutional Risk/Return Metrics (Sortino, Downside Deviation, Omega, CVaR)
        let inst = super::metrics::compute_institutional_suite(&daily_returns, None, 252.0);

        let sheet = BacktestTearsheet::new(strategy_id, annualized_sharpe, max_drawdown, total_return)?
            .with_deflated_sharpe(dsr)
            .with_extended_metrics(
                std_ret * (252.0_f64).sqrt(),
                win_rate,
                profit_factor,
                trades_count as f64,
                Vec::new(),
            )
            .with_institutional_metrics(
                inst.sortino_ratio,
                inst.downside_deviation,
                inst.omega_ratio,
                inst.cvar_95,
            );

        Ok(sheet)
    }
}
