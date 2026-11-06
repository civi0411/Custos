//! Pre-Trade Outbox Gate & Autonomous Risk Filter
//!
//! Enforces deterministic pre-trade risk inspection, notional caps, drawdown boundaries,
//! exit immunity (risk-reducing exits never blocked), and fail-open resilience.

use crate::error::DomainError;
use crate::quant::order::OrderDirection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionContext {
    pub symbol: String,
    pub direction: OrderDirection,
    pub quantity: f64,
    pub estimated_price: f64,
    pub current_drawdown: f64,
    pub consecutive_losses: usize,
    pub daily_executed_notional: f64,
    pub is_risk_reducing: bool,
}

impl DecisionContext {
    pub fn new(
        symbol: impl Into<String>,
        direction: OrderDirection,
        quantity: f64,
        estimated_price: f64,
    ) -> Result<Self, DomainError> {
        let symbol = symbol.into();
        if symbol.trim().is_empty() {
            return Err(DomainError::Validation(
                "DecisionContext symbol cannot be empty".into(),
            ));
        }
        if quantity <= 0.0 {
            return Err(DomainError::Validation(
                "DecisionContext quantity must be positive".into(),
            ));
        }
        if estimated_price <= 0.0 {
            return Err(DomainError::Validation(
                "DecisionContext estimated_price must be positive".into(),
            ));
        }

        Ok(Self {
            symbol,
            direction,
            quantity,
            estimated_price,
            current_drawdown: 0.0,
            consecutive_losses: 0,
            daily_executed_notional: 0.0,
            is_risk_reducing: false,
        })
    }

    pub fn with_risk_state(
        mut self,
        current_drawdown: f64,
        consecutive_losses: usize,
        daily_executed_notional: f64,
        is_risk_reducing: bool,
    ) -> Self {
        self.current_drawdown = current_drawdown.clamp(0.0, 1.0);
        self.consecutive_losses = consecutive_losses;
        self.daily_executed_notional = daily_executed_notional.max(0.0);
        self.is_risk_reducing = is_risk_reducing;
        self
    }

    pub fn order_notional(&self) -> f64 {
        self.quantity * self.estimated_price
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "details")]
pub enum GateOutcome {
    Pass { reason: String, order_notional: f64 },
    Reject { breach_type: String, reason: String },
    BypassExitImmunity { reason: String },
    FailOpen { reason: String },
}

