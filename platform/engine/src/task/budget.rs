//! Budget Governor Service (Custos.md §16.3)
//!
//! Controls token and span quotas with atomic Reserve & Settle cycles,
//! enforcing the Anti-Gaming Invariant to prevent models from gaming execution limits.

use custos_domain::{new_id, Budget, DomainError, Headroom, ReservationToken};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct BudgetGovernor {
    budget: Arc<RwLock<Budget>>,
    active_reservations: Arc<RwLock<HashMap<String, ReservationToken>>>,
}

impl BudgetGovernor {
    pub fn new(budget: Budget) -> Self {
        Self {
            budget: Arc::new(RwLock::new(budget)),
            active_reservations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Reserves computational tokens/spans before dispatching model inference.
    pub fn reserve(
        &self,
        task_id: &str,
        spans: u32,
        tokens: u64,
    ) -> Result<ReservationToken, DomainError> {
        let mut budget_guard = self.budget.write().expect("lock poisoned");
        budget_guard.reserve(spans, tokens)?;

        let token = ReservationToken {
            token_id: new_id("rsrv"),
            task_id: task_id.to_string(),
            reserved_spans: spans,
            reserved_tokens: tokens,
        };

        let mut res_guard = self.active_reservations.write().expect("lock poisoned");
        res_guard.insert(token.token_id.clone(), token.clone());

        Ok(token)
    }

    /// Settles the actual consumption after receiving ExecutionReceipt.
    pub fn settle(
        &self,
        token_id: &str,
        actual_spans: u32,
        actual_tokens: u64,
    ) -> Result<(), DomainError> {
        let mut res_guard = self.active_reservations.write().expect("lock poisoned");
        let token = res_guard
            .remove(token_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "ReservationToken".into(),
                id: token_id.into(),
            })?;

        let mut budget_guard = self.budget.write().expect("lock poisoned");
        // Settle reservation against actuals
        budget_guard.settle(actual_spans, actual_tokens)?;

        // If reserved amount exceeded actual, refund remaining difference
        let span_diff = token.reserved_spans.saturating_sub(actual_spans);
        let token_diff = token.reserved_tokens.saturating_sub(actual_tokens);
        if span_diff > 0 || token_diff > 0 {
            budget_guard.refund(span_diff, token_diff)?;
        }

        Ok(())
    }

    /// Refunds the entire reservation upon action cancellation or dispatch failure.
    pub fn refund(&self, token_id: &str) -> Result<(), DomainError> {
        let mut res_guard = self.active_reservations.write().expect("lock poisoned");
        let token = res_guard
            .remove(token_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "ReservationToken".into(),
                id: token_id.into(),
            })?;

        let mut budget_guard = self.budget.write().expect("lock poisoned");
        budget_guard.refund(token.reserved_spans, token.reserved_tokens)?;

        Ok(())
    }

    /// Provides abstract headroom signal (Abundant, Constrained, Critical) without leaking raw counters (Anti-Gaming).
    pub fn headroom(&self) -> Headroom {
        let budget_guard = self.budget.read().expect("lock poisoned");
        budget_guard.headroom()
    }

    /// Returns a snapshot of the current budget metrics.
    pub fn snapshot(&self) -> Budget {
        let budget_guard = self.budget.read().expect("lock poisoned");
        budget_guard.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_governor_reserve_settle_refund_cycle() {
        let governor = BudgetGovernor::new(Budget::new(10, 1000, 100));

        assert_eq!(governor.headroom(), Headroom::Abundant);

        // Reserve 400 tokens
        let token = governor.reserve("task_1", 1, 400).expect("Reserve ok");
        assert_eq!(token.reserved_tokens, 400);

        // Settle actual 250 tokens (150 should be automatically refunded)
        governor.settle(&token.token_id, 1, 250).expect("Settle ok");

        let snap = governor.snapshot();
        assert_eq!(snap.settled_tokens, 250);
        assert_eq!(snap.reserved_tokens, 0);

        // Reserve another 600 tokens (total 250 + 600 = 850 / 1000 => 150 remaining = 15%)
        let token2 = governor.reserve("task_1", 1, 600).expect("Reserve ok");
        assert_eq!(governor.headroom(), Headroom::Constrained);

        // Refund token2 -> back to Abundant (250 / 1000)
        governor.refund(&token2.token_id).expect("Refund ok");
        let snap2 = governor.snapshot();
        assert_eq!(snap2.reserved_tokens, 0);
        assert_eq!(governor.headroom(), Headroom::Abundant);
    }
}
