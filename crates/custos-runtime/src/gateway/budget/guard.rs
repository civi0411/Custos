use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCheck {
    pub allowed: bool,
    pub remaining_tokens: u64,
    pub remaining_cost_usd: f32,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BudgetLimits {
    pub max_tokens_per_task: u64,
    pub max_cost_usd_per_task: f32,
}

impl Default for BudgetLimits {
    fn default() -> Self {
        Self {
            max_tokens_per_task: 128_000,
            max_cost_usd_per_task: 2.0, // $2.00 max per task by default
        }
    }
}

pub struct BudgetGuard {
    limits: BudgetLimits,
}

impl Default for BudgetGuard {
    fn default() -> Self {
        Self::new(BudgetLimits::default())
    }
}

impl BudgetGuard {
    pub fn new(limits: BudgetLimits) -> Self {
        Self { limits }
    }

    pub fn check(
        &self,
        current_tokens_used: u64,
        current_cost_usd: f32,
        estimated_next_tokens: u64,
        estimated_next_cost_usd: f32,
    ) -> Result<BudgetCheck> {
        let projected_tokens = current_tokens_used.saturating_add(estimated_next_tokens);
        let projected_cost = current_cost_usd + estimated_next_cost_usd;

        if projected_tokens > self.limits.max_tokens_per_task {
            return Ok(BudgetCheck {
                allowed: false,
                remaining_tokens: self
                    .limits
                    .max_tokens_per_task
                    .saturating_sub(current_tokens_used),
                remaining_cost_usd: (self.limits.max_cost_usd_per_task - current_cost_usd).max(0.0),
                reason: Some(format!(
                    "Token budget exceeded: projected {} > limit {}",
                    projected_tokens, self.limits.max_tokens_per_task
                )),
            });
        }

        if projected_cost > self.limits.max_cost_usd_per_task {
            return Ok(BudgetCheck {
                allowed: false,
                remaining_tokens: self
                    .limits
                    .max_tokens_per_task
                    .saturating_sub(current_tokens_used),
                remaining_cost_usd: (self.limits.max_cost_usd_per_task - current_cost_usd).max(0.0),
                reason: Some(format!(
                    "Cost ceiling exceeded: projected ${:.4} > limit ${:.4}",
                    projected_cost, self.limits.max_cost_usd_per_task
                )),
            });
        }

        Ok(BudgetCheck {
            allowed: true,
            remaining_tokens: self
                .limits
                .max_tokens_per_task
                .saturating_sub(projected_tokens),
            remaining_cost_usd: (self.limits.max_cost_usd_per_task - projected_cost).max(0.0),
            reason: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_guard_within_limits() {
        let guard = BudgetGuard::default();
        let check = guard.check(10_000, 0.10, 5_000, 0.05).unwrap();
        assert!(check.allowed);
        assert!(check.remaining_tokens > 0);
    }

    #[test]
    fn test_budget_guard_exceeded() {
        let guard = BudgetGuard::new(BudgetLimits {
            max_tokens_per_task: 1_000,
            max_cost_usd_per_task: 0.10,
        });
        let check = guard.check(800, 0.08, 300, 0.05).unwrap();
        assert!(!check.allowed);
        assert!(check.reason.unwrap().contains("Token budget exceeded"));
    }
}
