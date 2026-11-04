//! Quantitative Risk Permit Configuration & Auditing
//!
//! Models risk limits, position sizing constraints, leverage boundaries,
//! Value at Risk (VaR 95%), and Deflated Sharpe Ratio hurdles for automated execution.

use crate::error::DomainError;
use crate::ids::new_id;
use crate::quant::backtest::BacktestTearsheet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum RiskBreach {
    DrawdownExceeded { actual: f64, limit: f64 },
    PositionSizeExceeded { actual_pct: f64, limit_pct: f64 },
    LeverageExceeded { actual: f64, limit: f64 },
    DsrBelowThreshold { actual: f64, threshold: f64 },
    StopLossInvalid { stop_loss: f64 },
}

impl std::fmt::Display for RiskBreach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskBreach::DrawdownExceeded { actual, limit } => {
                write!(
                    f,
                    "Max drawdown ({:.2}%) exceeded limit ({:.2}%)",
                    actual * 100.0,
                    limit * 100.0
                )
            }
            RiskBreach::PositionSizeExceeded { actual_pct, limit_pct } => {
                write!(
                    f,
                    "Position size ({:.2}%) exceeded limit ({:.2}%)",
                    actual_pct * 100.0,
                    limit_pct * 100.0
                )
            }
            RiskBreach::LeverageExceeded { actual, limit } => {
                write!(f, "Leverage ({actual:.2}x) exceeded limit ({limit:.2}x)")
            }
            RiskBreach::DsrBelowThreshold { actual, threshold } => {
                write!(f, "Deflated Sharpe Ratio ({actual:.3}) fell below threshold ({threshold:.3})")
            }
            RiskBreach::StopLossInvalid { stop_loss } => {
                write!(f, "Configured stop loss ({:.2}%) is invalid", stop_loss * 100.0)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskPermitConfig {
    pub id: String,
    pub permit_id: String,
    pub strategy_id: String,
    pub max_position_pct: f64,
    pub max_leverage: f64,
    pub stop_loss_pct: f64,
    pub var_95: f64,
    pub max_drawdown_limit: f64,
    pub dsr_threshold: f64,
    pub created_at: i64,
}

impl RiskPermitConfig {
    pub fn new(
        strategy_id: impl Into<String>,
        permit_id: impl Into<String>,
        max_position_pct: f64,
        max_leverage: f64,
        stop_loss_pct: f64,
    ) -> Result<Self, DomainError> {
        let strategy_id = strategy_id.into();
        let permit_id = permit_id.into();

        if strategy_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "RiskPermitConfig strategy_id cannot be empty".into(),
            ));
        }
        if max_position_pct <= 0.0 || max_position_pct > 1.0 {
            return Err(DomainError::Validation(
                "max_position_pct must be between 0.0 and 1.0".into(),
            ));
        }
        if max_leverage < 1.0 {
            return Err(DomainError::Validation(
                "max_leverage must be at least 1.0".into(),
            ));
        }
        if stop_loss_pct <= 0.0 || stop_loss_pct >= 1.0 {
            return Err(DomainError::Validation(
                "stop_loss_pct must be between 0.0 and 1.0".into(),
            ));
        }

        let now = chrono::Utc::now().timestamp();
        Ok(Self {
            id: new_id("risk_permit"),
            permit_id,
            strategy_id,
            max_position_pct,
            max_leverage,
            stop_loss_pct,
            var_95: 0.02,
            max_drawdown_limit: 0.20,
            dsr_threshold: 0.0,
            created_at: now,
        })
    }

    pub fn with_drawdown_and_dsr_limits(
        mut self,
        max_drawdown_limit: f64,
        dsr_threshold: f64,
    ) -> Self {
        self.max_drawdown_limit = max_drawdown_limit;
        self.dsr_threshold = dsr_threshold;
        self
    }

    pub fn audit_tearsheet(&self, tearsheet: &BacktestTearsheet) -> Result<(), RiskBreach> {
        if tearsheet.max_drawdown > self.max_drawdown_limit {
            return Err(RiskBreach::DrawdownExceeded {
                actual: tearsheet.max_drawdown,
                limit: self.max_drawdown_limit,
            });
        }
        if tearsheet.deflated_sharpe < self.dsr_threshold {
            return Err(RiskBreach::DsrBelowThreshold {
                actual: tearsheet.deflated_sharpe,
                threshold: self.dsr_threshold,
            });
        }
        Ok(())
    }

    pub fn validate_order_weight(&self, target_weight: f64) -> Result<(), RiskBreach> {
        if target_weight.abs() > self.max_position_pct {
            return Err(RiskBreach::PositionSizeExceeded {
                actual_pct: target_weight.abs(),
                limit_pct: self.max_position_pct,
            });
        }
        Ok(())
    }
}