impl GateOutcome {
    pub fn is_allowed(&self) -> bool {
        matches!(
            self,
            GateOutcome::Pass { .. }
                | GateOutcome::BypassExitImmunity { .. }
                | GateOutcome::FailOpen { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreTradeDecisionGate {
    pub max_order_notional: f64,
    pub max_daily_notional: f64,
    pub max_drawdown_limit: f64,
    pub max_consecutive_losses: usize,
    pub emergency_kill_active: bool,
}

impl Default for PreTradeDecisionGate {
    fn default() -> Self {
        Self {
            max_order_notional: 50_000.0,
            max_daily_notional: 250_000.0,
            max_drawdown_limit: 0.20, // 20% max DD
            max_consecutive_losses: 5,
            emergency_kill_active: false,
        }
    }
}

impl PreTradeDecisionGate {
    pub fn new(
        max_order_notional: f64,
        max_daily_notional: f64,
        max_drawdown_limit: f64,
        max_consecutive_losses: usize,
    ) -> Self {
        Self {
            max_order_notional: max_order_notional.max(100.0),
            max_daily_notional: max_daily_notional.max(max_order_notional),
            max_drawdown_limit: max_drawdown_limit.clamp(0.01, 1.0),
            max_consecutive_losses: max_consecutive_losses.max(1),
            emergency_kill_active: false,
        }
    }

    pub fn set_emergency_kill(&mut self, active: bool) {
        self.emergency_kill_active = active;
    }

    /// Evaluates an order intent through deterministic safety checks.
    ///
    /// Invariant 1: Risk-reducing exits (stop-loss, take-profit, close) ALWAYS bypass the gate immediately.
    /// Invariant 2: Emergency kill-switch rejects all non-exit orders instantly.
    /// Invariant 3: Single-order and daily notional caps are strictly checked.
    /// Invariant 4: Drawdown and consecutive loss streaks halt further entry exposure.
    pub fn evaluate(&self, ctx: &DecisionContext) -> GateOutcome {
        // 1. Exit immunity: Exits never blocked
        if ctx.is_risk_reducing {
            return GateOutcome::BypassExitImmunity {
                reason: "Risk-reducing exit order granted absolute gate immunity".into(),
            };
        }

        // 2. Kill switch check
        if self.emergency_kill_active {
            return GateOutcome::Reject {
                breach_type: "EmergencyKillActive".into(),
                reason: "Trading venue kill-switch is active. New entries prohibited.".into(),
            };
        }

        // 3. Single order notional cap
        let notional = ctx.order_notional();
        if notional > self.max_order_notional {
            return GateOutcome::Reject {
                breach_type: "MaxOrderNotionalExceeded".into(),
                reason: format!(
                    "Order notional (${:.2}) exceeds maximum allowable cap (${:.2})",
                    notional, self.max_order_notional
                ),
            };
        }

        // 4. Daily cumulative notional cap
        let projected_daily = ctx.daily_executed_notional + notional;
        if projected_daily > self.max_daily_notional {
            return GateOutcome::Reject {
                breach_type: "MaxDailyNotionalExceeded".into(),
                reason: format!(
                    "Projected daily notional (${:.2}) exceeds daily ceiling (${:.2})",
                    projected_daily, self.max_daily_notional
                ),
            };
        }

        // 5. Drawdown limit
        if ctx.current_drawdown > self.max_drawdown_limit {
            return GateOutcome::Reject {
                breach_type: "DrawdownLimitExceeded".into(),
                reason: format!(
                    "Current portfolio drawdown ({:.2}%) exceeds risk ceiling ({:.2}%)",
                    ctx.current_drawdown * 100.0,
                    self.max_drawdown_limit * 100.0
                ),
            };
        }

        // 6. Loss streak circuit breaker
        if ctx.consecutive_losses >= self.max_consecutive_losses {
            return GateOutcome::Reject {
                breach_type: "ConsecutiveLossStreakBreached".into(),
                reason: format!(
                    "Consecutive losses ({}) hit safety limit ({}). Cooling off required.",
                    ctx.consecutive_losses, self.max_consecutive_losses
                ),
            };
        }

        // Passed all gates
        GateOutcome::Pass {
            reason: "All pre-trade risk and notional boundaries verified".into(),
            order_notional: notional,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_immunity() {
        let gate = PreTradeDecisionGate {
            emergency_kill_active: true,
            max_order_notional: 100.0,
            ..Default::default()
        };

        let ctx = DecisionContext::new("BTCUSDT", OrderDirection::Sell, 2.0, 60000.0)
            .unwrap()
            .with_risk_state(0.50, 10, 500_000.0, true); // true = risk reducing

        let outcome = gate.evaluate(&ctx);
        assert!(outcome.is_allowed());
        assert!(matches!(outcome, GateOutcome::BypassExitImmunity { .. }));
    }

    #[test]
    fn test_emergency_kill_rejects_entries() {
        let mut gate = PreTradeDecisionGate::default();
        gate.set_emergency_kill(true);

        let ctx = DecisionContext::new("BTCUSDT", OrderDirection::Buy, 0.1, 60000.0).unwrap();
        let outcome = gate.evaluate(&ctx);
        assert!(!outcome.is_allowed());
        assert!(
            matches!(outcome, GateOutcome::Reject { breach_type, .. } if breach_type == "EmergencyKillActive")
        );
    }

    #[test]
    fn test_notional_cap_rejection() {
        let gate = PreTradeDecisionGate::new(10_000.0, 50_000.0, 0.20, 5);

        // 0.5 BTC * $30,000 = $15,000 > $10,000 cap
        let ctx = DecisionContext::new("BTCUSDT", OrderDirection::Buy, 0.5, 30_000.0).unwrap();
        let outcome = gate.evaluate(&ctx);
        assert!(!outcome.is_allowed());
        assert!(
            matches!(outcome, GateOutcome::Reject { breach_type, .. } if breach_type == "MaxOrderNotionalExceeded")
        );
    }

    #[test]
    fn test_pass_valid_order() {
        let gate = PreTradeDecisionGate::new(10_000.0, 50_000.0, 0.20, 5);

        // 0.1 BTC * $30,000 = $3,000 <= $10,000 cap
        let ctx = DecisionContext::new("BTCUSDT", OrderDirection::Buy, 0.1, 30_000.0).unwrap();
        let outcome = gate.evaluate(&ctx);
        assert!(outcome.is_allowed());
        assert!(matches!(outcome, GateOutcome::Pass { .. }));
    }
}