/// Shared position-level risk protection semantics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectionSpec {
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub trailing_stop_pct: f64,
    pub trailing_activation_pct: f64,
    pub time_limit_seconds: i64,
}

impl Default for ProtectionSpec {
    fn default() -> Self {
        Self {
            stop_loss_pct: 0.05,        // 5% Stop loss
            take_profit_pct: 0.15,      // 15% Take profit
            trailing_stop_pct: 0.03,    // 3% Trailing stop
            trailing_activation_pct: 0.05, // 5% Profit needed to activate trailing stop
            time_limit_seconds: 86_400, // 24 hours
        }
    }
}

impl ProtectionSpec {
    pub fn new(
        stop_loss_pct: f64,
        take_profit_pct: f64,
        trailing_stop_pct: f64,
        trailing_activation_pct: f64,
    ) -> Self {
        Self {
            stop_loss_pct: stop_loss_pct.clamp(0.0, 1.0),
            take_profit_pct: take_profit_pct.clamp(0.0, 5.0),
            trailing_stop_pct: trailing_stop_pct.clamp(0.0, 1.0),
            trailing_activation_pct: trailing_activation_pct.clamp(0.0, 5.0),
            time_limit_seconds: 0,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.stop_loss_pct > 0.0
            || self.take_profit_pct > 0.0
            || self.trailing_stop_pct > 0.0
            || self.time_limit_seconds > 0
    }
}

/// Live tracking state for an open protected position
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectionState {
    pub symbol: String,
    pub entry_price: f64,
    pub highest_price: f64,
    pub lowest_price: f64,
    pub trailing_active: bool,
    pub spec: ProtectionSpec,
}

impl ProtectionState {
    pub fn open(symbol: impl Into<String>, entry_price: f64, spec: ProtectionSpec) -> Self {
        let price = entry_price.max(0.0);
        Self {
            symbol: symbol.into(),
            entry_price: price,
            highest_price: price,
            lowest_price: price,
            trailing_active: false,
            spec,
        }
    }

    pub fn update(&mut self, current_price: f64, is_long: bool) -> bool {
        if self.entry_price <= 0.0 {
            return false;
        }

        self.highest_price = self.highest_price.max(current_price);
        self.lowest_price = self.lowest_price.min(current_price);

        let pnl_pct = if is_long {
            (current_price - self.entry_price) / self.entry_price
        } else {
            (self.entry_price - current_price) / self.entry_price
        };

        // Check Hard Stop Loss
        if self.spec.stop_loss_pct > 0.0 && pnl_pct <= -self.spec.stop_loss_pct {
            return true;
        }

        // Check Take Profit
        if self.spec.take_profit_pct > 0.0 && pnl_pct >= self.spec.take_profit_pct {
            return true;
        }

        // Check Trailing Stop
        if self.spec.trailing_stop_pct > 0.0 {
            if !self.trailing_active && pnl_pct >= self.spec.trailing_activation_pct {
                self.trailing_active = true;
            }

            if self.trailing_active {
                if is_long {
                    let pullback = (self.highest_price - current_price) / self.highest_price;
                    if pullback >= self.spec.trailing_stop_pct {
                        return true;
                    }
                } else {
                    let pullback = (current_price - self.lowest_price) / self.lowest_price;
                    if pullback >= self.spec.trailing_stop_pct {
                        return true;
                    }
                }
            }
        }

        false
    }
}

